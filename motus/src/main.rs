//! Motus kawaii — Ratatui
use jeux_core::theme;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph},
    Frame, Terminal,
};
use rand::seq::SliceRandom;
use std::{
    collections::HashMap,
    io::stdout,
    process::Command,
    time::{Duration, Instant},
};

const MAX_TRIES: usize = 6;
const LEVELS: [(usize, usize); 5] = [(5, 2), (6, 1), (7, 0), (8, 0), (9, 0)];

struct App {
    menu: bool,
    level: usize,
    secret: Vec<char>,
    secret_norm: Vec<char>,
    secret_orig: String,
    len: usize,
    grid: Vec<Vec<char>>,
    states: Vec<Vec<u8>>, // 0=miss 1=present 2=good
    current: Vec<char>,
    locked: Vec<bool>,
    cursor: usize,
    over: bool,
    won: bool,
    message: String,
    def: String,
    dark: bool,
    parts: Vec<(f32, f32, f32, f32, f32, Color)>, // x, y, vx, vy, life, col
    fw_t: f32,
    fw_top: f32,
    fw_w: f32,
    rng: rand::rngs::ThreadRng,
    dict: HashMap<usize, Vec<String>>,
    secrets: HashMap<usize, Vec<String>>,
}

fn detect_dark() -> bool {
    // COLORFGBG="15;0" → fond sombre ; défaut : sombre
    if let Ok(v) = std::env::var("COLORFGBG") {
        let parts: Vec<&str> = v.split(';').collect();
        if parts.len() >= 2 {
            if let Ok(bg) = parts[parts.len() - 1].parse::<u8>() {
                return bg == 0 || bg == 8;
            }
        }
    }
    true
}

impl App {
    fn new() -> Self {
        let mut dict: HashMap<usize, Vec<String>> = HashMap::new();
        let mut secrets: HashMap<usize, Vec<String>> = HashMap::new();
        // embarqué dans le binaire (include_str! à la compilation)
        let mots_embed = include_str!("../data/mots.txt");
        let secrets_embed = include_str!("../data/secrets.txt");
        for (s, m) in [(mots_embed, &mut dict), (secrets_embed, &mut secrets)] {
            for w in s.lines() {
                // les secrets sont stockés avec accents (forme d'identité) :
                // la clé de longueur et la comparaison se font sur la forme
                // normalisée (sans accents, saisie clavier US friendly)
                let w = w.trim();
                let norm = jeux_core::dico::normalise(w);
                if (5..=9).contains(&norm.chars().count()) {
                    m.entry(norm.chars().count()).or_default().push(w.to_string());
                }
            }
        }
        App {
            menu: true,
            level: 0,
            secret: Vec::new(),
            secret_norm: Vec::new(),
            secret_orig: String::new(),
            len: 5,
            grid: Vec::new(),
            states: Vec::new(),
            current: Vec::new(),
            locked: Vec::new(),
            cursor: 0,
            over: false,
            won: false,
            message: String::new(),
            def: String::new(),
            dark: detect_dark(),
            parts: Vec::new(),
            fw_t: 0.0,
            fw_top: 10.0,
            fw_w: 100.0,
            rng: rand::thread_rng(),
            dict,
            secrets,
        }
    }

    fn start(&mut self, level: usize) {
        self.level = level;
        self.len = LEVELS[level].0;
        let pool = self.secrets.get(&self.len).cloned().unwrap_or_default();
        let mut rng = rand::thread_rng();
        match pool.choose(&mut rng) {
            Some(w) => {
                self.secret_orig = w.clone();
                self.secret_norm = jeux_core::dico::normalise(w).chars().collect();
                self.secret = self.secret_norm.clone();
            }
            None => {
                self.message = "Dictionnaire introuvable (＞﹏＜".into();
                self.menu = true;
                return;
            }
        }
        self.grid.clear();
        self.states.clear();
        self.current = vec!['\0'; self.len];
        self.current[0] = self.secret[0];
        self.locked = vec![false; self.len];
        self.locked[0] = true;
        self.cursor = self.first_free();
        self.over = false;
        self.won = false;
        self.def.clear();
        self.message = format!("Mot de {} lettres · entre 1 et {} ♪", self.len, MAX_TRIES);
        self.menu = false;
    }

    fn first_free(&self) -> usize {
        (0..self.len).find(|&i| !self.locked[i]).unwrap_or(self.len)
    }
    fn next_free_after(&self, from: usize) -> usize {
        (from + 1..self.len).find(|&i| !self.locked[i]).unwrap_or(self.len)
    }
    fn prev_free_before(&self, from: usize) -> usize {
        // exclure la case courante (← doit RECULER), borner si sentinelle
        let hi = self.len.min(from) - 1;
        (0..=hi)
            .rev()
            .find(|&i| !self.locked[i])
            .unwrap_or(self.first_free())
    }

    fn type_rune(&mut self, r: char) {
        if self.over || self.cursor >= self.len || self.locked[self.cursor] {
            return;
        }
        self.current[self.cursor] = r;
        self.cursor = self.next_free_after(self.cursor);
        self.message = "Entrée pour valider ♪".into();
    }

    fn backspace(&mut self) {
        if self.cursor < self.len && !self.locked[self.cursor] && self.current[self.cursor] != '\0' {
            self.current[self.cursor] = '\0';
            return;
        }
        for c in (0..self.cursor).rev() {
            if !self.locked[c] && self.current[c] != '\0' {
                self.current[c] = '\0';
                self.cursor = c;
                return;
            }
        }
    }

    fn score(&self, guess: &[char]) -> (Vec<u8>, Vec<bool>) {
        let mut fb = vec![0u8; self.len];
        let mut locked = vec![false; self.len];
        let mut rem: Vec<char> = self.secret.clone();
        for i in 0..self.len {
            if guess[i] == self.secret[i] {
                fb[i] = 2;
                locked[i] = true;
                if let Some(p) = rem.iter().position(|&r| r == guess[i]) {
                    rem.remove(p);
                }
            }
        }
        for i in 0..self.len {
            if fb[i] == 2 {
                continue;
            }
            if let Some(p) = rem.iter().position(|&r| r == guess[i]) {
                fb[i] = 1;
                rem.remove(p);
            }
        }
        (fb, locked)
    }

    /// Feux d'artifice de victoire : gerbes aléatoires au-dessus de la grille.
    fn step_fireworks(&mut self, dt: f32) {
        use rand::Rng;
        self.fw_t -= dt;
        if self.fw_t <= 0.0 && self.fw_top > 4.0 {
            self.fw_t = self.rng.gen_range(0.25..0.6);
            let bx = self.rng.gen_range(8.0..(self.fw_w - 8.0));
            let by = self.rng.gen_range(2.0..(self.fw_top - 1.0));
            let col = match self.rng.gen_range(0..3) {
                0 => Color::Rgb(214, 72, 140),
                1 => Color::Rgb(123, 184, 255),
                _ => Color::Rgb(255, 200, 120),
            };
            for _ in 0..24 {
                let a = self.rng.gen_range(0.0..std::f32::consts::TAU);
                let sp = self.rng.gen_range(3.0..14.0);
                self.parts.push((
                    bx, by,
                    a.cos() * sp, a.sin() * sp,
                    self.rng.gen_range(0.5..1.1),
                    col,
                ));
            }
        }
        for p in self.parts.iter_mut() {
            p.0 += p.2 * dt;
            p.1 += p.3 * dt;
            p.3 += 2.0 * dt;
            p.4 -= dt;
        }
        self.parts.retain(|p| p.4 > 0.0);
    }

    fn submit(&mut self) {
        if self.over || self.cursor < self.len {
            return;
        }
        let word: String = self.current.iter().collect();
        if !self.dict.get(&self.len).map_or(false, |v| v.contains(&word)) {
            self.message = format!("{} n'est pas dans mon dictionnaire... (＞﹏＜", word);
            return;
        }
        let guess: Vec<char> = self.current.clone();
        let (fb, locked) = self.score(&guess);
        self.grid.push(guess.clone());
        self.states.push(fb);
        if word == self.secret.iter().collect::<String>() {
            self.over = true;
            self.won = true;
            self.message = format!(
                "GAGNÉ ! {} essai(s) d'avance ☆*:.｡.o(≧▽≦)o.｡.:*☆",
                MAX_TRIES - self.grid.len()
            );
            let orig = self.secret_orig.clone();
            self.fetch_def(&orig);
            return;
        }
        if self.grid.len() >= MAX_TRIES {
            self.over = true;
            self.message = format!(
                "Perdu ! Le mot était {} (´；ω；`)",
                self.secret_orig.to_uppercase()
            );
            let orig = self.secret_orig.clone();
            self.fetch_def(&orig);
            return;
        }
        self.locked = locked;
        self.current = vec!['\0'; self.len];
        for i in 0..self.len {
            if self.locked[i] {
                self.current[i] = self.secret[i];
            }
        }
        self.cursor = self.first_free();
        self.message = "Raté ! Une lettre en plus... ♪".into();
    }

    fn fetch_def(&mut self, word: &str) {
        // curl en subprocess : pas de dépendance TLS
        let url = format!(
            "https://fr.wiktionary.org/w/api.php?action=parse&prop=wikitext&page={}&format=json&formatversion=2&redirects=1",
            word
        );
        if let Ok(out) = Command::new("curl")
            .args(["-s", "--max-time", "8", "-A", "motus-kawaii/1.0", &url])
            .output()
        {
            let s = String::from_utf8_lossy(&out.stdout);
            // extraire "wikitext":"..." puis parser les lignes # 
            if let Some(p) = s.find("\"wikitext\":\"") {
                let rest = &s[p + 12..];
                let wikitext = rest
                    .strip_suffix("\"}") // fin du json
                    .unwrap_or(rest.split_to_quote().unwrap_or(rest));
                let wikitext = unescape_json(wikitext);
                let mut defs = Vec::new();
                let mut in_fr = false;
                for l in wikitext.lines() {
                    if l.starts_with("== {{langue|fr}} ==") {
                        in_fr = true;
                    } else if in_fr && l.starts_with("== {{") {
                        break;
                    } else if in_fr && l.starts_with("# ") {
                        defs.push(clean_wiki(&l[2..]));
                    }
                }
                self.def = defs.join("\n");
            }
        }
    }

    fn colors(&self) -> (Color, Color, Color, Color) {
        let t = if self.dark { theme::SOMBRE } else { theme::CLAIR };
        (
            Color::Rgb(t.good.0, t.good.1, t.good.2),
            Color::Rgb(t.present.0, t.present.1, t.present.2),
            Color::Rgb(t.absent.0, t.absent.1, t.absent.2),
            Color::Rgb(t.given.0, t.given.1, t.given.2),
        )
    }
}

trait SplitQuote {
    fn split_to_quote(&self) -> Option<&str>;
}
impl SplitQuote for str {
    fn split_to_quote(&self) -> Option<&str> {
        let mut esc = false;
        for (i, c) in self.char_indices() {
            if esc {
                esc = false;
                continue;
            }
            match c {
                '\\' => esc = true,
                '"' => return Some(&self[..i]),
                _ => {}
            }
        }
        None
    }
}

/// Coupe un texte par mots à `max` colonnes (pas de mot tronqué).
fn wrap_text(s: &str, max: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for w in s.split_whitespace() {
        let len = cur.chars().count() + 1 + w.chars().count();
        if !cur.is_empty() && len > max {
            out.push(std::mem::take(&mut cur));
        }
        if !cur.is_empty() {
            cur.push(' ');
        }
        cur.push_str(w);
        while cur.chars().count() > max {
            let cut: String = cur.chars().take(max).collect();
            out.push(cut);
            cur = cur.chars().skip(max).collect();
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn unescape_json(s: &str) -> String {
    let mut out = String::new();
    let mut it = s.chars();
    while let Some(c) = it.next() {
        if c == '\\' {
            match it.next() {
                Some('n') => out.push('\n'),
                Some('"') => out.push('"'),
                Some('\\') => out.push('\\'),
                Some('u') => {
                    let hex: String = it.by_ref().take(4).collect();
                    if let Ok(n) = u32::from_str_radix(&hex, 16) {
                        if let Some(ch) = char::from_u32(n) {
                            out.push(ch);
                        }
                    }
                }
                other => {
                    if let Some(o) = other {
                        out.push(o);
                    }
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn clean_wiki(s: &str) -> String {
    let mut s = s.replace("'''", "").replace("''", "");
    while let Some(a) = s.find("{{") {
        if let Some(b) = s[a..].find("}}") {
            s = format!("{}{}", &s[..a], &s[a + b + 2..]);
        } else {
            break;
        }
    }
    while let Some(a) = s.find("[[") {
        if let Some(b) = s[a..].find("]]") {
            let inner = &s[a + 2..a + b];
            let inner = inner.split('|').last().unwrap_or(inner);
            s = format!("{}{}{}", &s[..a], inner, &s[a + b + 2..]);
        } else {
            break;
        }
    }
    s
}

fn ui(f: &mut Frame, app: &App) {
    let area = f.area();
    let (good, pres, miss, given) = app.colors();
    let accent = if app.dark { Color::Rgb(255, 158, 196) } else { Color::Rgb(214, 72, 140) };
    let title_s = Style::default().fg(accent).add_modifier(Modifier::BOLD);
    let msg_s = Style::default().fg(if app.dark { Color::Rgb(187, 187, 187) } else { Color::Rgb(85, 85, 85) });
    let help_s = Style::default().fg(Color::DarkGray);

    if app.menu {
        let mut lines = vec![
            Line::from(Span::styled(" ♡  M O T U S  ♡ ", title_s)),
            Line::from(""),
            Line::from(Span::styled("Choisis ta longueur :", msg_s)),
            Line::from(""),
        ];
        for (i, (n, _hints)) in LEVELS.iter().enumerate() {
            let style = if i == app.level {
                Style::default().fg(accent).add_modifier(Modifier::BOLD)
            } else {
                msg_s
            };
            lines.push(Line::from(Span::styled(
                format!("  [{}] {} lettres · {} essais", i + 1, n, MAX_TRIES),
                style,
            )));
        }
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "↑↓ choisir · entrée jouer · 1-5 direct · q quitter",
            help_s,
        )));
        let vh = lines.len() as u16;
        let vy = area.y + (area.height.saturating_sub(vh)) / 2;
        f.render_widget(
            Paragraph::new(lines).alignment(Alignment::Center),
            ratatui::layout::Rect { x: area.x, y: vy, width: area.width, height: vh },
        );
        return;
    }

    // grille dessinée : colonnes de 6 (border chars autour de cellules de 5)
    let cw = 6usize; // 1 bordure + 5
    let gw = app.len * cw + 1;
    let gh = MAX_TRIES * 2 + 1;
    let col = (area.width as usize).saturating_sub(gw) / 2;
    // bloc complet (HUD 3 + grille + messages ~6) centré verticalement
    let voff = (area.height as usize).saturating_sub(gh + 5) / 2;
    let row = voff + 3;
    // fw_top/fw_w renseignés par run_app via f.area()

    // titre + HUD
    let hud = if app.over {
        vec![
            Line::from(Span::styled("♡  M O T U S  ♡", title_s)).alignment(Alignment::Center),
            Line::from(""),
            Line::from(Span::styled(app.message.clone(), msg_s)).alignment(Alignment::Center),
        ]
    } else {
        vec![
            Line::from(Span::styled("♡  M O T U S  ♡", title_s)).alignment(Alignment::Center),
            Line::from(""),
            Line::from(Span::styled(
                format!(
                    "mot de {} lettres · essai {}/{}",
                    app.len,
                    app.grid.len() + 1,
                    MAX_TRIES
                ),
                msg_s,
            ))
            .alignment(Alignment::Center),
        ]
    };
    // feux d'artifice (victoire) : couche au-dessus de la HUD/grille
    if app.won && !app.parts.is_empty() {
        let mut by_row: std::collections::HashMap<usize, Vec<(usize, char, Color)>> = std::collections::HashMap::new();
        for p in &app.parts {
            let row = (p.1.round().max(0.0)) as usize;
            let x = (p.0.round().max(0.0)) as usize;
            if row < voff.min(10) && x < area.width as usize {
                by_row.entry(row).or_default().push((x, char::from_u32(0x00B7).unwrap_or('·'), p.5));
            }
        }
        for (r, mut items) in by_row {
            let mut spans: Vec<Span> = Vec::new();
            items.sort_by_key(|(x, _, _)| *x);
            let mut cur = 0usize;
            for (x, ch, col) in items {
                if x > cur {
                    spans.push(Span::raw(" ".repeat(x - cur)));
                }
                spans.push(Span::styled(ch.to_string(), Style::default().fg(col)));
                cur = x + 1;
            }
            let y = r as u16;
            if y < area.height {
                f.render_widget(
                    Line::from(spans),
                    Rect { x: 0, y, width: area.width, height: 1 },
                );
            }
        }
    }
    let hud_area = Rect { x: 0, y: voff as u16, width: area.width, height: 3 };
    f.render_widget(Paragraph::new(hud), hud_area);

    // bordures de la grille
    let border_s = Style::default().fg(Color::Gray);
    let seg = "─".repeat(cw - 1);
    for r in 0..gh {
        let y = (row + r) as u16;
        if y >= area.height {
            break;
        }
        let mut line = String::new();
        if col > 0 {
            line.push_str(&" ".repeat(col));
        }
        if r % 2 == 0 {
            // ligne horizontale
            let left = if r == 0 { '┌' } else if r == gh - 1 { '└' } else { '├' };
            let mid = if r == 0 { '┬' } else if r == gh - 1 { '┴' } else { '┼' };
            let right = if r == 0 { '┐' } else if r == gh - 1 { '┘' } else { '┤' };
            line.push(left);
            for c in 0..app.len {
                if c > 0 {
                    line.push(mid);
                }
                line.push_str(&seg);
            }
            line.push(right);
        } else {
            // ligne de cellules : bords verticaux seulement
            line.push('│');
            for c in 0..app.len {
                line.push_str(&" ".repeat(cw - 1));
                line.push('│');
            }
        }
        f.render_widget(
            Line::from(Span::styled(line, border_s)),
            Rect { x: 0, y, width: area.width, height: 1 },
        );
    }

    // cellules (lignes impaires)
    let vline_y = row; // top border row index
    for gr in 0..MAX_TRIES {
        let r = 2 * gr + 1;
        let y = (row + r) as u16;
        if y >= area.height {
            break;
        }
        let mut spans: Vec<Span> = Vec::new();
        if col > 0 {
            spans.push(Span::raw(" ".repeat(col)));
        }
        spans.push(Span::styled("│", border_s));
        for c in 0..app.len {
            let mut ch = '\0';
            let mut st = Style::default().fg(miss);
            if gr < app.grid.len() {
                ch = app.grid[gr][c];
                st = match app.states[gr][c] {
                    2 => Style::default().fg(good).add_modifier(Modifier::BOLD),
                    1 => Style::default().fg(pres).add_modifier(Modifier::BOLD),
                    _ => Style::default().fg(miss),
                };
            } else if gr == app.grid.len() && !app.over {
                ch = app.current[c];
                let on_cursor = c == app.cursor;
                if ch == '\0' && on_cursor {
                    ch = '_';
                }
                st = if app.locked[c] {
                    Style::default().fg(given).add_modifier(Modifier::BOLD)
                } else if on_cursor {
                    Style::default().fg(accent).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(miss)
                };
                // la case active se voit par le caractère (crochets), pas que la couleur
                if on_cursor && ch != '\0' {
                    let letter = if ch == '_' { "_".to_string() } else { ch.to_string() };
                    ch = '\0'; // texte personnalisé ci-dessous
                    let txt = format!(" [{}] ", letter);
                    spans.push(Span::styled(txt, st));
                    spans.push(Span::styled("│", border_s));
                    continue;
                }
            }
            let txt = if ch == '\0' {
                "  ·  ".to_string()
            } else { format!("  {}  ", ch) };
            spans.push(Span::styled(txt, st));
            spans.push(Span::styled("│", border_s));
        }
        f.render_widget(Line::from(spans), Rect { x: 0, y, width: area.width, height: 1 });
        let _ = vline_y;
    }

    // messages bas
    let bottom = (row + gh) as u16 + 1;
    let mut lines = vec![
        Line::from(Span::styled(app.message.clone(), msg_s)).alignment(Alignment::Center),
        Line::from(Span::styled(
            if app.over {
                "[n] rejouer · esc menu · q quitter".to_string()
            } else {
                "←→ bouger · backspace · entrée · esc menu".to_string()
            },
            help_s,
        ))
        .alignment(Alignment::Center),
    ];
    if !app.def.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled("📖 Définition", msg_s)).alignment(Alignment::Center));
        // wrap à la largeur utile : lignes longues sinon coupées par le terminal
        let width = area.width as usize;
        let max = width.saturating_sub(8).max(30);
        let mut dl = 0;
        for d in app.def.lines() {
            for frag in wrap_text(d, max) {
                if dl >= 4 { break; }
                lines.push(Line::from(Span::styled(frag, msg_s)).alignment(Alignment::Center));
                dl += 1;
            }
            if dl >= 4 { break; }
        }
    }
    let bh = (area.height.saturating_sub(bottom)).min(6);
    if bh > 0 {
        f.render_widget(
            Paragraph::new(lines),
            Rect { x: 0, y: bottom, width: area.width, height: bh },
        );
    }
    let _ = gw;
}

fn main() -> std::io::Result<()> {
    let mut app = App::new();
    let mut term = Terminal::new(ratatui::backend::CrosstermBackend::new(stdout()))?;
    crossterm::terminal::enable_raw_mode()?;
    let mut out = stdout();
    crossterm::execute!(out, crossterm::terminal::EnterAlternateScreen, crossterm::event::EnableMouseCapture)?;
    term.clear()?;

    let res = run_app(&mut term, &mut app);

    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(
        &mut term.backend_mut(),
        crossterm::terminal::LeaveAlternateScreen,
        crossterm::event::DisableMouseCapture
    )?;
    term.show_cursor()?;
    res
}

fn run_app(
    term: &mut Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
    app: &mut App,
) -> std::io::Result<()> {
    loop {
        let area = term.size()?;
        app.fw_top = ((area.height as usize).saturating_sub(13 + 5) / 2) as f32; // ≈ voff
        app.fw_w = area.width as f32;
        term.draw(|f| ui(f, app))?;
        let poll_ms = if app.over && app.won { 50 } else { 100 };
        if event::poll(Duration::from_millis(poll_ms))? {
            if let Event::Key(k) = event::read()? {
                if k.kind != KeyEventKind::Press {
                    continue;
                }
                if k.modifiers.contains(KeyModifiers::CONTROL) && k.code == KeyCode::Char('c') {
                    return Ok(());
                }
                match k.code {
                    KeyCode::Char('q') if app.menu => return Ok(()),
                    KeyCode::Esc if !app.menu => app.menu = true,
                    _ => {}
                }
                if app.menu {
                    match k.code {
                        KeyCode::Char(c @ '1'..='5') => {
                            app.start(c as usize - '1' as usize)
                        }
                        KeyCode::Down | KeyCode::Char('j') | KeyCode::Right | KeyCode::Char('l') => {
                            app.level = (app.level + 1) % LEVELS.len()
                        }
                        KeyCode::Up | KeyCode::Char('k') | KeyCode::Left | KeyCode::Char('h') => {
                            app.level = (app.level + LEVELS.len() - 1) % LEVELS.len()
                        }
                        KeyCode::Enter => app.start(app.level),
                        _ => {}
                    }
                } else {
                    match k.code {
                        KeyCode::Char('q') if app.over => return Ok(()),
                        KeyCode::Char('n') if app.over => app.start(app.level),
                        KeyCode::Char('n') => app.type_rune('n'),
                        KeyCode::Left => app.cursor = app.prev_free_before(app.cursor),
                        KeyCode::Right => app.cursor = app.next_free_after(app.cursor),
                        KeyCode::Backspace => app.backspace(),
                        KeyCode::Enter => app.submit(),
                        KeyCode::Char(c) if c.is_ascii_lowercase() => app.type_rune(c),
                        _ => {}
                    }
                }
            }
        } else if app.over && app.won {
            // pas d'événement : animer les feux d'artifice
            app.step_fireworks(0.05);
        }
    }
}

// silencieux les imports non utilisés éventuels
#[allow(dead_code)]
fn _now() -> Instant { Instant::now() }
#[allow(dead_code)]
fn _layout_check(c: Constraint) -> Constraint { c }

#[cfg(test)]
mod tests {
    #[test]
    fn wrap_par_mots() {
        let s = "Mot très long : une définition qui dépasse largement les quatre-vingts colonnes d'un terminal standard moderne";
        let out = super::wrap_text(s, 40);
        assert!(out.iter().all(|l| l.chars().count() <= 40), "{:?}", out);
        assert!(out.len() >= 3);
        // pas de perte de contenu
        assert_eq!(out.join(" ").split_whitespace().count(), s.split_whitespace().count());
    }
}

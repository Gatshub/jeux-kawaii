//! Takuzu kawaii — Ratatui. Binairo : 0/1, pas de triple, équilibre, unicité.
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use jeux_core::theme;
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame, Terminal,
};
use rand::seq::SliceRandom;
use rand::Rng;
use std::{collections::HashSet, io::stdout, time::Duration};

type Board = Vec<Vec<i8>>; // -1 vide, 0/1

const CELL_W: usize = 5;

struct Level {
    name: &'static str,
    size: usize,
    hints: usize,
}

const LEVELS: [Level; 3] = [
    Level { name: "facile", size: 6, hints: 2 },
    Level { name: "moyen", size: 8, hints: 1 },
    Level { name: "difficile", size: 10, hints: 0 },
];

// ─── Moteur ──────────────────────────────────────────────────────────────────

fn no_triple(b: &Board, i: usize, j: usize) -> bool {
    let n = b.len();
    let v = b[i][j];
    if j >= 2 && b[i][j - 1] == v && b[i][j - 2] == v { return false; }
    if j + 2 < n && b[i][j + 1] == v && b[i][j + 2] == v { return false; }
    if j >= 1 && j + 1 < n && b[i][j - 1] == v && b[i][j + 1] == v { return false; }
    if i >= 2 && b[i - 1][j] == v && b[i - 2][j] == v { return false; }
    if i + 2 < n && b[i + 1][j] == v && b[i + 2][j] == v { return false; }
    if i >= 1 && i + 1 < n && b[i - 1][j] == v && b[i + 1][j] == v { return false; }
    true
}

fn row_counts(b: &Board, i: usize) -> (usize, usize) {
    b[i].iter().fold((0, 0), |(z, o), &v| match v {
        0 => (z + 1, o),
        1 => (z, o + 1),
        _ => (z, o),
    })
}

fn col_counts(b: &Board, j: usize) -> (usize, usize) {
    (0..b.len()).fold((0, 0), |(z, o), i| match b[i][j] {
        0 => (z + 1, o),
        1 => (z, o + 1),
        _ => (z, o),
    })
}

fn row_eq(a: &[i8], c: &[i8]) -> bool { a == c }

fn col_eq(b: &Board, j1: usize, j2: usize) -> bool {
    (0..b.len()).all(|i| b[i][j1] == b[i][j2])
}

/// Remplit la grille par backtracking aléatoire (génération).
fn fill_gen(b: &mut Board, pos: usize, rng: &mut impl Rng) -> bool {
    let n = b.len();
    if pos == n * n {
        return true;
    }
    let (i, j) = (pos / n, pos % n);
    let mut vals = [0i8, 1];
    vals.shuffle(rng);
    for &v in vals.iter() {
        b[i][j] = v;
        if !no_triple(b, i, j) {
            b[i][j] = -1;
            continue;
        }
        let (rz, ro) = row_counts(b, i);
        let (cz, co) = col_counts(b, j);
        if rz > n / 2 || ro > n / 2 || cz > n / 2 || co > n / 2 {
            b[i][j] = -1;
            continue;
        }
        if j == n - 1 && rz + ro == n {
            if (0..i).any(|k| row_eq(&b[k], &b[i])) {
                b[i][j] = -1;
                continue;
            }
        }
        if i == n - 1 && cz + co == n {
            if (0..j).any(|k| col_eq(b, k, j)) {
                b[i][j] = -1;
                continue;
            }
        }
        if fill_gen(b, pos + 1, rng) {
            return true;
        }
        b[i][j] = -1;
    }
    false
}

/// Déductions simples. Renvoie false si contradiction.
fn propagate(b: &mut Board) -> bool {
    let n = b.len();
    loop {
        let mut changed = false;
        // équilibre
        for i in 0..n {
            let (z, o) = row_counts(b, i);
            if z + o == n { continue; }
            if z == n / 2 && o < n / 2 {
                for j in 0..n { if b[i][j] == -1 { b[i][j] = 1; changed = true; } }
            } else if o == n / 2 && z < n / 2 {
                for j in 0..n { if b[i][j] == -1 { b[i][j] = 0; changed = true; } }
            }
        }
        for j in 0..n {
            let (z, o) = col_counts(b, j);
            if z + o == n { continue; }
            if z == n / 2 && o < n / 2 {
                for i in 0..n { if b[i][j] == -1 { b[i][j] = 1; changed = true; } }
            } else if o == n / 2 && z < n / 2 {
                for i in 0..n { if b[i][j] == -1 { b[i][j] = 0; changed = true; } }
            }
        }
        // adjacences
        for i in 0..n {
            for j in 0..n {
                let v = b[i][j];
                if v < 0 { continue; }
                if j + 1 < n && b[i][j + 1] == v {
                    if j >= 1 && b[i][j - 1] == -1 { b[i][j - 1] = 1 - v; changed = true; }
                    if j + 2 < n && b[i][j + 2] == -1 { b[i][j + 2] = 1 - v; changed = true; }
                }
                if i + 1 < n && b[i + 1][j] == v {
                    if i >= 1 && b[i - 1][j] == -1 { b[i - 1][j] = 1 - v; changed = true; }
                    if i + 2 < n && b[i + 2][j] == -1 { b[i + 2][j] = 1 - v; changed = true; }
                }
                if j >= 1 && j + 1 < n && b[i][j - 1] == v && b[i][j + 1] == -1 {
                    b[i][j + 1] = 1 - v; changed = true;
                }
                if j >= 1 && j + 1 < n && b[i][j + 1] == v && b[i][j - 1] == -1 {
                    b[i][j - 1] = 1 - v; changed = true;
                }
                if i >= 1 && i + 1 < n && b[i - 1][j] == v && b[i + 1][j] == -1 {
                    b[i + 1][j] = 1 - v; changed = true;
                }
                if i >= 1 && i + 1 < n && b[i + 1][j] == v && b[i - 1][j] == -1 {
                    b[i - 1][j] = 1 - v; changed = true;
                }
            }
        }
        // contradiction ?
        for i in 0..n {
            for j in 0..n {
                if b[i][j] >= 0 && !no_triple(b, i, j) { return false; }
            }
            let (z, o) = row_counts(b, i);
            if z > n / 2 || o > n / 2 { return false; }
        }
        for j in 0..n {
            let (z, o) = col_counts(b, j);
            if z > n / 2 || o > n / 2 { return false; }
        }
        if !changed { return true; }
    }
}

/// Nombre de solutions (plafonné à `limit`). Clone : propagate est destructif.
fn count_solutions(orig: &Board, limit: usize) -> usize {
    let mut b = orig.clone();
    if !propagate(&mut b) {
        return 0;
    }
    let n = b.len();
    for i in 0..n {
        for j in 0..n {
            if b[i][j] == -1 {
                let mut count = 0;
                for v in [0i8, 1] {
                    b[i][j] = v;
                    count += count_solutions(&b, limit - count);
                    if count >= limit {
                        b[i][j] = -1;
                        return count;
                    }
                }
                b[i][j] = -1;
                return count;
            }
        }
    }
    for i in 0..n {
        for k in i + 1..n {
            if row_eq(&b[i], &b[k]) || col_eq(&b, i, k) {
                return 0;
            }
        }
    }
    1
}

fn generate_puzzle(n: usize) -> (Board, Board) {
    let mut rng = rand::thread_rng();
    loop {
        let mut full: Board = vec![vec![-1; n]; n];
        if !fill_gen(&mut full, 0, &mut rng) {
            continue;
        }
        let mut p = full.clone();
        let mut order: Vec<usize> = (0..n * n).collect();
        order.shuffle(&mut rng);
        for &c in order.iter() {
            let (i, j) = (c / n, c % n);
            let v = p[i][j];
            p[i][j] = -1;
            if count_solutions(&p.clone(), 2) != 1 {
                p[i][j] = v;
            }
        }
        return (p, full);
    }
}

// ─── État des lignes/colonnes : 0 neutre, 1 correct, 2 erreur ────────────────

fn line_state(vals: &[i8]) -> u8 {
    let n = vals.len();
    let (mut z, mut o, mut emp) = (0, 0, 0);
    for &v in vals {
        match v {
            0 => z += 1,
            1 => o += 1,
            _ => emp += 1,
        }
    }
    for k in 0..n.saturating_sub(2) {
        if vals[k] >= 0 && vals[k] == vals[k + 1] && vals[k + 1] == vals[k + 2] {
            return 2;
        }
    }
    if emp > 0 {
        if z > n / 2 || o > n / 2 { return 2; }
        return 0;
    }
    if z != o { return 2; }
    1
}

// ─── App ─────────────────────────────────────────────────────────────────────

struct App {
    menu: bool,
    level_idx: usize,
    grid: Board,
    hint: Vec<Vec<bool>>,
    sol: Board,
    cursor: usize,
    won: bool,
    message: String,
    dark: bool,
}

impl App {
    fn new() -> Self {
        App {
            menu: true,
            level_idx: 0,
            grid: vec![],
            hint: vec![],
            sol: vec![],
            cursor: 0,
            won: false,
            message: String::new(),
            dark: theme::detecte_sombre(),
        }
    }

    fn start(&mut self, level_idx: usize) {
        let lv = &LEVELS[level_idx];
        let (p, s) = generate_puzzle(lv.size);
        let n = lv.size;
        let mut hint = vec![vec![false; n]; n];
        let mut grid = p;
        if lv.hints > 0 {
            let mut rng = rand::thread_rng();
            let mut perm: Vec<usize> = (0..n * n).collect();
            perm.shuffle(&mut rng);
            for &c in perm.iter().take(lv.hints) {
                let (i, j) = (c / n, c % n);
                hint[i][j] = true;
                grid[i][j] = s[i][j];
            }
        }
        self.level_idx = level_idx;
        self.grid = grid;
        self.hint = hint;
        self.sol = s;
        self.cursor = 0;
        self.won = false;
        self.message = "Alterne les 0 et les 1 (๑˃ᴗ˂)ﻭ".into();
        self.menu = false;
    }

    fn row_state(&self, i: usize) -> u8 { line_state(&self.grid[i]) }

    fn col_state(&self, j: usize) -> u8 {
        let vals: Vec<i8> = (0..self.grid.len()).map(|i| self.grid[i][j]).collect();
        line_state(&vals)
    }

    fn dup_rows(&self) -> HashSet<usize> {
        let n = self.grid.len();
        let mut bad = HashSet::new();
        for i in 0..n {
            if self.grid[i].iter().any(|&v| v < 0) { continue; }
            for k in i + 1..n {
                if self.grid[k].iter().any(|&v| v < 0) { continue; }
                if row_eq(&self.grid[i], &self.grid[k]) {
                    bad.insert(i); bad.insert(k);
                }
            }
        }
        bad
    }

    fn dup_cols(&self) -> HashSet<usize> {
        let n = self.grid.len();
        let mut bad = HashSet::new();
        for j in 0..n {
            if (0..n).any(|i| self.grid[i][j] < 0) { continue; }
            for k in j + 1..n {
                if (0..n).any(|i| self.grid[i][k] < 0) { continue; }
                if col_eq(&self.grid, j, k) {
                    bad.insert(j); bad.insert(k);
                }
            }
        }
        bad
    }

    fn check_win(&self) -> bool {
        let n = self.grid.len();
        (0..n).all(|i| self.row_state(i) == 1 && self.col_state(i) == 1)
            && self.dup_rows().is_empty()
            && self.dup_cols().is_empty()
    }

    fn after_placement(&mut self) {
        if self.check_win() {
            self.won = true;
            self.message = "GAGNÉ ! Trop kawaii !! ☆*:.｡.o(≧▽≦)o.｡.:*☆".into();
        }
    }
}

// ─── Vue ─────────────────────────────────────────────────────────────────────

fn state_glyph(state: u8) -> &'static str {
    match state { 1 => "✓", 2 => "✗", _ => "·" }
}

fn ui(f: &mut Frame, app: &App) {
    let area = f.area();
    let dark = app.dark;
    let t = if dark { theme::SOMBRE } else { theme::CLAIR };
    let accent = Color::Rgb(t.accent.0, t.accent.1, t.accent.2);
    let ok = Color::Rgb(t.good.0, t.good.1, t.good.2);
    let ko = Color::Rgb(t.ko.0, t.ko.1, t.ko.2);
    let muted = if dark { Color::Rgb(154, 140, 160) } else { Color::Rgb(138, 125, 143) };
    let soft = if dark { Color::Rgb(201, 187, 209) } else { Color::Rgb(110, 90, 117) };
    let title_s = Style::default().fg(accent).add_modifier(Modifier::BOLD);
    let menu_s = Style::default().fg(soft);
    let sel_s = Style::default().fg(accent).add_modifier(Modifier::BOLD);
    let msg_s = Style::default().fg(accent).add_modifier(Modifier::BOLD);
    let help_s = Style::default().fg(Color::DarkGray);
    let border_s = Style::default().fg(muted);

    if app.menu {
        let mut lines = vec![
            Line::from(Span::styled(" ♡  T A K U Z U  ♡ ", title_s)),
            Line::from(""),
            Line::from(Span::styled("Choisis ton niveau :", menu_s)),
            Line::from(""),
        ];
        for (i, lv) in LEVELS.iter().enumerate() {
            let hint_word = if lv.hints > 1 { "indices" } else { "indice" };
            let prefix = format!("  [{}] ", i + 1);
            let line = format!("{:<5} · {:<9} · {} {:<7}", format!("{}x{}", lv.size, lv.size), lv.name, lv.hints, hint_word);
            let style = if i == app.level_idx { sel_s } else { menu_s };
            lines.push(Line::from(Span::styled(prefix + &line, style)));
        }
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "↑↓ choisir · entrée jouer · 1/2/3 direct · q quitter",
            help_s,
        )));
        let vh = lines.len() as u16;
        let vy = area.y + (area.height.saturating_sub(vh)) / 2;
        f.render_widget(
            Paragraph::new(lines).alignment(ratatui::layout::Alignment::Center),
            ratatui::layout::Rect { x: area.x, y: vy, width: area.width, height: vh },
        );
        return;
    }

    let n = app.grid.len();
    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(Span::styled(" ♡  T A K U Z U  ♡ ", title_s)));
    lines.push(Line::from(Span::styled(format!("niveau : {}", LEVELS[app.level_idx].name), menu_s)));
    lines.push(Line::from(""));

    // palette 0/1
    let remain = |d: i8| -> usize {
        let cnt = app.grid.iter().flatten().filter(|&&v| v == d).count();
        n * n / 2 - cnt
    };
    let mut pal: Vec<Span> = Vec::new();
    for d in [0i8, 1] {
        let r = remain(d);
        let style = if r == 0 {
            Style::default().fg(muted)
        } else {
            Style::default().fg(accent).add_modifier(Modifier::BOLD)
        };
        pal.push(Span::styled(format!("  {} ×{}  ", d, r), style));
    }
    lines.push(Line::from(pal));
    lines.push(Line::from(""));

    let dr = app.dup_rows();
    let dc = app.dup_cols();

    // gouttière d'état (✓/✗/·)
    let gutter = |state: u8, dup: bool| {
        let s = if dup {
            Style::default().fg(ko).add_modifier(Modifier::BOLD)
        } else {
            match state {
                1 => Style::default().fg(ok).add_modifier(Modifier::BOLD),
                2 => Style::default().fg(ko).add_modifier(Modifier::BOLD),
                _ => Style::default().fg(muted),
            }
        };
        let pad = CELL_W / 2;
        Span::styled(format!("{}{}{}", " ".repeat(pad), state_glyph(state), " ".repeat(pad)), s)
    };

    let seg = "─".repeat(CELL_W);
    let hline = |left: char, mid: char, right: char, indent: usize| -> Line<'static> {
        let mut s = String::from(" ".repeat(indent));
        s.push(left);
        for c in 0..n {
            if c > 0 { s.push(mid); }
            s.push_str(&seg);
        }
        s.push(right);
        Line::from(Span::styled(s, border_s))
    };

    lines.push(hline('┌', '┬', '┐', CELL_W / 2));
    for i in 0..n {
        let mut spans: Vec<Span> = vec![gutter(app.row_state(i), dr.contains(&i))];
        for j in 0..n {
            let idx = i * n + j;
            let ch = match app.grid[i][j] { 0 => "○", 1 => "●", _ => "·" };
            let style = if idx == app.cursor && !app.won {
                Style::default().fg(accent).add_modifier(Modifier::BOLD)
            } else if app.hint[i][j] {
                Style::default().fg(muted).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(soft)
            };
            let pad = (CELL_W - 1) / 2;
            spans.push(Span::styled("│", border_s));
            spans.push(Span::styled(format!("{}{}{}", " ".repeat(pad), ch, " ".repeat(pad + 1)), style));
        }
        spans.push(Span::styled("│", border_s));
        lines.push(Line::from(spans));
        if i < n - 1 {
            lines.push(hline('├', '┼', '┤', CELL_W / 2));
        }
    }
    lines.push(hline('└', '┴', '┘', CELL_W / 2));
    let mut last: Vec<Span> = vec![gutter(0, false)];
    for j in 0..n {
        let mut st = app.col_state(j);
        if dc.contains(&j) { st = 2; }
        last.push(gutter(st, dc.contains(&j)));
    }
    lines.push(Line::from(last));
    lines.push(Line::from(""));

    lines.push(Line::from(Span::styled(app.message.clone(), msg_s)));
    lines.push(Line::from(""));
    if app.won {
        lines.push(Line::from(Span::styled("🐾 ♡ 🐾 ♡ 🐾", Style::default().fg(accent))));
        lines.push(Line::from(Span::styled("[n] rejouer · esc menu · q quitter", help_s)));
    } else {
        lines.push(Line::from(Span::styled(
            "↑↓←→ bouger · espace/0/1 placer · backspace effacer · n rejouer · esc menu",
            help_s,
        )));
    }
    let vh = lines.len() as u16;
    let vy = area.y + (area.height.saturating_sub(vh)) / 2;
    f.render_widget(
        Paragraph::new(lines).alignment(ratatui::layout::Alignment::Center),
        ratatui::layout::Rect { x: area.x, y: vy, width: area.width, height: vh },
    );
}

fn main() -> std::io::Result<()> {
    let mut app = App::new();
    let backend = ratatui::backend::CrosstermBackend::new(stdout());
    let mut term = Terminal::new(backend)?;
    crossterm::terminal::enable_raw_mode()?;
    crossterm::execute!(
        stdout(),
        crossterm::terminal::EnterAlternateScreen,
        crossterm::event::EnableMouseCapture
    )?;

    let res = run(&mut term, &mut app);

    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(
        term.backend_mut(),
        crossterm::terminal::LeaveAlternateScreen,
        crossterm::event::DisableMouseCapture
    )?;
    term.show_cursor()?;
    res
}

fn run(
    term: &mut Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
    app: &mut App,
) -> std::io::Result<()> {
    loop {
        term.draw(|f| ui(f, app))?;
        if event::poll(Duration::from_millis(150))? {
            if let Event::Key(k) = event::read()? {
                if k.kind != KeyEventKind::Press { continue; }
                if k.modifiers.contains(KeyModifiers::CONTROL) && k.code == KeyCode::Char('c') {
                    return Ok(());
                }
                if app.menu {
                    match k.code {
                        KeyCode::Char(c @ '1'..='3') => app.start(c as usize - '1' as usize),
                        KeyCode::Up | KeyCode::Char('k') | KeyCode::Left | KeyCode::Char('h') => {
                            if app.level_idx > 0 { app.level_idx -= 1; }
                        }
                        KeyCode::Down | KeyCode::Char('j') | KeyCode::Right | KeyCode::Char('l') => {
                            if app.level_idx < LEVELS.len() - 1 { app.level_idx += 1; }
                        }
                        KeyCode::Enter => app.start(app.level_idx),
                        KeyCode::Char('q') => return Ok(()),
                        _ => {}
                    }
                    continue;
                }
                let n = app.grid.len();
                match k.code {
                    KeyCode::Char('q') if app.won => return Ok(()),
                    KeyCode::Esc => { *app = App::new(); continue; }
                    KeyCode::Char('n') => {
                        let lv = app.level_idx;
                        app.start(lv);
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        if app.cursor / n > 0 { app.cursor -= n; }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if app.cursor / n < n - 1 { app.cursor += n; }
                    }
                    KeyCode::Left | KeyCode::Char('h') => {
                        if app.cursor % n > 0 { app.cursor -= 1; }
                    }
                    KeyCode::Right | KeyCode::Char('l') => {
                        if app.cursor % n < n - 1 { app.cursor += 1; }
                    }
                    KeyCode::Backspace | KeyCode::Delete | KeyCode::Char('x') => {
                        let (i, j) = (app.cursor / n, app.cursor % n);
                        if !app.hint[i][j] && app.grid[i][j] >= 0 {
                            app.grid[i][j] = -1;
                            app.message = "Case vidée~".into();
                        } else if app.hint[i][j] {
                            app.message = "C'est un indice, il reste ! (｡•̀ᴗ-)✧".into();
                        }
                    }
                    KeyCode::Char(' ') => {
                        let (i, j) = (app.cursor / n, app.cursor % n);
                        if app.hint[i][j] {
                            app.message = "Case d'indice, déjà remplie !".into();
                            continue;
                        }
                        app.grid[i][j] = (app.grid[i][j] + 2) % 3 - 1; // -1→0→1→-1
                        app.message = match app.grid[i][j] {
                            0 => "Un petit 0 par ici ♡".into(),
                            _ => "Un petit 1 par ici ♡".into(),
                        };
                        app.after_placement();
                    }
                    KeyCode::Char(c @ '0') | KeyCode::Char(c @ '1') => {
                        let (i, j) = (app.cursor / n, app.cursor % n);
                        if app.hint[i][j] {
                            app.message = "Case d'indice, déjà remplie !".into();
                            continue;
                        }
                        app.grid[i][j] = c as i8 - '0' as i8;
                        app.message = match app.grid[i][j] {
                            0 => "Un petit 0 par ici ♡".into(),
                            _ => "Un petit 1 par ici ♡".into(),
                        };
                        app.after_placement();
                    }
                    _ => {}
                }
            }
        }
    }
}

// tests du moteur
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generation_solution_unique() {
        for &n in &[6, 8] {
            let (p, s) = generate_puzzle(n);
            assert_eq!(count_solutions(&p, 2), 1, "taille {} : pas unique", n);
            assert!(p.iter().flatten().any(|&v| v == -1));
            for i in 0..n {
                assert_eq!(line_state(&s[i]), 1);
            }
        }
    }

    #[test]
    fn line_state_basique() {
        assert_eq!(line_state(&[0, 0, 0]), 2); // triple
        assert_eq!(line_state(&[0, 1, -1]), 0); // incomplet
        assert_eq!(line_state(&[0, 1, 0, 1]), 1); // complet valide
        assert_eq!(line_state(&[0, 0, 1, 1]), 1); // complet équilibré
        assert_eq!(line_state(&[0, 0, 1, -1]), 0); // z=2=n/2 : à la limite, pas dépassée
        assert_eq!(line_state(&[0, 0, 1, 1, 0, 0]), 2); // 4 zéros sur 6 → dépassé
    }
}

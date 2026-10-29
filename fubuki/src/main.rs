//! Fubuki kawaii — Ratatui. Grille 3×3, chiffres 1-9, sommes de lignes/colonnes.
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use jeux_core::theme::{self, Theme};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame, Terminal,
};
use rand::seq::SliceRandom;
use std::{
    io::stdout,
    time::{Duration, Instant},
};

const TICK: Duration = Duration::from_millis(400);

struct Level {
    name: &'static str,
    hints: usize,
}

const LEVELS: [Level; 3] = [
    Level { name: "facile", hints: 2 },
    Level { name: "moyen", hints: 1 },
    Level { name: "difficile", hints: 0 },
];

struct App {
    menu: bool,
    level_idx: usize,
    grid: [i8; 9],          // -1 = vide
    hint_cell: [bool; 9],
    used: [bool; 10],
    sol: [i8; 9],
    row_sum: [i32; 3],
    col_sum: [i32; 3],
    cursor: usize,
    won: bool,
    message: String,
    paws: usize,
    dark: bool,
    started: Instant,
    last_draw: Option<String>,
}

impl App {
    fn new() -> Self {
        App {
            menu: true,
            level_idx: 0,
            grid: [-1; 9],
            hint_cell: [false; 9],
            used: [false; 10],
            sol: [0; 9],
            row_sum: [0; 3],
            col_sum: [0; 3],
            cursor: 0,
            won: false,
            message: String::new(),
            paws: 0,
            dark: theme::detecte_sombre(),
            started: Instant::now(),
            last_draw: None,
        }
    }

    fn start(&mut self, level_idx: usize) {
        self.level_idx = level_idx;
        self.grid = [-1; 9];
        self.hint_cell = [false; 9];
        self.used = [false; 10];
        self.sol = [1, 2, 3, 4, 5, 6, 7, 8, 9];
        self.sol.shuffle(&mut rand::thread_rng());
        self.row_sum = [
            (self.sol[0] + self.sol[1] + self.sol[2]) as i32,
            (self.sol[3] + self.sol[4] + self.sol[5]) as i32,
            (self.sol[6] + self.sol[7] + self.sol[8]) as i32,
        ];
        self.col_sum = [
            (self.sol[0] + self.sol[3] + self.sol[6]) as i32,
            (self.sol[1] + self.sol[4] + self.sol[7]) as i32,
            (self.sol[2] + self.sol[5] + self.sol[8]) as i32,
        ];
        let n = LEVELS[level_idx].hints;
        let mut perm: Vec<usize> = (0..9).collect();
        perm.shuffle(&mut rand::thread_rng());
        for &p in perm.iter().take(n) {
            self.grid[p] = self.sol[p];
            self.hint_cell[p] = true;
            self.used[self.sol[p] as usize] = true;
        }
        self.cursor = 0;
        self.won = false;
        self.message = "Place les chiffres (๑˃ᴗ˂)ﻭ".into();
        self.menu = false;
    }

    fn row_sum_of(&self, i: usize) -> i32 {
        self.grid[3 * i] as i32 + self.grid[3 * i + 1] as i32 + self.grid[3 * i + 2] as i32
    }
    fn col_sum_of(&self, j: usize) -> i32 {
        self.grid[j] as i32 + self.grid[3 + j] as i32 + self.grid[6 + j] as i32
    }
    fn row_full(&self, i: usize) -> bool {
        self.grid[3 * i] > 0 && self.grid[3 * i + 1] > 0 && self.grid[3 * i + 2] > 0
    }
    fn col_full(&self, j: usize) -> bool {
        self.grid[j] > 0 && self.grid[3 + j] > 0 && self.grid[6 + j] > 0
    }
    fn full(&self) -> bool {
        (0..3).all(|i| self.row_full(i))
    }
    fn check_win(&self) -> bool {
        (0..3).all(|i| self.row_sum_of(i) == self.row_sum[i] && self.col_sum_of(i) == self.col_sum[i])
    }
}

fn colors(dark: bool) -> Theme {
    if dark {
        theme::SOMBRE
    } else {
        theme::CLAIR
    }
}

struct Palette {
    ok: Color,
    ko: Color,
    muted: Color,
    soft: Color,
    accent: Color,
}

fn palette(dark: bool) -> Palette {
    let t = colors(dark);
    Palette {
        ok: Color::Rgb(t.good.0, t.good.1, t.good.2),
        ko: Color::Rgb(t.ko.0, t.ko.1, t.ko.2),
        muted: if dark { Color::Rgb(154, 140, 160) } else { Color::Rgb(95, 85, 100) },
        soft: if dark { Color::Rgb(201, 187, 209) } else { Color::Rgb(75, 65, 85) },
        accent: Color::Rgb(t.accent.0, t.accent.1, t.accent.2),
    }
}

fn ui(f: &mut Frame, app: &App) {
    let area = f.area();
    let p = palette(app.dark);
    let title_s = Style::default().fg(p.accent).add_modifier(Modifier::BOLD);
    let menu_s = Style::default().fg(p.soft);
    let sel_s = Style::default().fg(p.accent).add_modifier(Modifier::BOLD);
    let msg_s = Style::default().fg(p.accent).add_modifier(Modifier::BOLD);
    let help_s = Style::default().fg(Color::DarkGray);

    if app.menu {
        let mut lines = vec![
            Line::from(Span::styled(" ♡  F U B U K I  ♡ ", title_s)),
            Line::from(""),
            Line::from(Span::styled("Choisis ton niveau :", menu_s)),
            Line::from(""),
        ];
        for (i, lv) in LEVELS.iter().enumerate() {
            let hint_word = if lv.hints > 1 { "indices" } else { "indice" };
            let prefix = format!("  [{}] ", i + 1);
            let line = format!("{:<9} · {} {}", lv.name, lv.hints, hint_word);
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

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(Span::styled(" ♡  F U B U K I  ♡ ", title_s)));
    lines.push(Line::from(Span::styled(format!("niveau : {}", LEVELS[app.level_idx].name), menu_s)));
    lines.push(Line::from(""));

    // palette 1..9
    let mut pal: Vec<Span> = Vec::new();
    for d in 1..=9 {
        let style = if app.used[d] {
            Style::default().fg(p.muted)
        } else {
            Style::default().fg(p.accent).add_modifier(Modifier::BOLD)
        };
        pal.push(Span::styled(format!("  {} ", d), style));
    }
    lines.push(Line::from(pal));
    lines.push(Line::from(""));

    // grille dessinée : cellules de 5 entre bordures, totaux intégrés
    let cellw = 5usize;
    let seg = "─".repeat(cellw);
    let border_s = Style::default().fg(p.muted);
    let sum_style = |sum: i32, target: i32, full: bool| {
        if !full {
            Style::default().fg(p.muted)
        } else if sum == target {
            Style::default().fg(p.ok).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(p.ko).add_modifier(Modifier::BOLD)
        }
    };

    let hline = |left: char, mid: char, right: char| -> Line<'static> {
        let mut s = String::new();
        s.push(left);
        for c in 0..3 {
            if c > 0 { s.push(mid); }
            s.push_str(&seg);
        }
        s.push(right);
        Line::from(Span::styled(s, border_s))
    };

    // cellule de grid OU total de colonne (dernière "ligne" = totaux)
    let cell_span = |ch: &str, style: Style| -> [Span; 2] {
        let total = cellw - ch.chars().count();
        let l = total / 2;
        [
            Span::styled("│", border_s),
            Span::styled(format!("{}{}{}", " ".repeat(l), ch, " ".repeat(total - l)), style),
        ]
    };

    lines.push(hline('┌', '┬', '┐'));
    for i in 0..3 {
        let mut spans: Vec<Span> = Vec::new();
        for j in 0..3 {
            let idx = 3 * i + j;
            let raw = if app.grid[idx] < 0 { "·" } else { &app.grid[idx].to_string() };
            let ch = if idx == app.cursor && !app.won {
                format!("[{}]", raw)
            } else {
                raw.to_string()
            };
            let ch = &ch;
            let style = if app.hint_cell[idx] {
                Style::default().fg(p.muted).add_modifier(Modifier::BOLD)
            } else if idx == app.cursor && !app.won {
                Style::default().fg(p.accent).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(p.soft)
            };
            let [bar, body] = cell_span(ch, style);
            spans.push(bar);
            spans.push(body);
        }
        spans.push(Span::styled("│", border_s));
        // total de ligne à droite (au-delà de la bordure)
        let s = sum_style(app.row_sum_of(i), app.row_sum[i], app.row_full(i));
        spans.push(Span::styled(format!("= {:>3}", app.row_sum[i]), s));
        lines.push(Line::from(spans));
        if i < 2 {
            lines.push(hline('├', '┼', '┤'));
        } else {
            lines.push(hline('└', '┴', '┘'));
        }
    }
    // ligne des totaux de colonnes : sous la grille, SANS bordure, mais avec
    // le même pas (5 chars/cellule + 1 séparation) pour tomber sous les colonnes
    let mut spans: Vec<Span> = Vec::new();
    for j in 0..3 {
        let s = sum_style(app.col_sum_of(j), app.col_sum[j], app.col_full(j));
        // 1 char de séparation (la bordure │ absente) + 5 chars de cellule
        spans.push(Span::raw(" "));
        let text = format!("={:>2}", app.col_sum[j]);
        let total = cellw - text.chars().count();
        let l = total / 2;
        spans.push(Span::styled(format!("{}{}{}", " ".repeat(l), text, " ".repeat(total - l)), s));
    }
    lines.push(Line::from(spans));
    lines.push(Line::from(""));

    lines.push(Line::from(Span::styled(app.message.clone(), msg_s)));
    lines.push(Line::from(""));

    if app.won {
        let n = 1 + app.paws.min(6);
        lines.push(Line::from(Span::styled("🐾 ♡ ".repeat(n), Style::default().fg(p.accent))));
        lines.push(Line::from(Span::styled(
            "[n] rejouer · esc menu · q quitter",
            help_s,
        )));
    } else {
        lines.push(Line::from(Span::styled(
            "↑↓←→ bouger · 1-9 placer · backspace effacer · n rejouer · esc menu",
            help_s,
        )));
    }

    // ratatui Center centre chaque ligne selon SA largeur → les lignes
    // courtes (hlines) dérivent face aux longues (grille + " = nn"). On
    // calcule la largeur max et on pad chaque ligne à droite pour que toutes
    // aient la même largeur, PUIS on centre le bloc.
    let width_of = |l: &Line| -> usize {
        l.spans.iter().map(|s| s.content.chars().count()).sum()
    };
    // largeur STRUCTURELLE : celle de la ligne de grille (la plus large et la
    // seule fixe). Les messages ne doivent jamais l'élargir (sinon tout le
    // bloc saute quand le texte change) → ils sont tronqués à cette largeur.
    // la ligne de GRILLE est reconnaissable : elle contient une bordure ┌ ou │
    // (et les totaux "= nn"). On la repère par son contenu, pas par sa taille.
    let gridw = lines
        .iter()
        .filter(|l| l.spans.iter().any(|s| s.content.chars().any(|c| "│┌├└".contains(c))))
        .map(width_of)
        .max()
        .unwrap_or(40);
    let maxw = gridw;
    let lines: Vec<Line> = lines
        .into_iter()
        .map(|mut l| {
            let lw = width_of(&l);
            let mut spans = std::mem::take(&mut l.spans);
            if lw > maxw {
                // tronquer le dernier span textuel
                let mut budget = lw - maxw;
                let mut out = Vec::new();
                for s in spans.drain(..) {
                    let slen = s.content.chars().count();
                    if slen <= budget {
                        budget -= slen;
                        out.push(s);
                    } else {
                        let keep = slen - budget;
                        let text: String = s.content.chars().take(keep).collect();
                        out.push(Span::styled(text, s.style));
                        budget = 0;
                    }
                }
                spans = out;
            }
            let lw: usize = spans.iter().map(|s| s.content.chars().count()).sum();
            if lw < maxw {
                spans.push(Span::raw(" ".repeat(maxw - lw)));
            }
            Line::from(spans)
        })
        .collect();
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
        let timeout = TICK.saturating_sub(app.started.elapsed());
        if event::poll(timeout)? {
            if let Event::Key(k) = event::read()? {
                if k.kind != KeyEventKind::Press {
                    continue;
                }
                if k.modifiers.contains(KeyModifiers::CONTROL) && k.code == KeyCode::Char('c') {
                    return Ok(());
                }
                if app.menu {
                    match k.code {
                        KeyCode::Char(c @ '1'..='3') => {
                            app.start(c as usize - '1' as usize)
                        }
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
                match k.code {
                    KeyCode::Char('q') if app.won => return Ok(()),
                    KeyCode::Esc => {
                        *app = App::new();
                        continue;
                    }
                    KeyCode::Char('n') => {
                        let lv = app.level_idx;
                        app.start(lv);
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        if app.cursor >= 3 { app.cursor -= 3; }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if app.cursor < 6 { app.cursor += 3; }
                    }
                    KeyCode::Left | KeyCode::Char('h') => {
                        if app.cursor % 3 > 0 { app.cursor -= 1; }
                    }
                    KeyCode::Right | KeyCode::Char('l') => {
                        if app.cursor % 3 < 2 { app.cursor += 1; }
                    }
                    KeyCode::Backspace | KeyCode::Delete | KeyCode::Char('x') | KeyCode::Char('0') => {
                        let v = app.grid[app.cursor];
                        if v > 0 && !app.hint_cell[app.cursor] {
                            app.used[v as usize] = false;
                            app.grid[app.cursor] = -1;
                            app.message = "Case vidée~".into();
                        } else if app.hint_cell[app.cursor] {
                            app.message = "C'est un indice, il reste ! (｡•̀ᴗ-)✧".into();
                        }
                    }
                    KeyCode::Char(c @ '1'..='9') => {
                        let d = c as u8 - b'0';
                        if app.hint_cell[app.cursor] {
                            app.message = "Case d'indice, déjà remplie !".into();
                            continue;
                        }
                        if app.used[d as usize] {
                            app.message = format!("Le {} est déjà posé ! (＞﹏＜)", d);
                            continue;
                        }
                        let old = app.grid[app.cursor];
                        if old > 0 {
                            app.used[old as usize] = false;
                        }
                        app.grid[app.cursor] = d as i8;
                        app.used[d as usize] = true;
                        app.message = format!("Un petit {} par ici ♡", d);
                        if app.full() {
                            if app.check_win() {
                                app.won = true;
                                app.message = "GAGNÉ ! Trop kawaii !! ☆*:.｡.o(≧▽≦)o.｡.:*☆".into();
                                app.started = Instant::now();
                            } else {
                                app.message = "Hmm, les sommes ne vont pas... (´･_･`)".into();
                            }
                        }
                    }
                    _ => {}
                }
            }
        } else {
            // tick
            if app.won {
                app.paws += 1;
            }
            app.started = Instant::now();
        }
        let _ = &app.last_draw;
    }
}

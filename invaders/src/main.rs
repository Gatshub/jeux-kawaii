//! Space Invaders kawaii — Ratatui. Rendu braille 2×4 points, comme la version Ruby.
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use jeux_core::theme;
use rand::Rng;
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame, Terminal,
};
use std::{io::stdout, time::Duration};

// ─── Constantes de gameplay (identiques à la version Ruby) ───────────────────

struct Level {
    name: &'static str,
    rows: usize,
    cols: usize,
    speed: u32,   // ticks par pas latéral
    bombs: u32,   // fréquence des bombes (×3 ticks)
    lives: i32,
}

const LEVELS: [Level; 3] = [
    Level { name: "facile", rows: 3, cols: 6, speed: 6, bombs: 4, lives: 4 },
    Level { name: "moyen", rows: 4, cols: 7, speed: 5, bombs: 6, lives: 3 },
    Level { name: "difficile", rows: 4, cols: 8, speed: 4, bombs: 9, lives: 2 },
];

/// Tick de base en ms, resserré au fil des kills (min TICK_MIN).
const TICK_BASE_MS: u64 = 55;
const TICK_MIN_MS: u64 = 40;

const DOTS_W: i32 = 88; // 44 colonnes braille
const DOTS_H: i32 = 60; // 15 lignes braille

const INVADER_SPRITES: [[(i32, i32); 10]; 2] = [
    // bouche ouverte
    [(0, 1), (1, 0), (2, 0), (3, 0), (4, 1), (0, 2), (4, 2), (1, 3), (3, 3), (-1, -1)],
    // bouche fermée
    [(0, 1), (1, 1), (2, 0), (3, 0), (4, 1), (0, 2), (4, 2), (1, 3), (2, 3), (3, 3)],
];
const SHIP_SPRITE: [(i32, i32); 7] = [(0, 3), (1, 2), (2, 0), (2, 1), (2, 2), (3, 2), (4, 3)];

const BUNKER_W: i32 = 12;
const BUNKER_H: i32 = 7;
const BUNKER_COUNT: usize = 4;
const BUNKER_Y: i32 = DOTS_H - 16;

// ─── App ─────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
struct Invader {
    row: usize,
    x: i32,
    y: i32,
    alive: bool,
}

#[derive(Clone)]
struct Bunker {
    dots: Vec<(i32, i32)>,
}

struct App {
    menu: bool,
    level_idx: usize,
    bunkers_on: bool,
    // état de jeu
    invaders: Vec<Invader>,
    bunkers: Vec<Bunker>,
    bullets: Vec<(i32, i32)>,
    bombs: Vec<(i32, i32)>,
    px: i32,
    dir: i32,
    frame: u32,
    lives: i32,
    score: u32,
    over: bool,
    won: bool,
    message: String,
    paws: u32,
    dark: bool,
    rng: rand::rngs::ThreadRng,
}

impl App {
    fn new() -> Self {
        App {
            menu: true,
            level_idx: 0,
            bunkers_on: true,
            invaders: vec![],
            bunkers: vec![],
            bullets: vec![],
            bombs: vec![],
            px: DOTS_W / 2 - 2,
            dir: 1,
            frame: 0,
            lives: 0,
            score: 0,
            over: false,
            won: false,
            message: String::new(),
            paws: 0,
            dark: theme::detecte_sombre(),
            rng: rand::thread_rng(),
        }
    }

    fn start(&mut self, level_idx: usize) {
        let lv = &LEVELS[level_idx];
        self.menu = false;
        self.level_idx = level_idx;
        self.lives = lv.lives;
        self.score = 0;
        self.px = DOTS_W / 2 - 2;
        self.bullets.clear();
        self.bombs.clear();
        self.dir = 1;
        self.frame = 0;
        self.over = false;
        self.won = false;
        self.message = "Défends la galaxie (๑˃ᴗ˂)ﻭ".into();
        self.invaders.clear();
        for r in 0..lv.rows {
            for c in 0..lv.cols {
                self.invaders.push(Invader {
                    row: r,
                    x: 8 + c as i32 * 8,
                    y: 2 + r as i32 * 7,
                    alive: true,
                });
            }
        }
        // bunkers
        self.bunkers.clear();
        if !self.bunkers_on {
            return;
        }
        let span = DOTS_W / BUNKER_COUNT as i32;
        for k in 0..BUNKER_COUNT as i32 {
            let bx = k * span + (span - BUNKER_W) / 2;
            let mut dots = Vec::new();
            for dy in 0..BUNKER_H {
                for dx in 0..BUNKER_W {
                    if dy == 0 && (dx < 2 || dx >= BUNKER_W - 2) { continue; }
                    if dy == 1 && (dx < 1 || dx >= BUNKER_W - 1) { continue; }
                    if dy >= BUNKER_H - 2 && dx >= 4 && dx < 8 { continue; }
                    dots.push((bx + dx, BUNKER_Y + dy));
                }
            }
            self.bunkers.push(Bunker { dots });
        }
    }

    fn alive_count(&self) -> usize {
        self.invaders.iter().filter(|i| i.alive).count()
    }

    fn tick_delay_ms(&self) -> u64 {
        let total = self.invaders.len().max(1);
        let progress = self.alive_count() as f64 / total as f64;
        (TICK_MIN_MS as f64 + (TICK_BASE_MS - TICK_MIN_MS) as f64 * progress) as u64
    }

    fn invader_dots(&self, inv: &Invader) -> Vec<(i32, i32)> {
        let sprite = &INVADER_SPRITES[(self.frame % 2) as usize];
        sprite
            .iter()
            .filter(|&&(dx, dy)| dx >= 0)
            .map(|&(dx, dy)| (inv.x + dx, inv.y + dy))
            .collect()
    }

    fn ship_dots(&self) -> Vec<(i32, i32)> {
        SHIP_SPRITE.iter().map(|&(dx, dy)| (self.px + dx, DOTS_H - 4 + dy)).collect()
    }

    /// Impact bunker : érode le point + voisins. true si touché.
    fn bunker_hit(&mut self, x: i32, y: i32) -> bool {
        for b in &mut self.bunkers {
            let struck: Vec<(i32, i32)> = b
                .dots
                .iter()
                .filter(|&&(dx, dy)| (dx - x).abs() <= 1 && (dy - y).abs() <= 1)
                .copied()
                .collect();
            if struck.is_empty() {
                continue;
            }
            let sx = struck.iter().map(|d| d.0).sum::<i32>() / struck.len() as i32;
            let sy = struck.iter().map(|d| d.1).sum::<i32>() / struck.len() as i32;
            b.dots.retain(|&(dx, dy)| (dx - sx).abs() > 1 || (dy - sy).abs() > 1);
            return true;
        }
        false
    }

    fn shoot(&mut self) {
        if self.over || !self.bullets.is_empty() {
            return;
        }
        self.bullets.push((self.px + 2, DOTS_H - 5));
        self.message = "Petit ♡ parti !".into();
    }

    fn step(&mut self) {
        let lv = &LEVELS[self.level_idx];
        self.frame += 1;

        // envahisseurs : pas latéral tous les `speed` ticks
        if self.frame % lv.speed == 0 {
            let xs: Vec<i32> = self.invaders.iter().filter(|i| i.alive).map(|i| i.x).collect();
            if !xs.is_empty() {
                let min_x = *xs.iter().min().unwrap();
                let max_x = *xs.iter().max().unwrap() + 5;
                let step = self.dir * 2;
                if (self.dir > 0 && max_x + step > DOTS_W - 2)
                    || (self.dir < 0 && min_x + step < 2)
                {
                    self.dir = -self.dir;
                    for i in self.invaders.iter_mut().filter(|i| i.alive) {
                        i.y += 4;
                    }
                } else {
                    for i in self.invaders.iter_mut().filter(|i| i.alive) {
                        i.x += step;
                    }
                }
                if let Some(bottom) = self.invaders.iter().filter(|i| i.alive).map(|i| i.y).max() {
                    if bottom >= DOTS_H - 12 {
                        self.over = true;
                        self.won = false;
                        self.message = "Ils ont débarqué... (´；ω；`)".into();
                        return;
                    }
                }
            }
        }

        // bombes
        if self.frame % (lv.bombs * 3) == 0 {
            let living: Vec<usize> = self
                .invaders
                .iter()
                .enumerate()
                .filter(|(_, i)| i.alive)
                .map(|(k, _)| k)
                .collect();
            let pick = self.rng.gen_range(0..living.len());
            if let Some(&k) = living.get(pick) {
                let s = self.invaders[k];
                self.bombs.push((s.x + 2, s.y + 4));
            }
        }
        for b in self.bombs.iter_mut() {
            b.1 += 3;
        }
        let ship = self.ship_dots();
        let bombs_snapshot = self.bombs.clone();
        let mut kept = Vec::new();
        for &b in &bombs_snapshot {
            if !(self.bunker_hit(b.0, b.1) || self.bunker_hit(b.0, b.1 - 1)) {
                kept.push(b);
            }
        }
        self.bombs = kept;
        if self.bombs.iter().any(|&b| {
            ship.iter()
                .any(|&(sx, sy)| (sx - b.0).abs() <= 1 && (sy - b.1).abs() <= 1)
        }) {
            self.bombs.clear();
            self.lives -= 1;
            self.message = format!("Aïe ! Il te reste {} ♡ (＞﹏＜)", self.lives);
            if self.lives <= 0 {
                self.over = true;
                self.won = false;
                self.message = "Game over... (´；ω；`) [n] pour recommencer ♪".into();
            }
            return;
        }
        self.bombs.retain(|&b| b.1 < DOTS_H - 3);

        // balle du joueur : 4 points par tick
        for b in self.bullets.iter_mut() {
            b.1 -= 4;
        }
        let mut msgs: Vec<String> = Vec::new();
        let bullets_snapshot = self.bullets.clone();
        let mut kept_bullets = Vec::new();
        for &b in &bullets_snapshot {
            if self.bunker_hit(b.0, b.1) || self.bunker_hit(b.0, b.1 + 1) {
                msgs.push("Ton ♡ s'est écrasé sur le barrage... (･_･;".into());
                continue;
            }
            let hit = self.invaders.iter().position(|i| {
                i.alive
                    && self
                        .invader_dots(i)
                        .iter()
                        .any(|&(dx, dy)| (dx - b.0).abs() <= 1 && (dy - b.1).abs() <= 1)
            });
            if let Some(k) = hit {
                self.invaders[k].alive = false;
                self.score += (4 - self.invaders[k].row) as u32 * 10;
                if self.alive_count() == 0 {
                    self.over = true;
                    self.won = true;
                    self.message = "GAGNÉ ! Galaxie sauvée !! ☆*:.｡.o(≧▽≦)o.｡.:*☆".into();
                } else {
                    self.message = format!("Un envahisseur de moins ! {} pts ♪", self.score);
                }
                continue;
            }
            kept_bullets.push(b);
        }
        self.bullets = kept_bullets;
        if let Some(m) = msgs.first() {
            if !self.over {
                self.message = m.clone();
            }
        }
        self.bullets.retain(|&b| b.1 >= 0);
    }
}

// ─── Rendu braille ───────────────────────────────────────────────────────────

const BRAILLE_BASE: u32 = 0x2800;
fn dot_bit(cx: i32, cy: i32) -> u32 {
    match (cx, cy) {
        (0, 0) => 1, (0, 1) => 2, (0, 2) => 4, (1, 0) => 8,
        (1, 1) => 16, (1, 2) => 32, (0, 3) => 64, (1, 3) => 128,
        _ => 0,
    }
}

fn ui(f: &mut Frame, app: &App) {
    let area = f.area();
    let dark = app.dark;
    let t = if dark { theme::SOMBRE } else { theme::CLAIR };
    let accent = Color::Rgb(t.accent.0, t.accent.1, t.accent.2);
    let ok = Color::Rgb(t.good.0, t.good.1, t.good.2);
    let ko = Color::Rgb(t.ko.0, t.ko.1, t.ko.2);
    let soft = if dark { Color::Rgb(201, 187, 209) } else { Color::Rgb(110, 90, 117) };
    let title_s = Style::default().fg(accent).add_modifier(Modifier::BOLD);
    let hud_s = Style::default().fg(soft);
    let msg_s = Style::default().fg(accent).add_modifier(Modifier::BOLD);
    let help_s = Style::default().fg(Color::DarkGray);

    if app.menu {
        let mut lines = vec![
            Line::from(Span::styled(" ♡  S P A C E   I N V A D E R S  ♡ ", title_s)),
            Line::from(""),
            Line::from(Span::styled("Choisis ton niveau :", hud_s)),
            Line::from(""),
        ];
        for (i, lv) in LEVELS.iter().enumerate() {
            let prefix = format!("  [{}] ", i + 1);
            let line = format!(
                "{:<9} · {:<11} · {} ♡",
                lv.name,
                format!("{}x{}", lv.rows, lv.cols),
                lv.lives
            );
            let style = if i == app.level_idx {
                Style::default().fg(accent).add_modifier(Modifier::BOLD)
            } else {
                hud_s
            };
            lines.push(Line::from(Span::styled(prefix + &line, style)));
        }
        lines.push(Line::from(""));
        let state = if app.bunkers_on {
            Span::styled("on", Style::default().fg(ok).add_modifier(Modifier::BOLD))
        } else {
            Span::styled("off", Style::default().fg(ko).add_modifier(Modifier::BOLD))
        };
        lines.push(Line::from(vec![
            Span::styled("  [b] bunkers : ", hud_s),
            state,
        ]));
        lines.push(Line::from(Span::styled(
            "↑↓ choisir · entrée jouer · 1/2/3 direct · b bunkers · q quitter",
            help_s,
        )));
        f.render_widget(Paragraph::new(lines).alignment(ratatui::layout::Alignment::Center), area);
        return;
    }

    // ── terrain braille ──
    let cols = (DOTS_W / 2) as usize;
    let rows = (DOTS_H / 4) as usize;
    // (bits, couleur) par cellule ; couleur du dernier point dessiné
    let mut cell_bits = vec![vec![0u32; cols]; rows];
    let mut cell_col = vec![vec![soft; cols]; rows];
    let mut draw = |dots: &[(i32, i32)], style: Color, bits: &mut Vec<Vec<u32>>, cols_: &mut Vec<Vec<Color>>| {
        for &(x, y) in dots {
            let (cy, cx) = (y / 4, x / 2);
            if cy < 0 || cx < 0 || cy as usize >= rows || cx as usize >= cols {
                continue;
            }
            bits[cy as usize][cx as usize] |= dot_bit(x.rem_euclid(2), y.rem_euclid(4));
            cols_[cy as usize][cx as usize] = style;
        }
    };
    for inv in &app.invaders {
        if inv.alive {
            let dots = app.invader_dots(inv);
            draw(&dots, accent, &mut cell_bits, &mut cell_col);
        }
    }
    let ship = app.ship_dots();
    draw(&ship, ok, &mut cell_bits, &mut cell_col);
    for b in &app.bunkers {
        draw(&b.dots, soft, &mut cell_bits, &mut cell_col);
    }
    let bullets: Vec<(i32, i32)> = app.bullets.clone();
    draw(&bullets, ok, &mut cell_bits, &mut cell_col);
    let bombs: Vec<(i32, i32)> = app.bombs.clone();
    draw(&bombs, ko, &mut cell_bits, &mut cell_col);

    // HUD
    let hearts = "♡ ".repeat(app.lives.max(0) as usize);
    lines_hud(f, area, app, title_s, hud_s, ok);

    // terrain (une Line par rangée braille, Span par run de même couleur)
    let mut terrain: Vec<Line> = Vec::new();
    for r in 0..rows {
        let mut spans: Vec<Span> = Vec::new();
        let mut run = String::new();
        let mut run_col = cell_col[r][0];
        for c in 0..cols {
            let col = cell_col[r][c];
            if col != run_col && !run.is_empty() {
                spans.push(Span::styled(std::mem::take(&mut run), Style::default().fg(run_col)));
            }
            run_col = col;
            let code = BRAILLE_BASE + cell_bits[r][c];
            run.push(char::from_u32(code).unwrap_or(' '));
        }
        spans.push(Span::styled(run, Style::default().fg(run_col)));
        terrain.push(Line::from(spans));
    }
    let th = terrain.len() as u16;
    let ty = area.y + 3;
    if ty + th <= area.y + area.height {
        f.render_widget(
            Paragraph::new(terrain),
            ratatui::layout::Rect { x: area.x, y: ty, width: area.width, height: th },
        );
    }

    // messages bas
    let bottom = ty + th;
    if bottom < area.y + area.height {
        let mut lines = vec![
            Line::from(Span::styled(app.message.clone(), msg_s)),
            Line::from(Span::styled(
                if app.over {
                    "[n] rejouer · esc menu · q quitter".to_string()
                } else {
                    "←→ bouger · espace tirer · n rejouer · esc menu".to_string()
                },
                help_s,
            )),
        ];
        if app.won {
            lines.insert(0, Line::from(Span::styled("🐾 ♡ ".repeat(1 + app.paws.min(6) as usize), Style::default().fg(accent))));
        }
        f.render_widget(
            Paragraph::new(lines),
            ratatui::layout::Rect {
                x: area.x,
                y: bottom,
                width: area.width,
                height: area.y + area.height - bottom,
            },
        );
    }
}

fn lines_hud(f: &mut Frame, area: ratatui::layout::Rect, app: &App, title_s: Style, hud_s: Style, ok: Color) {
    let hearts = "♡ ".repeat(app.lives.max(0) as usize);
    let line = Line::from(vec![
        Span::styled(format!("score : {}", app.score), hud_s),
        Span::raw("   "),
        Span::styled(format!("niveau : {}", LEVELS[app.level_idx].name), hud_s),
        Span::raw("   "),
        Span::styled(format!("vies : {}", hearts.trim_end()), Style::default().fg(ok)),
    ]);
    f.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(" ♡  S P A C E   I N V A D E R S  ♡ ", title_s)),
            line,
            Line::from(""),
        ]),
        ratatui::layout::Rect { x: area.x, y: area.y, width: area.width, height: 3 },
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
        let delay = Duration::from_millis(if app.menu { 150 } else { app.tick_delay_ms() });
        let has_event = event::poll(delay)?;
        if has_event {
            if let Event::Key(k) = event::read()? {
                if k.kind != KeyEventKind::Press { continue; }
                if k.modifiers.contains(KeyModifiers::CONTROL) && k.code == KeyCode::Char('c') {
                    return Ok(());
                }
                if app.menu {
                    match k.code {
                        KeyCode::Char(c @ '1'..='3') => app.start(c as usize - '1' as usize),
                        KeyCode::Char('b') => app.bunkers_on = !app.bunkers_on,
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
                    KeyCode::Char('q') if app.over => return Ok(()),
                    KeyCode::Esc => { app.menu = true; continue; }
                    KeyCode::Char('n') => {
                        let lv = app.level_idx;
                        app.start(lv);
                    }
                    KeyCode::Left | KeyCode::Char('h') => {
                        if !app.over { app.px = (app.px - 2).max(1); }
                    }
                    KeyCode::Right | KeyCode::Char('l') => {
                        if !app.over { app.px = (app.px + 2).min(DOTS_W - 6); }
                    }
                    KeyCode::Char(' ') | KeyCode::Up | KeyCode::Char('w') => app.shoot(),
                    _ => {}
                }
            }
        } else if !app.menu && !app.over {
            app.step();
            if app.won {
                app.paws += 1;
            }
        } else if app.won {
            app.paws += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn victoire_par_kill_complet() {
        let mut app = App::new();
        app.start(0); // facile 3x6 = 18 envahisseurs
        let total = app.invaders.len();
        assert_eq!(app.alive_count(), total);
        // tuer tout le monde en plaçant la balle sur chaque envahisseur et en
        // appelant step() (téléportation, pas de timing)
        for k in 0..total {
            let inv = app.invaders[k];
            // la balle avance de 4 pts/tick : la placer SOUS l'envahisseur
            // pour qu'elle traverse sa zone (y+6 → y+2 → collision)
            let mut guard = 0;
            while app.invaders[k].alive && guard < 50 {
                if app.bullets.is_empty() {
                    let inv = app.invaders[k];
                    app.bullets = vec![(inv.x + 2, inv.y + 6)];
                }
                app.step();
                guard += 1;
            }
            assert!(!app.invaders[k].alive, "kill {} raté", k);
        }
        assert!(app.won, "victoire non détectée");
        assert!(app.over);
        assert!(app.score > 0);
}

    #[test]
    fn defaite_par_debarquement() {
        let mut app = App::new();
        app.start(0);
        // téléporter la formation en bas
        for i in app.invaders.iter_mut() {
            i.y = DOTS_H - 12;
            i.x = 8;
        }
        app.frame = LEVELS[0].speed - 1; // step() incrémentera → multiple de speed
        app.step();
        assert!(app.over && !app.won, "débarquement non détecté");
        assert!(app.message.contains("débarqué"));
    }
}

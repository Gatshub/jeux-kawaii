//! Space Invaders kawaii — version graphique (macroquad).
//! Fenêtre 960×660, sprites dessinés (crabes animés, vaisseau, bunkers destructibles),
//! explosions en particules, screen-shake, thème pastel adaptatif clair/sombre ('t').
//!
//! Gameplay transposé des constantes de la version Ruby (formation, vitesses,
//! vies, scores) à l'échelle graphique.

use macroquad::prelude::*;
use ::rand::Rng;  // trait gen_range (rand 0.8)

const W: f32 = 960.0;
const H: f32 = 660.0;

const LEVELS: [(&str, usize, usize, f32, f32, i32); 3] = [
    // nom, rows, cols, sec/pas, sec/bombe, vies
    ("facile", 3, 6, 0.42, 1.45, 4),
    ("moyen", 4, 7, 0.34, 1.10, 3),
    ("difficile", 4, 8, 0.27, 0.80, 2),
];

const SHIP_SPEED: f32 = 420.0;
const BULLET_SPEED: f32 = 620.0;
const BOMB_SPEED: f32 = 260.0;
const STEP: f32 = 18.0;
const DESCENT: f32 = 34.0;

#[derive(Clone, Copy)]
struct Palette {
    bg: Color,
    accent: Color,
    ok: Color,
    ko: Color,
    soft: Color,
    text: Color,
    dim: Color,
}

fn palette(dark: bool) -> Palette {
    if dark {
        Palette {
            bg: Color::new(0.07, 0.06, 0.09, 1.0),
            accent: Color::from_hex(0xFF9EC4),
            ok: Color::from_hex(0x7BD8A8),
            ko: Color::from_hex(0xFF7B8A),
            soft: Color::from_hex(0xC9BBD1),
            text: Color::from_hex(0xEDE6EF),
            dim: Color::from_hex(0x6E5F73),
        }
    } else {
        Palette {
            bg: Color::new(0.97, 0.95, 0.97, 1.0),
            accent: Color::from_hex(0xD6488C),
            ok: Color::from_hex(0x1F7A4D),
            ko: Color::from_hex(0xB3243C),
            soft: Color::from_hex(0x6E5A75),
            text: Color::from_hex(0x2A2130),
            dim: Color::from_hex(0x8A7D8F),
        }
    }
}

#[derive(Clone, Copy)]
struct Invader {
    row: usize,
    col: usize,
    alive: bool,
}

#[derive(Clone, Copy)]
struct Particle {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    life: f32,
    col: Color,
}

#[derive(Clone, Copy)]
struct Shard {
    x: f32,
    y: f32,
    alive: bool,
}

struct App {
    menu: bool,
    level: usize,
    bunkers_on: bool,
    invaders: Vec<Invader>,
    shards: Vec<Shard>,
    ship_x: f32,
    bullets: Vec<(f32, f32)>,
    bombs: Vec<(f32, f32)>,
    parts: Vec<Particle>,
    dir: f32,
    step_t: f32,
    bomb_t: f32,
    frame: u32,
    lives: i32,
    score: u32,
    over: bool,
    won: bool,
    message: String,
    shake: f32,
    flash: f32,
    dark: bool,
    form_x: f32,
    descent: f32,
}

fn invader_xy(row: usize, col: usize, form_x: f32, descent: f32) -> (f32, f32) {
    (form_x + col as f32 * 72.0, 90.0 + descent + row as f32 * 62.0)
}

impl App {
    fn new() -> Self {
        App {
            menu: true,
            level: 0,
            bunkers_on: true,
            invaders: vec![],
            shards: vec![],
            ship_x: W / 2.0,
            bullets: vec![],
            bombs: vec![],
            parts: vec![],
            dir: 1.0,
            step_t: 0.0,
            bomb_t: 0.0,
            frame: 0,
            lives: 0,
            score: 0,
            over: false,
            won: false,
            message: String::new(),
            shake: 0.0,
            flash: 0.0,
            dark: true,
            form_x: 60.0,
            descent: 0.0,
        }
    }

    fn start(&mut self, level: usize) {
        let (_, rows, cols, _, _, lives) = LEVELS[level];
        self.menu = false;
        self.level = level;
        self.lives = lives;
        self.score = 0;
        self.ship_x = W / 2.0;
        self.bullets.clear();
        self.bombs.clear();
        self.parts.clear();
        self.dir = 1.0;
        self.step_t = 0.0;
        self.bomb_t = 0.0;
        self.frame = 0;
        self.over = false;
        self.won = false;
        self.shake = 0.0;
        self.flash = 0.0;
        self.form_x = 60.0;
        self.descent = 0.0;
        self.message = "Défends la galaxie (๑˃ᴗ˂)ﻭ".into();
        self.invaders = (0..rows)
            .flat_map(|r| (0..cols).map(move |c| Invader { row: r, col: c, alive: true }))
            .collect();
        self.shards.clear();
        if !self.bunkers_on {
            return;
        }
        let span = W / 4.0;
        for k in 0..4i32 {
            let bx = k as f32 * span + (span - 110.0) / 2.0;
            let by = H - 190.0;
            for r in 0..6i32 {
                for c in 0..11i32 {
                    let roof = r == 0 && (c < 2 || c >= 9);
                    let roof2 = r == 1 && (c < 1 || c >= 10);
                    let cavity = r >= 4 && c >= 4 && c <= 6;
                    if roof || roof2 || cavity {
                        continue;
                    }
                    self.shards.push(Shard {
                        x: bx + c as f32 * 10.0,
                        y: by + r as f32 * 10.0,
                        alive: true,
                    });
                }
            }
        }
    }

    fn explode(&mut self, x: f32, y: f32, col: Color, n: usize) {
        let mut rng = ::rand::thread_rng();
        for _ in 0..n {
            let a = rng.gen_range(0.0..std::f32::consts::TAU);
            let sp = rng.gen_range(60.0..280.0);
            self.parts.push(Particle {
                x,
                y,
                vx: a.cos() * sp,
                vy: a.sin() * sp - 40.0,
                life: rng.gen_range(0.35..0.8),
                col,
            });
        }
    }
}

fn hit_shards(app: &mut App, x: f32, y: f32, r: f32) -> bool {
    let mut hit = false;
    for s in app.shards.iter_mut() {
        if s.alive && (s.x + 5.0 - x).abs() <= r && (s.y + 5.0 - y).abs() <= r {
            s.alive = false;
            hit = true;
        }
    }
    hit
}

trait MulA { fn mul_alpha(self, a: u8) -> Color; }
impl MulA for Color {
    fn mul_alpha(self, a: u8) -> Color {
        Color::new(self.r, self.g, self.b, a as f32 / 255.0)
    }
}

fn window_conf() -> Conf {
    Conf {
        window_title: "♡ Space Invaders kawaii ♡".to_owned(),
        window_width: W as i32,
        window_height: H as i32,
        window_resizable: false,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut app = App::new();
    let mut pal = palette(app.dark);

    loop {
        let dt = get_frame_time();
        clear_background(pal.bg);

        if is_key_pressed(KeyCode::T) {
            app.dark = !app.dark;
            pal = palette(app.dark);
        }

        // étoiles de fond
        for i in 0..40 {
            let x = ((i * 137) % 940) as f32 + 10.0;
            let y = ((i * 293) % 620) as f32 + 10.0;
            let tw = 0.4
                + 0.6
                    * ((get_time() * (0.5 + (i % 5) as f64 * 0.2) + i as f64).sin() * 0.5 + 0.5);
            draw_circle(x, y, 1.2 + (i % 3) as f32 * 0.5, pal.dim.mul_alpha((tw * 255.0) as u8));
        }

        if app.menu {
            menu_ui(&mut app, pal);
        } else {
            game_update(&mut app, &pal, dt);
            game_render(&app, pal);
        }

        next_frame().await
    }
}

fn menu_ui(app: &mut App, pal: Palette) {
    let cx = W / 2.0;
    let title = " ♡  S P A C E   I N V A D E R S  ♡ ";
    let tw = measure_text(title, None, 46, 1.0).width;
    draw_text(title, cx - tw / 2.0, 150.0, 46.0, pal.accent);

    let sub = "Choisis ton niveau :";
    draw_text(
        sub,
        cx - measure_text(sub, None, 22, 1.0).width / 2.0,
        250.0,
        22.0,
        pal.soft,
    );

    for (i, (name, rows, cols, _, _, lives)) in LEVELS.iter().enumerate() {
        let y = 320.0 + i as f32 * 56.0;
        let label = format!("{}  ·  {}x{}  ·  {} ♡", name, rows, cols, lives);
        let sel = i == app.level;
        let col = if sel { pal.accent } else { pal.soft };
        let lw = measure_text(&label, None, 26, 1.0).width;
        if sel {
            draw_text("▸", cx - lw / 2.0 - 34.0, y, 26.0, col);
            draw_rectangle_lines(cx - lw / 2.0 - 20.0, y - 28.0, lw + 40.0, 40.0, 2.0, pal.accent);
            if is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::Space) {
                app.start(i);
            }
        }
        draw_text(&label, cx - lw / 2.0, y, 26.0, col);
        if is_mouse_button_pressed(MouseButton::Left) {
            let (mx, my) = mouse_position();
            if mx > cx - lw / 2.0 - 20.0 && mx < cx + lw / 2.0 + 20.0 && my > y - 28.0 && my < y + 12.0
            {
                app.level = i;
                app.start(i);
            }
        }
    }
    if is_key_pressed(KeyCode::Up) && app.level > 0 {
        app.level -= 1;
    }
    if is_key_pressed(KeyCode::Down) && app.level < LEVELS.len() - 1 {
        app.level += 1;
    }
    if is_key_pressed(KeyCode::Key1) || is_key_pressed(KeyCode::Kp1) {
        app.start(0);
    }
    if is_key_pressed(KeyCode::Key2) || is_key_pressed(KeyCode::Kp2) {
        app.start(1);
    }
    if is_key_pressed(KeyCode::Key3) || is_key_pressed(KeyCode::Kp3) {
        app.start(2);
    }

    let blabel = format!("  [b] bunkers : {}", if app.bunkers_on { "on" } else { "off" });
    draw_text(
        &blabel,
        cx - measure_text(&blabel, None, 20, 1.0).width / 2.0,
        520.0,
        20.0,
        pal.soft,
    );
    if is_key_pressed(KeyCode::B) {
        app.bunkers_on = !app.bunkers_on;
    }
    let help = "↑↓ choisir · entrée jouer · 1/2/3 direct · b bunkers · t thème · q quitter";
    draw_text(
        help,
        cx - measure_text(help, None, 16, 1.0).width / 2.0,
        590.0,
        16.0,
        pal.dim,
    );
    if is_key_pressed(KeyCode::Q) || is_key_pressed(KeyCode::Escape) {
        std::process::exit(0);
    }
}

fn game_update(app: &mut App, pal: &Palette, dt: f32) {
    app.shake = (app.shake - dt * 3.0).max(0.0);
    app.flash = (app.flash - dt * 4.0).max(0.0);

    if is_key_pressed(KeyCode::Escape) {
        app.menu = true;
        return;
    }
    if is_key_pressed(KeyCode::Q) {
        std::process::exit(0);
    }

    if app.over {
        if is_key_pressed(KeyCode::N) || is_key_pressed(KeyCode::Enter) {
            let lv = app.level;
            app.start(lv);
        }
        if is_key_pressed(KeyCode::Escape) {
            app.menu = true;
        }
        for p in app.parts.iter_mut() {
            p.x += p.vx * dt;
            p.y += p.vy * dt;
            p.vy += 300.0 * dt;
            p.life -= dt;
        }
        app.parts.retain(|p| p.life > 0.0);
        return;
    }

    // vaisseau
    if is_key_down(KeyCode::Left) || is_key_down(KeyCode::H) {
        app.ship_x = (app.ship_x - SHIP_SPEED * dt).max(30.0);
    }
    if is_key_down(KeyCode::Right) || is_key_down(KeyCode::L) {
        app.ship_x = (app.ship_x + SHIP_SPEED * dt).min(W - 30.0);
    }
    if (is_key_pressed(KeyCode::Space) || is_key_pressed(KeyCode::W)) && app.bullets.len() < 2 {
        app.bullets.push((app.ship_x, H - 60.0));
    }

    // formation
    let (_, _, sec_step0, sec_bomb, _, _lives) = LEVELS[app.level];
    let total = app.invaders.len().max(1);
    let alive = app.invaders.iter().filter(|i| i.alive).count();
    let sec_step = sec_step0 as f32 * (alive as f32 / total as f32).max(0.35);
    app.step_t += dt;
    if app.step_t >= sec_step {
        app.step_t = 0.0;
        app.frame += 1;
        let xs: Vec<f32> = app
            .invaders
            .iter()
            .filter(|i| i.alive)
            .map(|i| invader_xy(i.row, i.col, app.form_x, app.descent).0)
            .collect();
        if !xs.is_empty() {
            let min_x = xs.iter().cloned().fold(f32::INFINITY, f32::min);
            let max_x = xs.iter().cloned().fold(f32::NEG_INFINITY, f32::max) + 54.0;
            if (app.dir > 0.0 && max_x + STEP > W - 20.0)
                || (app.dir < 0.0 && min_x - STEP < 20.0)
            {
                app.dir = -app.dir;
                app.descent += DESCENT;
            } else {
                app.form_x += app.dir * STEP;
            }
        }
        let max_y = app
            .invaders
            .iter()
            .filter(|i| i.alive)
            .map(|i| invader_xy(i.row, i.col, app.form_x, app.descent).1)
            .fold(0.0, f32::max);
        if max_y >= H - 260.0 {
            app.over = true;
            app.won = false;
            app.message = "Ils ont débarqué... (´；ω；`)".into();
        }
    }

    // bombes
    app.bomb_t += dt;
    if app.bomb_t >= sec_bomb {
        app.bomb_t = 0.0;
        let mut rng = ::rand::thread_rng();
        let living: Vec<&Invader> = app.invaders.iter().filter(|i| i.alive).collect();
        if !living.is_empty() {
            let inv = living[rng.gen_range(0..living.len())];
            let (x, y) = invader_xy(inv.row, inv.col, app.form_x, app.descent);
            app.bombs.push((x + 27.0, y + 40.0));
        }
    }
    for b in app.bombs.iter_mut() {
        b.1 += BOMB_SPEED * dt;
    }
    let bombs_snap = app.bombs.clone();
    app.bombs.clear();
    for &(x, y) in &bombs_snap {
        if hit_shards(app, x, y + 4.0, 9.0) {
            app.explode(x, y, pal.soft, 8);
        } else {
            app.bombs.push((x, y));
        }
    }
    // collision bombes → vaisseau (AVANT rejet hors champ)
    let ship_y = H - 52.0;
    if app
        .bombs
        .iter()
        .any(|&(x, y)| y + 6.0 >= ship_y && (x - app.ship_x).abs() < 26.0)
    {
        app.bombs.clear();
        app.lives -= 1;
        app.explode(app.ship_x, ship_y, pal.ko, 30);
        app.shake = 1.0;
        app.flash = 0.6;
        if app.lives <= 0 {
            app.over = true;
            app.message = "Game over... (´；ω；`) [n] pour recommencer ♪".into();
        } else {
            app.message = format!("Aïe ! Il te reste {} ♡ (＞﹏＜)", app.lives);
        }
    }
    app.bombs.retain(|&(_, y)| y < H - 20.0);

    // balles
    for b in app.bullets.iter_mut() {
        b.1 -= BULLET_SPEED * dt;
    }
    let bullets_snap = app.bullets.clone();
    app.bullets.clear();
    'outer: for &(x, y) in &bullets_snap {
        if y < 20.0 {
            continue;
        }
        if hit_shards(app, x, y - 4.0, 9.0) {
            app.explode(x, y, pal.soft, 6);
            continue;
        }
        for inv in app.invaders.iter_mut() {
            if !inv.alive {
                continue;
            }
            let (ix, iy) = invader_xy(inv.row, inv.col, app.form_x, app.descent);
            if x > ix - 4.0 && x < ix + 58.0 && y > iy - 4.0 && y < iy + 44.0 {
                inv.alive = false;
                app.score += (4 - inv.row as u32) * 10;
                app.explode(ix + 27.0, iy + 20.0, pal.accent, 26);
                app.shake = 0.4;
                let remaining = app.invaders.iter().filter(|i| i.alive).count();
                if remaining == 0 {
                    app.over = true;
                    app.won = true;
                    app.message = "GAGNÉ ! Galaxie sauvée !! ☆*:.｡.o(≧▽≦)o.｡.:*☆".into();
                } else {
                    app.message = format!("Un envahisseur de moins ! {} pts ♪", app.score);
                }
                continue 'outer;
            }
        }
        app.bullets.push((x, y));
    }

    for p in app.parts.iter_mut() {
        p.x += p.vx * dt;
        p.y += p.vy * dt;
        p.vy += 300.0 * dt;
        p.life -= dt;
    }
    app.parts.retain(|p| p.life > 0.0);
}

fn draw_invader(x: f32, y: f32, frame: u32, col: Color) {
    let open = frame % 2 == 0;
    draw_rectangle(x + 8.0, y + 10.0, 38.0, 24.0, col);
    draw_circle(x + 12.0, y + 12.0, 7.0, col);
    draw_circle(x + 42.0, y + 12.0, 7.0, col);
    if open {
        draw_rectangle(x + 2.0, y + 30.0, 10.0, 5.0, col);
        draw_rectangle(x + 42.0, y + 30.0, 10.0, 5.0, col);
        draw_rectangle(x, y + 18.0, 8.0, 5.0, col);
        draw_rectangle(x + 46.0, y + 18.0, 8.0, 5.0, col);
    } else {
        draw_rectangle(x + 4.0, y + 32.0, 10.0, 4.0, col);
        draw_rectangle(x + 40.0, y + 32.0, 10.0, 4.0, col);
    }
    draw_circle(x + 20.0, y + 20.0, 3.2, Color::new(0.08, 0.05, 0.08, 0.9));
    draw_circle(x + 34.0, y + 20.0, 3.2, Color::new(0.08, 0.05, 0.08, 0.9));
    draw_circle(x + 14.0, y + 26.0, 2.4, col);
    draw_circle(x + 40.0, y + 26.0, 2.4, col);
}

fn game_render(app: &App, pal: Palette) {
    let mut rng = ::rand::thread_rng();
    let sx = if app.shake > 0.0 { rng.gen_range(-6.0..6.0) * app.shake } else { 0.0 };
    let sy = if app.shake > 0.0 { rng.gen_range(-6.0..6.0) * app.shake } else { 0.0 };

    if app.flash > 0.0 {
        draw_rectangle(0.0, 0.0, W, H, pal.ko.mul_alpha((app.flash * 90.0) as u8));
    }

    for inv in &app.invaders {
        if !inv.alive {
            continue;
        }
        let (x, y) = invader_xy(inv.row, inv.col, app.form_x, app.descent);
        draw_invader(x + sx, y + sy, app.frame, pal.accent);
    }

    for s in &app.shards {
        if s.alive {
            draw_rectangle(s.x + sx, s.y + sy, 9.0, 9.0, pal.soft);
        }
    }

    let ship_y = H - 52.0 + sy;
    let x = app.ship_x + sx;
    draw_triangle(
        Vec2::new(x, ship_y - 16.0),
        Vec2::new(x - 22.0, ship_y + 12.0),
        Vec2::new(x + 22.0, ship_y + 12.0),
        pal.ok,
    );
    draw_rectangle(x - 22.0, ship_y + 10.0, 44.0, 6.0, pal.ok);
    draw_text("♡", x - 6.0, ship_y + 2.0, 18.0, pal.bg);

    for &(bx, by) in &app.bullets {
        draw_text("♡", bx + sx - 6.0, by + sy + 6.0, 16.0, pal.ok);
    }
    for &(bx, by) in &app.bombs {
        draw_circle(bx + sx, by + sy, 4.5, pal.ko);
        draw_circle(bx + sx, by + sy - 8.0, 2.5, pal.ko.mul_alpha(140));
    }

    for p in &app.parts {
        draw_rectangle(
            p.x + sx,
            p.y + sy,
            4.0,
            4.0,
            p.col.mul_alpha((p.life * 255.0).min(255.0) as u8),
        );
    }

    let hud = format!("score : {}", app.score);
    draw_text(&hud, 24.0, 34.0, 22.0, pal.text);
    let (_, name, _, _, _, _) = LEVELS[app.level];
    draw_text(&format!("niveau : {}", name), 200.0, 34.0, 22.0, pal.text);
    let hearts = "♡ ".repeat(app.lives.max(0) as usize);
    draw_text(&format!("vies : {}", hearts.trim_end()), W - 200.0, 34.0, 22.0, pal.ok);

    if !app.message.is_empty() {
        let mw = measure_text(&app.message, None, 24, 1.0).width;
        draw_text(&app.message, W / 2.0 - mw / 2.0, H - 20.0, 24.0, pal.accent);
    }
    if app.won {
        let paws = "🐾 ♡ ".repeat(6);
        let pw = measure_text(&paws, None, 28, 1.0).width;
        draw_text(&paws, W / 2.0 - pw / 2.0, H / 2.0, 28.0, pal.accent);
    }
    let help = if app.over {
        "[n] rejouer · esc menu · q quitter"
    } else {
        "←→ bouger · espace tirer · t thème · esc menu"
    };
    let hw = measure_text(help, None, 15, 1.0).width;
    draw_text(help, W / 2.0 - hw / 2.0, H - 44.0, 15.0, pal.dim);
}

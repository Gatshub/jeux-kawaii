//! Spacewar! (1962, PDP-1) — port macroquad, 2 joueurs, vectoriel rétro.
//! SPEC : spacewar/SPEC.md

use macroquad::prelude::*;
use ::rand::Rng;

// ---------------------------------------------------------------- constantes

const W: f32 = 1024.0;
const H: f32 = 768.0;
const CENTER: Vec2 = vec2(W / 2.0, H / 2.0);

const STAR_R: f32 = 20.0;
const STAR_KILL_R: f32 = 26.0;
const STAR_DANGER_R: f32 = 60.0;
const GRAVITY: f32 = 60.0;

const TURN_SPEED: f32 = 3.5;
const THRUST: f32 = 300.0;
const TORPEDO_SPEED: f32 = 420.0;
const TORP_MAX_SPEED: f32 = 600.0;
const TORP_LIFE: f32 = 3.0;
const TORP_MAX_PER_SHIP: usize = 4;
const TORP_FRIEND_DELAY: f32 = 0.2;
const TORP_RELOAD: f32 = 0.25; // entre deux tirs (documenté SPEC §2)
const FUEL_RESUME: f32 = FUEL_MAX * 0.2; // B3 : seuil de reprise de poussée

const FUEL_MAX: f32 = 6.0;
const FUEL_BURN: f32 = 1.0; // par seconde de poussée
const FUEL_REGEN: f32 = 0.35; // par seconde hors poussée

const HYPERRISK: f32 = 1.0 / 8.0;
const HYPER_COOLDOWN: f32 = 1.0;

const ROUNDS_TO_WIN: u32 = 5;
const COUNTDOWN_TIME: f32 = 3.0;

/// Couleurs jouables pour les vaisseaux : blanc + arc-en-ciel.
/// (nom, couleur) ; le nom est affiché dans sa propre couleur dans le menu.
const SHIP_COLORS: [(&str, Color); 8] = [
    ("Blanc", Color::new(1.0, 1.0, 1.0, 1.0)),
    ("Rouge", Color::new(1.0, 0.25, 0.25, 1.0)),
    ("Orange", Color::new(1.0, 0.55, 0.1, 1.0)),
    ("Jaune", Color::new(1.0, 0.9, 0.2, 1.0)),
    ("Vert", Color::new(0.3, 0.9, 0.35, 1.0)),
    ("Cyan", Color::new(0.25, 0.85, 0.95, 1.0)),
    ("Bleu", Color::new(0.35, 0.55, 1.0, 1.0)),
    ("Magenta", Color::new(0.95, 0.35, 0.9, 1.0)),
];

/// Remanence (phosphore) : alpha avec lequel l'image précédente est
/// gardée. 0 = désactivée. S'applique aux pixels tels quels : la traînée
/// garde la couleur des objets qui l'ont tracée.
const REMANENCE: [(&str, f32); 4] = [
    ("Off", 0.0),
    ("Faible", 0.35),
    ("Moyenne", 0.65),
    ("Forte", 0.85),
];

const WHITE: Color = Color::new(1.0, 1.0, 1.0, 1.0);
const DIM: Color = Color::new(0.667, 0.667, 0.667, 1.0);
const FLAME: Color = Color::new(1.0, 0.878, 0.690, 1.0);
const ACCENT: Color = Color::new(0.502, 0.753, 1.0, 1.0);

// ---------------------------------------------------------------------- états

#[derive(PartialEq, Clone, Copy)]
enum Phase {
    Menu,
    Countdown,
    Play,
    RoundEnd,
    MatchEnd,
}

// -------------------------------------------------------------------- vaisseau

struct Ship {
    pos: Vec2,
    vel: Vec2,
    angle: f32,
    alive: bool,
    fuel: f32,
    cooldown: f32,      // délai entre tirs
    hyper_ready: f32,   // temps restant avant hyperspace dispo
    is_left: bool,      // J1 (aiguille) ou J2 (fusée)
    implosion: Option<f32>,
    implosion_pos: Vec2, // B5 : position de départ de l'hyperspace
    thrust_lock: bool,  // B3 : poussée verrouillée à réservoir vide
    thrusting: bool,    // poussée active cette frame (rendu de la flamme)
    color: Color,       // couleur du trait (choisie au menu)
}

impl Ship {
    fn new(is_left: bool, color_idx: usize) -> Self {
        let (pos, angle) = if is_left {
            (vec2(200.0, H / 2.0), 0.0) // cap → droite
        } else {
            (vec2(W - 200.0, H / 2.0), std::f32::consts::PI) // cap ← gauche
        };
        Ship {
            pos,
            vel: vec2(0.0, 0.0),
            angle,
            alive: true,
            fuel: FUEL_MAX,
            cooldown: 0.0,
            hyper_ready: 0.0,
            is_left,
            implosion: None,
            implosion_pos: vec2(0.0, 0.0),
            thrust_lock: false,
            thrusting: false,
            color: SHIP_COLORS[color_idx % SHIP_COLORS.len()].1,
        }
    }

    /// Sommets du vaisseau en coordonnées locales (unités écran, cap = +x).
    fn shape(&self) -> Vec<Vec2> {
        if self.is_left {
            // Aiguille : effilée vers l'avant.
            vec![
                vec2(14.0, 0.0),
                vec2(-10.0, 7.0),
                vec2(-6.0, 0.0),
                vec2(-10.0, -7.0),
            ]
        } else {
            // Fusée/soucoupe : plus large, nez arrondi anguleux.
            vec![
                vec2(12.0, 0.0),
                vec2(2.0, 5.0),
                vec2(-8.0, 9.0),
                vec2(-11.0, 0.0),
                vec2(-8.0, -9.0),
                vec2(2.0, -5.0),
            ]
        }
    }

    fn draw_at(&self, dx: f32, dy: f32) {
        let (x, y) = (self.pos.x + dx, self.pos.y + dy);
        if let Some(t) = self.implosion {
            // B5 : implosion à la position de départ, visible même mort.
            let (ix, iy) = (self.implosion_pos.x + dx, self.implosion_pos.y + dy);
            draw_circle_lines(ix, iy, t * 30.0, 2.0, ACCENT);
        }
        if !self.alive {
            return;
        }
        let pts = self.shape();
        for i in 0..pts.len() {
            let a = rot(pts[i], self.angle) + vec2(x, y);
            let b = rot(pts[(i + 1) % pts.len()], self.angle) + vec2(x, y);
            draw_line(a.x, a.y, b.x, b.y, 2.0, self.color);
        }
        if self.thrusting {
            // Flamme vacillante (rendu uniquement ; revue §4).
            let mut rng = ::rand::thread_rng();
            let len: f32 = rng.gen_range(9.0..16.0);
            let fwd = self.forward();
            let tail = vec2(x, y) - fwd * (14.0 + len);
            draw_line(x - fwd.x * 12.0, y - fwd.y * 12.0, tail.x, tail.y, 2.0, FLAME);
        }
    }


    fn forward(&self) -> Vec2 {
        vec2(self.angle.cos(), self.angle.sin())
    }
}

fn rot(v: Vec2, a: f32) -> Vec2 {
    vec2(v.x * a.cos() - v.y * a.sin(), v.x * a.sin() + v.y * a.cos())
}

// ------------------------------------------------------------------ torpilles

struct Torpedo {
    pos: Vec2,
    vel: Vec2,
    life: f32,
    owner: usize, // 0 = J1, 1 = J2
}

// ----------------------------------------------------------------- particules

struct Particle {
    pos: Vec2,
    vel: Vec2,
    life: f32,
}

// ---------------------------------------------------------------------- jeu

struct Game {
    phase: Phase,
    phase_t: f32,
    ships: [Ship; 2],
    torpedoes: Vec<Torpedo>,
    particles: Vec<Particle>,
    score: [u32; 2],
    solo: bool, // mode test : J2 est un vaisseau inerte (pas d'input)
    color_idx: [usize; 2], // choix de couleur dans SHIP_COLORS (menu)
    remanence: usize,      // intensité de remanence (index dans REMANENCE)
}

impl Game {
    fn new() -> Self {
        Game {
            phase: Phase::Menu,
            phase_t: 0.0,
            ships: [Ship::new(true, 0), Ship::new(false, 0)],
            torpedoes: Vec::new(),
            particles: Vec::new(),
            score: [0, 0],
            solo: false,
            color_idx: [0, 0],
            remanence: 0,
        }
    }

    fn reset_round(&mut self) {
        self.ships = [
            Ship::new(true, self.color_idx[0]),
            Ship::new(false, self.color_idx[1]),
        ];
        self.torpedoes.clear();
        self.particles.clear();
    }

    fn controls(&self, i: usize) -> (bool, bool, bool, bool, bool) {
        if i == 0 {
            (
                is_key_down(KeyCode::A),
                is_key_down(KeyCode::D),
                is_key_down(KeyCode::W),
                is_key_down(KeyCode::S),
                is_key_down(KeyCode::Q),
            )
        } else {
            (
                is_key_down(KeyCode::Left),
                is_key_down(KeyCode::Right),
                is_key_down(KeyCode::Up),
                is_key_down(KeyCode::Down),
                is_key_down(KeyCode::M),
            )
        }
    }


    fn update_play(&mut self, dt: f32) {
        for i in 0..2 {
            if self.solo && i == 1 {
                continue;
            }
            let (left, right, thrust, fire, hyper) = self.controls(i);
            let torp_count = self.torpedoes.iter().filter(|t| t.owner == i).count();
            let ship = &mut self.ships[i];

            if !ship.alive {
                continue;
            }
            ship.cooldown = (ship.cooldown - dt).max(0.0);
            ship.hyper_ready = (ship.hyper_ready - dt).max(0.0);

            if left {
                ship.angle -= TURN_SPEED * dt;
            }
            if right {
                ship.angle += TURN_SPEED * dt;
            }

            // Hystérésis : à réservoir vide, la poussée reste verrouillée
            // jusqu'à régénération de 20 % (SPEC §2 : « régénération partielle »).
            // Hystérésis (revue B3) : à réservoir vide la poussée reste
            // verrouillée jusqu'à FUEL_RESUME (20 %), même si on relâche puis
            // repousse la touche. Plus de reprise par à-coups.
            if thrust && !ship.thrust_lock && ship.fuel > 0.0 {
                ship.vel += ship.forward() * THRUST * dt;
                ship.fuel = (ship.fuel - FUEL_BURN * dt).max(0.0);
                ship.thrusting = true;
                if ship.fuel <= 0.0 {
                    ship.thrust_lock = true;
                }
            } else {
                if ship.fuel >= FUEL_RESUME {
                    ship.thrust_lock = false;
                }
                ship.fuel = (ship.fuel + FUEL_REGEN * dt).min(FUEL_MAX);
                ship.thrusting = false;
            }

            if fire && ship.cooldown <= 0.0 && torp_count < TORP_MAX_PER_SHIP {
                let v = ship.vel + ship.forward() * TORPEDO_SPEED;
                self.torpedoes.push(Torpedo {
                    pos: ship.pos + ship.forward() * 16.0,
                    vel: v,
                    life: TORP_LIFE,
                    owner: i,
                });
                ship.cooldown = TORP_RELOAD;
            }

            if hyper && ship.hyper_ready <= 0.0 {
                ship.implosion = Some(0.35);
                ship.implosion_pos = ship.pos;
                ship.hyper_ready = HYPER_COOLDOWN;
                let mut rng = ::rand::thread_rng();
                loop {
                    let p = vec2(rng.gen_range(60.0..W - 60.0), rng.gen_range(60.0..H - 60.0));
                    if (p - CENTER).length() > STAR_DANGER_R + 40.0 {
                        ship.pos = p;
                        break;
                    }
                }
                ship.vel = vec2(0.0, 0.0); // choix de gameplay : réarrivée immobile (à valider)
                if rng.gen::<f32>() < HYPERRISK {
                    ship.alive = false;
                    explode(&mut self.particles, ship.pos, 30);
                }
            }
        }

        // Physique commune : gravité + intégration + enveloppement.
        // En solo, J2 est gelé : pas de gravité, l'étoile ne le tue pas.
        for i in 0..2 {
            if self.solo && i == 1 {
                continue;
            }
            let ship = &mut self.ships[i];
            if !ship.alive {
                continue;
            }
            let d = CENTER - ship.pos;
            let dist = d.length().max(1.0);
            ship.vel += d / dist * GRAVITY * dt;
            ship.pos += ship.vel * dt;
            wrap(&mut ship.pos);
            if (ship.pos - CENTER).length() < STAR_KILL_R {
                ship.alive = false;
                explode(&mut self.particles, ship.pos, 40);
            }
        }

        // Torpilles.
        let mut keep: Vec<bool> = Vec::with_capacity(self.torpedoes.len());
        for t in &mut self.torpedoes {
            let d = CENTER - t.pos;
            let dist = d.length().max(1.0);
            t.vel += d / dist * GRAVITY * dt;
            if t.vel.length() > TORP_MAX_SPEED {
                t.vel = t.vel.normalize() * TORP_MAX_SPEED;
            }
            t.pos += t.vel * dt;
            wrap(&mut t.pos);
            t.life -= dt;
            let age = TORP_LIFE - t.life;
            let mut alive = t.life > 0.0 && (t.pos - CENTER).length() >= STAR_KILL_R;
            if alive {
                for (i, ship) in self.ships.iter_mut().enumerate() {
                    if ship.alive
                        && !(self.solo && i == 1)
                        && !(i == t.owner && age < TORP_FRIEND_DELAY)
                        && (ship.pos - t.pos).length() < 12.0
                    {
                        ship.alive = false;
                        explode(&mut self.particles, ship.pos, 40);
                        alive = false;
                        break;
                    }
                }
            }
            keep.push(alive);
        }
        let mut ki = 0;
        self.torpedoes.retain(|_| {
            let k = keep[ki];
            ki += 1;
            k
        });

        // Particules.
        self.particles.retain_mut(|p| {
            p.pos += p.vel * dt;
            wrap(&mut p.pos);
            p.life -= dt;
            p.life > 0.0
        });

        // Implosions.
        for ship in &mut self.ships {
            if let Some(t) = &mut ship.implosion {
                *t -= dt;
                if *t <= 0.0 {
                    ship.implosion = None;
                }
            }
        }

        self.settle_round();
    }

    /// À appeler après la physique : compte le point si la manche est finie.
    fn settle_round(&mut self) {
        if self.phase != Phase::Play {
            return;
        }
        let dead = [!self.ships[0].alive, !self.ships[1].alive];
        if !dead[0] && !dead[1] {
            return;
        }
        if !dead[0] && dead[1] {
            self.score[0] += 1;
        } else if dead[0] && !dead[1] {
            self.score[1] += 1;
        }
        // Double mort : personne ne marque.
        self.phase = if self.score[0] >= ROUNDS_TO_WIN || self.score[1] >= ROUNDS_TO_WIN {
            Phase::MatchEnd
        } else {
            Phase::RoundEnd
        };
        self.phase_t = 0.0;
    }

    fn draw_world(&self) {
        // Étoile + zone de danger.
        draw_circle(CENTER.x, CENTER.y, STAR_R, DIM);
        for i in 0..16 {
            let a0 = i as f32 / 16.0 * std::f32::consts::TAU;
            let a1 = a0 + std::f32::consts::TAU / 32.0;
            draw_line(
                CENTER.x + a0.cos() * STAR_DANGER_R,
                CENTER.y + a0.sin() * STAR_DANGER_R,
                CENTER.x + a1.cos() * STAR_DANGER_R,
                CENTER.y + a1.sin() * STAR_DANGER_R,
                1.0,
                DIM,
            );
        }
        // R1 : les copies de raccord (±W / ±H) ne sont dessinées que pour
        // les objets proches du bord QUI LES APPELLE (la copie +W n'existe
        // que si l'objet frôle le bord gauche) — sinon elles apparaîtraient
        // dans les bandes noires quand la fenêtre n'est pas exactement 4:3.
        // Marge 50 px > rayon vaisseau + flamme + rayon torpille.
        let wants = |x: f32, y: f32, dx: f32, dy: f32| {
            (dx > 0.0 && x < 50.0)
                || (dx < 0.0 && x > W - 50.0)
                || (dy > 0.0 && y < 50.0)
                || (dy < 0.0 && y > H - 50.0)
        };
        for (dx, dy) in [(0.0, 0.0), (W, 0.0), (-W, 0.0), (0.0, H), (0.0, -H)] {
            let mut anything = dx == 0.0 && dy == 0.0;
            for t in &self.torpedoes {
                if !anything && wants(t.pos.x, t.pos.y, dx, dy) {
                    anything = true;
                }
            }
            for ship in &self.ships {
                if !anything && ship.alive && wants(ship.pos.x, ship.pos.y, dx, dy) {
                    anything = true;
                }
            }
            if !anything {
                continue;
            }
            for t in &self.torpedoes {
                // Torpille de la couleur de son propriétaire.
                let col = self.ships[t.owner].color;
                let dir = t.vel.normalize_or_zero() * 5.0;
                draw_line(t.pos.x - dir.x + dx, t.pos.y - dir.y + dy,
                          t.pos.x + dir.x + dx, t.pos.y + dir.y + dy, 2.0, col);
            }
            self.ships[0].draw_at(dx, dy);
            self.ships[1].draw_at(dx, dy);
        }
        for p in &self.particles {
            let a = p.life.min(1.0);
            draw_circle(p.pos.x, p.pos.y, 1.5, Color::new(1.0, 0.9, 0.75, a));
        }
    }

    fn draw_hud(&self, font: &Font) {
        // HUD en trois blocs : "J1 ◇ n" · icône étoile au trait · "m ◇ J2".
        // Le ◇ vient de la police ; l'étoile est vectorielle (⏣ n'existe
        // dans aucune police courante — revue §3).
        let left = format!("J1 ◇ {}", self.score[0]);
        let right = format!("{} ◇ J2", self.score[1]);
        let lw = measure_text(&left, Some(font), 22, 1.0).width;
        let rw = measure_text(&right, Some(font), 22, 1.0).width;
        let icon_w = 40.0;
        let gap = 40.0;
        let total = lw + gap + icon_w + gap + rw;
        let x0 = W / 2.0 - total / 2.0;
        draw_text_ex(&left, x0, 34.0, TextParams { font: Some(font), font_size: 22, color: WHITE, ..Default::default() });
        let icx = x0 + lw + gap + icon_w / 2.0;
        draw_circle_lines(icx, 27.0, 8.0, 1.5, WHITE);
        draw_circle(icx, 27.0, 2.5, DIM);
        draw_text_ex(&right, x0 + lw + gap + icon_w + gap, 34.0,
                     TextParams { font: Some(font), font_size: 22, color: WHITE, ..Default::default() });
        // Barres de carburant, 100 px, centrées sous chaque bloc score.
        let centers = [x0 + lw / 2.0, x0 + lw + gap + icon_w + gap + rw / 2.0];
        for (i, ship) in self.ships.iter().enumerate() {
            let x = centers[i] - 50.0;
            let fw = 100.0 * ship.fuel / FUEL_MAX;
            draw_rectangle_lines(x, 44.0, 100.0, 6.0, 1.0, DIM);
            if ship.alive && fw > 0.0 {
                draw_rectangle(x + 1.0, 45.0, fw - 2.0, 4.0, WHITE);
            }
        }
    }
}

fn explode(particles: &mut Vec<Particle>, pos: Vec2, n: usize) {
    let mut rng = ::rand::thread_rng();
    for _ in 0..n {
        let a: f32 = rng.gen_range(0.0..std::f32::consts::TAU);
        let s: f32 = rng.gen_range(40.0..260.0);
        particles.push(Particle {
            pos,
            vel: vec2(a.cos() * s, a.sin() * s),
            life: rng.gen_range(0.5..1.0),
        });
    }
}

fn wrap(p: &mut Vec2) {
    if p.x < 0.0 { p.x += W; } else if p.x > W { p.x -= W; }
    if p.y < 0.0 { p.y += H; } else if p.y > H { p.y -= H; }
}

fn centered(text: &str, y: f32, size: f32, color: Color, font: &Font) {
    let w = measure_text(text, Some(font), size as u16, 1.0).width;
    draw_text_ex(text, W / 2.0 - w / 2.0, y,
                 TextParams { font: Some(font), font_size: size as u16, color, ..Default::default() });
}

// ---------------------------------------------------------------------- main

#[macroquad::main(window_conf)]
async fn main() {
    // Police embarquée : Noto Sans Mono (couvre ◇, flèches, accents).
    // Licence : assets/NotoSansMono-LICENSE.txt.
    let font = load_ttf_font_from_bytes(include_bytes!(
        "../assets/NotoSansMono-Regular.ttf"
    ))
    .expect("police Noto Sans Mono introuvable");
    let mut game = Game::new();

    // Texture de remanence : le monde (vaisseaux, torpilles, particules,
    // étoile) est rendu dedans ; l'image précédente y est re-dessinée avec
    // un alpha de décroissance -> traînée de phosphore, couleur comprise.
    let rt = render_target(W as u32, H as u32);
    rt.texture.set_filter(FilterMode::Linear);
    let rt_cam = Camera2D {
        zoom: vec2(2.0 / W, 2.0 / H), // même convention y-bas que la caméra écran
        target: CENTER,
        offset: vec2(0.0, 0.0),
        render_target: Some(rt.clone()),
        ..Default::default()
    };

    loop {
        clear_background(BLACK);
        // Caméra : repère logique 1024×768 mis à l'échelle uniformément sur la
        // fenêtre réelle (centré, bandes noires). Règle le facteur d'échelle
        // HiDPI et le redimensionnement en une fois (revue §2).
        {
            let sw = screen_width();
            let sh = screen_height();
            let scale = (sw / W).min(sh / H);
            let cam = Camera2D {
                // zoom.y positif : le monde est en coordonnées écran
                // (y vers le bas), comme le rendu. Un zoom.y négatif
                // retournerait tout verticalement.
                zoom: vec2(2.0 * scale / sw, 2.0 * scale / sh),
                target: CENTER,
                offset: vec2(0.0, 0.0),
                ..Default::default()
            };
            set_camera(&cam);
        }
        // Rendu du monde dans la texture de remanence (hors menu).
        if game.phase != Phase::Menu {
            set_camera(&rt_cam);
            clear_background(BLACK);
            let decay = REMANENCE[game.remanence % REMANENCE.len()].1;
            if decay > 0.0 {
                // L'image précédente, atténuée : c'est la traînée.
                draw_texture_ex(
                    &rt.texture,
                    0.0,
                    0.0,
                    Color::new(1.0, 1.0, 1.0, decay),
                    DrawTextureParams { dest_size: Some(vec2(W, H)), ..Default::default() },
                );
            }
            game.draw_world();
            // Blit au centre de la fenêtre (repère logique de la caméra écran).
            set_default_camera();
            clear_background(BLACK);
            let sw = screen_width();
            let sh = screen_height();
            let scale = (sw / W).min(sh / H);
            let vx = (sw - W * scale) / 2.0;
            let vy = (sh - H * scale) / 2.0;
            draw_texture_ex(
                &rt.texture,
                vx,
                vy,
                WHITE,
                DrawTextureParams { dest_size: Some(vec2(W * scale, H * scale)), ..Default::default() },
            );
            // Réactiver la caméra logique pour l'UI (HUD, textes).
            let cam = Camera2D {
                zoom: vec2(2.0 * scale / sw, 2.0 * scale / sh),
                target: CENTER,
                offset: vec2(0.0, 0.0),
                ..Default::default()
            };
            set_camera(&cam);
        }
        match game.phase {
            Phase::Menu => {
                centered("SPACEWAR!", H * 0.25, 48.0, WHITE, &font);
                centered("Joueur 1 : A/D tourner · W poussée · S tir · Q hyperspace", H * 0.42, 20.0, DIM, &font);
                centered("Joueur 2 : ←/→ tourner · ↑ poussée · ↓ tir · M hyperspace", H * 0.42 + 28.0, 20.0, DIM, &font);
                centered("Premier à 5 victoires", H * 0.42 + 60.0, 20.0, DIM, &font);
                centered("[1] mode solo (test : J2 inerte)", H * 0.42 + 92.0, 20.0, DIM, &font);
                // Sélecteurs de couleur : le nom est affiché dans sa couleur.
                {
                    for (key, i) in [(KeyCode::C, 0usize), (KeyCode::V, 1usize)] {
                        let (name, col) = SHIP_COLORS[game.color_idx[i]];
                        let who = if i == 0 { "J1" } else { "J2" };
                        let label = format!("[{}] couleur {} : {}", if i == 0 { "C" } else { "V" }, who, name);
                        let w = measure_text(&label, Some(&font), 20, 1.0).width;
                        let yy = H * 0.42 + 124.0 + (i * 30) as f32;
                        // Préfixe en DIM, nom de la couleur dans sa couleur.
                        let prefix = format!("[{}] couleur {} : ", if i == 0 { "C" } else { "V" }, who);
                        let pw = measure_text(&prefix, Some(&font), 20, 1.0).width;
                        draw_text_ex(&label, W / 2.0 - w / 2.0, yy,
                                     TextParams { font: Some(&font), font_size: 20, color: DIM, ..Default::default() });
                        draw_text_ex(name, W / 2.0 - w / 2.0 + pw, yy,
                                     TextParams { font: Some(&font), font_size: 20, color: col, ..Default::default() });
                        if is_key_pressed(key) {
                            game.color_idx[i] = (game.color_idx[i] + 1) % SHIP_COLORS.len();
                        }
                    }
                }
                if (get_time() as f32 * 2.0).fract() < 0.5 {
                    centered("[ESPACE] commencer", H * 0.70, 24.0, WHITE, &font);
                }
                // Option remanence : intensité affichée en accent.
                {
                    let name = REMANENCE[game.remanence].0;
                    let label = format!("[X] remanence : {}", name);
                    let w = measure_text(&label, Some(&font), 20, 1.0).width;
                    let yy = H * 0.42 + 186.0;
                    draw_text_ex(&label, W / 2.0 - w / 2.0, yy,
                                 TextParams { font: Some(&font), font_size: 20, color: DIM, ..Default::default() });
                    let prefix_w = measure_text("[X] remanence : ", Some(&font), 20, 1.0).width;
                    draw_text_ex(name, W / 2.0 - w / 2.0 + prefix_w, yy,
                                 TextParams { font: Some(&font), font_size: 20, color: ACCENT, ..Default::default() });
                    if is_key_pressed(KeyCode::X) {
                        game.remanence = (game.remanence + 1) % REMANENCE.len();
                    }
                }
                centered("ÉCHAP : quitter", H - 20.0, 14.0, DIM, &font);
                if is_key_pressed(KeyCode::Escape) {
                    std::process::exit(0);
                }
                if is_key_pressed(KeyCode::Space) {
                    game.solo = false;
                    game.score = [0, 0];
                    game.reset_round();
                    game.phase = Phase::Countdown;
                    game.phase_t = 0.0;
                }
                if is_key_pressed(KeyCode::Key1) {
                    game.solo = true;
                    game.score = [0, 0];
                    game.reset_round();
                    game.phase = Phase::Countdown;
                    game.phase_t = 0.0;
                }
            }
            Phase::Countdown => {
                game.phase_t += get_frame_time();
                game.draw_world();
                game.draw_hud(&font);
                let remaining = COUNTDOWN_TIME - game.phase_t;
                if remaining <= 0.0 {
                    game.phase = Phase::Play;
                } else {
                    let n = remaining.ceil() as u32;
                    centered(&n.to_string(), CENTER.y + 30.0, 90.0, WHITE, &font);
                }
                if is_key_pressed(KeyCode::Escape) {
                    game.phase = Phase::Menu;
                }
            }
            Phase::Play => {
                game.update_play(get_frame_time().min(0.05));
                game.draw_world();
                game.draw_hud(&font);
                if is_key_pressed(KeyCode::Escape) {
                    game.phase = Phase::Menu;
                }
            }
            Phase::RoundEnd => {
                game.phase_t += get_frame_time();
                game.draw_world();
                game.draw_hud(&font);
                let alive = [game.ships[0].alive, game.ships[1].alive];
                let msg = if !alive[0] && !alive[1] {
                    "Étoile ! Personne ne marque".to_string()
                } else if !alive[0] {
                    "Le joueur 2 marque !".to_string()
                } else {
                    "Le joueur 1 marque !".to_string()
                };
                centered(&msg, H * 0.30, 32.0, WHITE, &font);
                if game.phase_t > 2.0 {
                    game.reset_round();
                    game.phase = Phase::Countdown;
                    game.phase_t = 0.0;
                }
            }
            Phase::MatchEnd => {
                game.draw_world();
                game.draw_hud(&font);
                let winner = if game.score[0] >= ROUNDS_TO_WIN { 1 } else { 2 };
                centered(&format!("JOUEUR {} GAGNE LA PARTIE !", winner), H * 0.30, 40.0, WHITE, &font);
                centered(&format!("score final : {} - {}", game.score[0], game.score[1]), H * 0.30 + 40.0, 24.0, DIM, &font);
                centered("[ESPACE] rejouer · [ÉCHAP] quitter", H * 0.30 + 80.0, 20.0, WHITE, &font);
                if is_key_pressed(KeyCode::Space) {
                    game.score = [0, 0];
                    game.reset_round();
                    game.phase = Phase::Countdown;
                    game.phase_t = 0.0;
                }
                if is_key_pressed(KeyCode::Escape) {
                    std::process::exit(0);
                }
            }
        }
        next_frame().await
    }
}


fn window_conf() -> Conf {
    // Wayland HiDPI : miniquad traite window_width comme des pixels physiques
    // puis divise par le facteur d'échelle du moniteur. On multiplie donc la
    // taille voulue (1024×768 logiques) par ce facteur pour que la fenêtre
    // s'ouvre à la bonne taille. La caméra (main) gère ensuite tout
    // redimensionnement.
    let scale = display_scale();
    Conf {
        window_title: "Spacewar!".to_owned(),
        window_width: (W * scale).round() as i32,
        window_height: (H * scale).round() as i32,
        window_resizable: true,
        ..Default::default()
    }
}

/// Facteur d'échelle du moniteur. Ordre : variable `SPACEWAR_SCALE`, puis
/// `hyprctl monitors -j` (moniteur ayant le focus), sinon 1.0.
fn display_scale() -> f32 {
    let valid = |v: f32| (0.5..=4.0).contains(&v);
    if let Some(v) = std::env::var("SPACEWAR_SCALE")
        .ok()
        .and_then(|s| s.parse::<f32>().ok())
        .filter(|v| valid(*v))
    {
        return v;
    }
    let out = match std::process::Command::new("hyprctl").args(["monitors", "-j"]).output() {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).into_owned(),
        _ => return 1.0,
    };
    parse_hypr_scale(&out).filter(|v| valid(*v)).unwrap_or(1.0)
}

/// Extrait le facteur d'échelle de la sortie JSON de `hyprctl monitors -j` :
/// celui du moniteur qui a le focus, sinon celui du premier moniteur.
fn parse_hypr_scale(json: &str) -> Option<f32> {
    // Un objet JSON par moniteur, chacun commençant par "id".
    let blocks: Vec<&str> = json.split("\"id\":").skip(1).collect();
    let block = blocks
        .iter()
        .find(|b| b.contains("\"focused\": true"))
        .or_else(|| blocks.first())?;
    let p = block.find("\"scale\":")?;
    let num: String = block[p + 8..]
        .chars()
        .skip_while(|c| c.is_whitespace())
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    num.parse::<f32>().ok()
}

#[cfg(test)]
mod tests {
    use super::parse_hypr_scale;

    #[test]
    fn scale_du_moniteur_focus() {
        let j = r#"[{"id": 0, "scale": 1.5, "focused": false},
                    {"id": 1, "scale": 2, "focused": true}]"#;
        assert_eq!(parse_hypr_scale(j), Some(2.0));
    }

    #[test]
    fn scale_premier_moniteur_sans_focus() {
        let j = r#"[{"id": 0, "scale": 1.6, "focused": false}]"#;
        assert_eq!(parse_hypr_scale(j), Some(1.6));
    }

    #[test]
    fn scale_absent_ou_json_vide() {
        assert_eq!(parse_hypr_scale("[]"), None);
        assert_eq!(parse_hypr_scale(r#"[{"id": 0}]"#), None);
    }
}

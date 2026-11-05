//! Short-lived effect sprites: wood chips, falling leaves, sparks, embers.
//! Simulated in pseudo-3D (ground position + height) so they arc and bounce.

use crate::assets::SpriteIds;
use crate::assets::palettes::OBJ_FX;
use crate::common::rng::Rng;
use crate::ppu::oam::ObjEntry;
use crate::render::camera::Camera;
use crate::render::sprites::DrawList;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Chip,
    Leaf,
    Spark,
    /// A glowing speck rising from a fire.
    Ember,
    /// A puff of dust kicked up by running feet.
    Dust,
}

#[derive(Clone, Debug)]
struct Particle {
    kind: Kind,
    /// Ground position (world px) and height above it.
    x: f32,
    y: f32,
    z: f32,
    vx: f32,
    vy: f32,
    vz: f32,
    life: i32,
    phase: f32,
    bounced: bool,
}

/// Frames before death during which a particle blinks, SNES-style.
const BLINK_FRAMES: i32 = 24;
const GRAVITY: f32 = 0.2;

pub struct Particles {
    list: Vec<Particle>,
    rng: Rng,
}

impl Particles {
    pub fn new(seed: u32) -> Self {
        Self { list: Vec::new(), rng: Rng::new(seed) }
    }

    fn rand(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (self.rng.next_u32() % 1000) as f32 / 1000.0 * (hi - lo)
    }

    fn spawn(&mut self, kind: Kind, x: f32, y: f32, z: f32, v: (f32, f32, f32), life: i32) {
        let phase = self.rand(0.0, 6.28);
        self.list.push(Particle { kind, x, y, z, vx: v.0, vy: v.1, vz: v.2, life, phase, bounced: false });
    }

    /// Wood chips bursting from an axe impact, thrown back toward the swinger.
    pub fn chips(&mut self, (x, y): (i32, i32), toward: (i32, i32), n: usize) {
        for _ in 0..n {
            let vx = toward.0 as f32 * self.rand(0.3, 1.2) + self.rand(-0.9, 0.9);
            let vy = toward.1 as f32 * self.rand(0.3, 1.0) + self.rand(-0.6, 0.6);
            let vz = self.rand(1.6, 2.8);
            self.spawn(Kind::Chip, x as f32, y as f32, 8.0, (vx, vy, vz), 70);
        }
    }

    /// Leaves shaken loose from a canopy. `canopy` is (x, y, w, h) in world
    /// px; `ground_y` is where they land.
    pub fn leaves(&mut self, canopy: (i32, i32, i32, i32), ground_y: i32, n: usize) {
        for _ in 0..n {
            let x = canopy.0 as f32 + self.rand(0.0, canopy.2 as f32);
            let top = canopy.1 as f32 + self.rand(0.0, canopy.3 as f32);
            let gy = ground_y as f32 + self.rand(-4.0, 10.0);
            let vz = -self.rand(0.25, 0.45);
            let drift = self.rand(-0.2, 0.2);
            self.spawn(Kind::Leaf, x, gy, gy - top, (drift, 0.0, vz), 220);
        }
    }

    pub fn sparks(&mut self, (x, y): (i32, i32), n: usize) {
        for _ in 0..n {
            let (dx, dy) = (self.rand(-3.0, 3.0), self.rand(-3.0, 3.0));
            self.spawn(Kind::Spark, x as f32 + dx, y as f32 + dy, 6.0, (0.0, 0.0, 0.0), 10);
        }
    }

    /// An ember drifting up from a fire at `(x, y)` (ground px).
    pub fn ember(&mut self, (x, y): (i32, i32)) {
        let (dx, vx) = (self.rand(-3.0, 3.0), self.rand(-0.15, 0.15));
        let life = self.rng.between(24, 40);
        self.spawn(Kind::Ember, x as f32 + dx, y as f32, 10.0, (vx, 0.0, 0.45), life);
    }

    /// A puff of dust behind running feet at `(x, y)` (ground px).
    pub fn dust(&mut self, (x, y): (i32, i32)) {
        let (dx, vx) = (self.rand(-2.0, 2.0), self.rand(-0.2, 0.2));
        self.spawn(Kind::Dust, x as f32 + dx, y as f32, 1.0, (vx, 0.0, 0.15), 14);
    }

    pub fn update(&mut self) {
        for p in &mut self.list {
            p.life -= 1;
            p.phase += 0.11;
            match p.kind {
                Kind::Chip => {
                    p.x += p.vx;
                    p.y += p.vy;
                    p.z += p.vz;
                    p.vz -= GRAVITY;
                    if p.z <= 0.0 {
                        p.z = 0.0;
                        if p.bounced {
                            p.vx = 0.0;
                            p.vy = 0.0;
                            p.vz = 0.0;
                        } else {
                            p.bounced = true;
                            p.vz = -p.vz * 0.35;
                            p.vx *= 0.5;
                            p.vy *= 0.5;
                        }
                    }
                }
                Kind::Leaf => {
                    if p.z > 0.0 {
                        // Flutter: sway sideways and slow down at the ends of each swing.
                        p.x += p.phase.sin() * 0.55 + p.vx;
                        p.z = (p.z + p.vz * (0.6 + 0.4 * p.phase.cos().abs())).max(0.0);
                    }
                }
                Kind::Spark => {}
                Kind::Dust => {
                    p.x += p.vx;
                    p.z += p.vz;
                }
                Kind::Ember => {
                    p.x += p.vx + p.phase.sin() * 0.2;
                    p.z += p.vz;
                }
            }
        }
        self.list.retain(|p| p.life > 0);
    }

    pub fn draw(&self, list: &mut DrawList, camera: &Camera, sprites: &SpriteIds, clock: u32) {
        for p in &self.list {
            if matches!(p.kind, Kind::Chip | Kind::Leaf) && p.life < BLINK_FRAMES && p.life % 4 < 2 {
                continue;
            }
            let (wx, wy) = (p.x as i32 - 4, (p.y - p.z) as i32 - 4);
            if !camera.sees(wx, wy, 8, 8) {
                continue;
            }
            let (sx, sy) = camera.to_screen(wx, wy);
            let (gfx, priority) = match p.kind {
                Kind::Chip => (sprites.chip[((clock / 4) as usize + (p.phase * 10.0) as usize) % 2 * (p.z > 0.0) as usize], 2),
                // High leaves fall in front of the canopy (BG2 high).
                Kind::Leaf => (sprites.leaf[(p.phase.sin() > 0.0) as usize], if p.z > 12.0 { 3 } else { 2 }),
                Kind::Spark => (sprites.spark[(p.life < 5) as usize], 3),
                Kind::Ember => (sprites.spark[1], 3),
                // Grows, then thins out.
                Kind::Dust => (sprites.dust[(p.life < 8) as usize], 2),
            };
            let flip = p.kind == Kind::Leaf && p.phase.cos() < 0.0;
            list.actor(p.y as i32, ObjEntry::new(sx, sy, gfx, OBJ_FX, priority).flipped(flip));
        }
    }
}

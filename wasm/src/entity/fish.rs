//! Fish swimming in the shallows, the sea along the shore and the river,
//! seen as dark shapes under the water. They idle about until a float lands
//! near one; it comes over to nibble, then bites, and is caught if the line
//! is struck in time. A fish that got away (or was frightened off) keeps
//! clear of the line for a while.

use crate::assets::SpriteIds;
use crate::assets::palettes::OBJ_SHADOW;
use crate::common::rng::Rng;
use crate::ppu::oam::ObjEntry;
use crate::render::camera::Camera;
use crate::render::sprites::DrawList;
use crate::world::{TILE_SIZE, Terrain, WorldMap};

const SPEED_SWIM: f32 = 0.3;
const SPEED_LURED: f32 = 0.45;
const SPEED_SPOOKED: f32 = 1.6;
/// How far a fish wanders at a time, and how close it must be to a float
/// to notice it (px).
const WANDER: i32 = 40;
pub const LURE_RADIUS: f32 = 56.0;
/// Frames a frightened fish darts off for, and then keeps clear of floats.
const SPOOKED_FRAMES: i32 = 40;
const WARY_FRAMES: i32 = 300;

#[derive(Clone, Copy, Debug, PartialEq)]
enum State {
    Swim,
    /// Swimming over to a float.
    Lured((f32, f32)),
    /// Darting away from a float.
    Spooked((f32, f32)),
}

#[derive(Clone, Debug)]
pub struct Fish {
    pub x: f32,
    pub y: f32,
    target: (f32, f32),
    state: State,
    timer: i32,
    wary: i32,
    facing_right: bool,
    anim: u32,
    rng: Rng,
}

/// Whether a fish can swim at the world pixel: in the shallows or the
/// river, or in deep water close to them (not out at sea).
pub fn swimmable(map: &WorldMap, (x, y): (i32, i32)) -> bool {
    let (cx, cy) = (x.div_euclid(TILE_SIZE), y.div_euclid(TILE_SIZE));
    if !(0..map.width()).contains(&cx) || !(0..map.height()).contains(&cy) {
        return false;
    }
    match map.terrain(cx, cy) {
        Terrain::ShallowWater | Terrain::River => true,
        Terrain::DeepWater => (-2..=2).any(|dy| (-2..=2).any(|dx| map.terrain(cx + dx, cy + dy) != Terrain::DeepWater)),
        _ => false,
    }
}

impl Fish {
    pub fn new((x, y): (i32, i32), seed: u32) -> Self {
        let mut rng = Rng::new(seed);
        let timer = rng.between(30, 200);
        let p = (x as f32, y as f32);
        Self { x: p.0, y: p.1, target: p, state: State::Swim, timer, wary: 0, facing_right: rng.chance(0.5), anim: 0, rng }
    }

    pub fn pos(&self) -> (i32, i32) {
        (self.x as i32, self.y as i32)
    }

    /// Whether it would come to a float at `at` (near enough and not wary).
    pub fn would_take(&self, at: (i32, i32)) -> bool {
        self.wary == 0 && !matches!(self.state, State::Spooked(_)) && self.dist(at) < LURE_RADIUS
    }

    pub fn lure(&mut self, at: (i32, i32)) {
        self.state = State::Lured((at.0 as f32, at.1 as f32));
    }

    /// Whether it has reached the float it was lured to.
    pub fn at_float(&self) -> bool {
        matches!(self.state, State::Lured((fx, fy)) if (self.x - fx).abs() < 3.0 && (self.y - fy).abs() < 3.0)
    }

    /// Frightened off (the hook came out, or it got away): darts away from
    /// the float and keeps clear of lines for a while.
    pub fn spook(&mut self, from: (i32, i32)) {
        let (dx, dy) = (self.x - from.0 as f32, self.y - from.1 as f32);
        let len = (dx * dx + dy * dy).sqrt().max(0.01);
        self.state = State::Spooked((dx / len, dy / len));
        self.timer = SPOOKED_FRAMES;
        self.wary = WARY_FRAMES;
    }

    fn dist(&self, (x, y): (i32, i32)) -> f32 {
        let (dx, dy) = (self.x - x as f32, self.y - y as f32);
        (dx * dx + dy * dy).sqrt()
    }

    pub fn update(&mut self, map: &WorldMap) {
        self.wary = (self.wary - 1).max(0);
        self.anim += 1;
        let (goal, speed) = match self.state {
            State::Swim => {
                self.timer -= 1;
                if self.timer <= 0 {
                    self.pick_target(map);
                }
                (self.target, SPEED_SWIM)
            }
            State::Lured(at) => (at, SPEED_LURED),
            State::Spooked((dx, dy)) => {
                self.timer -= 1;
                if self.timer <= 0 {
                    self.state = State::Swim;
                    self.target = (self.x, self.y);
                }
                ((self.x + dx * 8.0, self.y + dy * 8.0), SPEED_SPOOKED)
            }
        };
        let (dx, dy) = (goal.0 - self.x, goal.1 - self.y);
        let len = (dx * dx + dy * dy).sqrt();
        if len < 0.5 {
            return;
        }
        let step = speed.min(len);
        let (nx, ny) = (self.x + dx / len * step, self.y + dy / len * step);
        if swimmable(map, (nx as i32, ny as i32)) {
            self.x = nx;
            self.y = ny;
            if dx.abs() > 0.2 {
                self.facing_right = dx > 0.0;
            }
        } else if self.state == State::Swim {
            self.target = (self.x, self.y);
        }
    }

    /// Picks somewhere nearby to drift to, or rests a moment.
    fn pick_target(&mut self, map: &WorldMap) {
        self.timer = self.rng.between(60, 220);
        for _ in 0..8 {
            let p = (self.x as i32 + self.rng.between(-WANDER, WANDER), self.y as i32 + self.rng.between(-WANDER / 2, WANDER / 2));
            if swimmable(map, p) {
                self.target = (p.0 as f32, p.1 as f32);
                return;
            }
        }
    }

    /// A dark shape under the water, by day only (at night the water hides
    /// it; the float still bobs when one comes).
    pub fn draw(&self, list: &mut DrawList, camera: &Camera, sprites: &SpriteIds, night: bool) {
        if night {
            return;
        }
        let (wx, wy) = (self.x as i32 - 8, self.y as i32 - 4);
        if !camera.sees(wx, wy, 16, 8) {
            return;
        }
        let (sx, sy) = camera.to_screen(wx, wy);
        let swish = if matches!(self.state, State::Spooked(_)) { 4 } else { 12 };
        let frame = (self.anim / swish) as usize % sprites.fish_shadow.len();
        list.shadow(ObjEntry::new(sx, sy, sprites.fish_shadow[frame], OBJ_SHADOW, 2).flipped(self.facing_right));
    }
}

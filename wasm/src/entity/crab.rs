//! Beach crabs: ambient wildlife that wanders sideways and scuttles away from
//! the player.

use super::body::Body;
use crate::assets::SpriteIds;
use crate::assets::palettes::SPR_CRAB;
use crate::common::math::Rect;
use crate::common::rng::Rng;
use crate::ppu::oam::Oam;
use crate::render::camera::Camera;
use crate::render::sprites::push_16x16;
use crate::world::{Terrain, WorldMap};

const HITBOX: Rect = Rect::new(2, 8, 12, 6);
const SPEED_WANDER: i32 = 96;
const SPEED_FLEE: i32 = 400;
const FLEE_RADIUS: i32 = 40;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    Idle,
    Walk,
    Flee,
}

#[derive(Clone, Debug)]
pub struct Crab {
    pub body: Body,
    rng: Rng,
    state: State,
    timer: i32,
    dir: (i32, i32),
    anim: u32,
}

impl Crab {
    pub fn new(px: i32, py: i32, seed: u32) -> Self {
        let mut rng = Rng::new(seed);
        let timer = rng.between(10, 90);
        Self { body: Body::new(px, py, HITBOX), rng, state: State::Idle, timer, dir: (1, 0), anim: 0 }
    }

    pub fn update(&mut self, map: &WorldMap, player_center: (i32, i32)) {
        let (cx, cy) = self.center();
        let (dx, dy) = (cx - player_center.0, cy - player_center.1);
        if dx * dx + dy * dy < FLEE_RADIUS * FLEE_RADIUS && self.state != State::Flee {
            // Crabs bolt sideways, away from the threat.
            self.state = State::Flee;
            self.dir = (if dx >= 0 { 1 } else { -1 }, dy.signum() * self.rng.range(2) as i32);
            self.timer = 40;
        }

        self.timer -= 1;
        if self.timer <= 0 {
            self.next_state();
        }

        let speed = match self.state {
            State::Idle => 0,
            State::Walk => SPEED_WANDER,
            State::Flee => SPEED_FLEE,
        };
        if speed > 0 {
            // Crabs keep to dry sand.
            let blocked = |r: Rect| map.any_cell(r, |x, y| map.props(x, y).solid || map.terrain(x, y) != Terrain::Sand);
            let moved = self.body.move_by(self.dir.0 * speed, self.dir.1 * speed / 2, 0, &blocked);
            if !moved {
                self.dir.0 = -self.dir.0;
            }
            self.anim += 1;
        }
    }

    fn next_state(&mut self) {
        if self.state == State::Idle {
            self.state = State::Walk;
            self.dir = (if self.rng.chance(0.5) { 1 } else { -1 }, self.rng.between(-1, 1));
            self.timer = self.rng.between(30, 100);
        } else {
            self.state = State::Idle;
            self.timer = self.rng.between(40, 160);
        }
    }

    pub fn center(&self) -> (i32, i32) {
        let (x, y) = self.body.px();
        (x + 8, y + 10)
    }

    pub fn sort_y(&self) -> i32 {
        self.body.px().1
    }

    pub fn draw(&self, oam: &mut Oam, camera: &Camera, sprites: &SpriteIds) {
        let (wx, wy) = self.body.px();
        if !camera.sees(wx, wy, 16, 16) {
            return;
        }
        let (sx, sy) = camera.to_screen(wx, wy);
        let frame = ((self.anim / 6) % 2) as usize;
        push_16x16(oam, sprites.crab[frame], sx, sy, SPR_CRAB);
    }
}

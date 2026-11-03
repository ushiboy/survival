//! Coconuts knocked out of palms: they drop from the crown, bounce, roll a
//! little and then wait on the ground to be picked up. Simulated in
//! pseudo-3D (ground position + height) like particles.

use crate::assets::SpriteIds;
use crate::assets::palettes::{OBJ_CRAB, OBJ_CRAB_NIGHT, OBJ_SHADOW};
use crate::ppu::oam::ObjEntry;
use crate::render::camera::Camera;
use crate::render::sprites::DrawList;
use crate::world::WorldMap;

const GRAVITY: f32 = 0.25;
/// Upward speed of each bounce after landing.
const BOUNCES: [f32; 2] = [1.5, 0.6];
const ROLL_FRICTION: f32 = 0.88;
/// Above this height the coconut is still among the fronds and drawn in
/// front of the canopy, where it hung.
const IN_CANOPY_Z: f32 = 18.0;
/// How close (px) the castaway's feet must come to pick it up.
const PICKUP_RANGE: f32 = 9.0;

#[derive(Clone, Debug)]
pub struct Coconut {
    /// Ground position (world px) and height above it.
    x: f32,
    y: f32,
    z: f32,
    vx: f32,
    vy: f32,
    vz: f32,
    bounces: usize,
    /// Rolls this way once it hits the ground.
    roll: (f32, f32),
    anim: u32,
}

impl Coconut {
    /// Drops from where it hung (`from`, world px) to land on the ground at
    /// `to`, then rolls along `roll` (px per frame).
    pub fn falling(from: (f32, f32), to: (f32, f32), roll: (f32, f32)) -> Self {
        let z = (to.1 - from.1).max(0.0);
        let frames = (2.0 * z / GRAVITY).sqrt().max(1.0);
        Self { x: from.0, y: to.1, z, vx: (to.0 - from.0) / frames, vy: 0.0, vz: 0.0, bounces: 0, roll, anim: 0 }
    }

    pub fn update(&mut self, map: &WorldMap) {
        self.anim = self.anim.wrapping_add(1);
        if self.resting() {
            return;
        }
        let (nx, ny) = (self.x + self.vx, self.y + self.vy);
        // Stop against anything solid instead of rolling into it.
        if map.props_at_px(nx as i32, ny as i32).solid && self.z <= 0.0 {
            self.vx = 0.0;
            self.vy = 0.0;
        } else {
            self.x = nx;
            self.y = ny;
        }
        if self.z > 0.0 || self.vz > 0.0 {
            self.z += self.vz;
            self.vz -= GRAVITY;
            if self.z <= 0.0 {
                self.z = 0.0;
                match BOUNCES.get(self.bounces) {
                    Some(&up) => {
                        self.vz = up;
                        (self.vx, self.vy) = self.roll;
                        self.roll = (self.roll.0 * 0.5, self.roll.1 * 0.5);
                    }
                    None => self.vz = 0.0,
                }
                self.bounces += 1;
            }
        } else {
            self.vx *= ROLL_FRICTION;
            self.vy *= ROLL_FRICTION;
            if self.vx.abs() + self.vy.abs() < 0.05 {
                self.vx = 0.0;
                self.vy = 0.0;
            }
        }
    }

    /// On the ground and still: ready to be picked up.
    pub fn resting(&self) -> bool {
        self.z <= 0.0 && self.vz <= 0.0 && self.vx == 0.0 && self.vy == 0.0 && self.bounces > BOUNCES.len()
    }

    /// Whether feet at `(fx, fy)` are close enough to pick it up.
    pub fn reachable_from(&self, (fx, fy): (i32, i32)) -> bool {
        let (dx, dy) = (fx as f32 - self.x, fy as f32 - self.y);
        self.resting() && dx * dx + dy * dy < PICKUP_RANGE * PICKUP_RANGE
    }

    pub fn ground(&self) -> (i32, i32) {
        (self.x as i32, self.y as i32)
    }

    pub fn draw(&self, list: &mut DrawList, camera: &Camera, sprites: &SpriteIds, night: bool) {
        let (wx, wy) = (self.x as i32 - 4, (self.y - self.z) as i32 - 7);
        if !camera.sees(wx, wy, 8, 8 + self.z as i32) {
            return;
        }
        let (sx, sy) = camera.to_screen(wx, wy);
        let airborne = self.z > 0.0 || self.vx != 0.0 || self.vy != 0.0;
        let frame = if airborne { (self.anim / 4 % 2) as usize } else { 0 };
        let palette = if night { OBJ_CRAB_NIGHT } else { OBJ_CRAB };
        let priority = if self.z > IN_CANOPY_Z { 3 } else { 2 };
        let flip = airborne && self.anim / 8 % 2 == 1;
        list.actor(self.y as i32, ObjEntry::new(sx, sy, sprites.coconut[frame], palette, priority).flipped(flip));
        if !night {
            let (gx, gy) = camera.to_screen(self.x as i32 - 4, self.y as i32 - 4);
            list.shadow(ObjEntry::new(gx, gy, sprites.shadow_coconut, OBJ_SHADOW, 2));
        }
    }
}

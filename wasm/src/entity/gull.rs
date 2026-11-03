//! Seagulls gliding over the beach. They fly above everything (sprite
//! priority 3, over the palm canopies) and cast a translucent shadow.

use crate::assets::SpriteIds;
use crate::assets::palettes::{OBJ_GULL, OBJ_SHADOW};
use crate::ppu::oam::ObjEntry;
use crate::render::camera::Camera;
use crate::render::sprites::DrawList;

const SKY_PRIORITY: u8 = 3;
const SHADOW_PRIORITY: u8 = 2;
/// Apparent height: the shadow lands this far below/right of the bird.
const ALTITUDE: (i32, i32) = (14, 40);

#[derive(Clone, Debug)]
pub struct Gull {
    /// Centre of the circling path (world px).
    centre: (f32, f32),
    radius: (f32, f32),
    speed: f32,
    phase: f32,
    t: u32,
    pos: (f32, f32),
}

impl Gull {
    pub fn new(centre: (f32, f32), radius: (f32, f32), speed: f32, phase: f32) -> Self {
        let mut g = Self { centre, radius, speed, phase, t: 0, pos: centre };
        g.update();
        g
    }

    pub fn update(&mut self) {
        self.t = self.t.wrapping_add(1);
        let a = self.phase + self.t as f32 * self.speed;
        self.pos = (self.centre.0 + a.cos() * self.radius.0, self.centre.1 + a.sin() * self.radius.1);
    }

    pub fn draw(&self, list: &mut DrawList, camera: &Camera, sprites: &SpriteIds, shadows: bool) {
        let (wx, wy) = (self.pos.0 as i32 - 8, self.pos.1 as i32 - 8);
        // Flap for a while, then glide.
        let flapping = (self.t / 90) % 3 != 2;
        let frame = if flapping { ((self.t / 8) % 2) as usize } else { 0 };
        let (sx, sy) = camera.to_screen(wx, wy);
        if camera.sees(wx, wy, 16, 16) {
            list.sky(ObjEntry::new(sx, sy, sprites.gull[frame], OBJ_GULL, SKY_PRIORITY));
        }
        let (shx, shy) = (wx + ALTITUDE.0, wy + ALTITUDE.1);
        if shadows && camera.sees(shx, shy, 16, 8) {
            let (x, y) = camera.to_screen(shx, shy);
            list.shadow(ObjEntry::new(x, y, sprites.shadow_gull, OBJ_SHADOW, SHADOW_PRIORITY));
        }
    }
}

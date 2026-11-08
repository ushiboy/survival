//! Campfires lit with the fire drill: a ring of stones round burning wood
//! that keeps going for a day, sending up embers, and lights the dark.

use crate::assets::SpriteIds;
use crate::assets::palettes::OBJ_CRAB;
use crate::ppu::oam::ObjEntry;
use crate::render::camera::Camera;
use crate::render::sprites::DrawList;

/// How long a fire burns (in-game minutes), and for how much of the end of
/// that it has died down to a small flame.
pub const BURN_MINUTES: u32 = 24 * 60;
const DYING_MINUTES: u32 = 2 * 60;
/// Frames per flame animation step.
const FLAME_FRAME: u32 = 5;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Campfire {
    /// Where it sits on the ground (world px, centre of the ring).
    pub x: i32,
    pub y: i32,
    /// When it was lit (in-game minutes since the start).
    lit_at: u32,
}

impl Campfire {
    pub fn new((x, y): (i32, i32), now: u32) -> Self {
        Self { x, y, lit_at: now }
    }

    pub fn burnt_out(&self, now: u32) -> bool {
        now >= self.lit_at + BURN_MINUTES
    }

    fn dying(&self, now: u32) -> bool {
        now + DYING_MINUTES >= self.lit_at + BURN_MINUTES
    }

    /// Where the flames are, for light and embers.
    pub fn flame_point(&self) -> (i32, i32) {
        (self.x, self.y - 8)
    }

    pub fn within(&self, (px, py): (i32, i32), r: i32) -> bool {
        let (dx, dy) = (px - self.x, py - self.y);
        dx * dx + dy * dy <= r * r
    }

    /// Stones and wood stay unlit by the night tint (palette 1), as the
    /// fire lights them itself.
    pub fn draw(&self, list: &mut DrawList, camera: &Camera, sprites: &SpriteIds, clock: u32, now: u32) {
        let (wx, wy) = (self.x - 8, self.y - 20);
        if !camera.sees(wx, wy, 16, 24) {
            return;
        }
        let (sx, sy) = camera.to_screen(wx, wy);
        let flames = if self.dying(now) { &sprites.fire.small } else { &sprites.fire.flames };
        let frame = (clock / FLAME_FRAME) as usize % flames.len();
        list.actor(self.y, ObjEntry::new(sx, sy, flames[frame], OBJ_CRAB, 2));
        list.actor(self.y, ObjEntry::new(sx, sy + 16, sprites.fire.ring, OBJ_CRAB, 2));
    }
}

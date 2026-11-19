//! Signal fires: hearths of stones bedded in earth with timber stacked on
//! them, set on open ground. Lit with the fire drill (a branch and a palm
//! frond burnt on them), they send up a column of smoke for a day, for a
//! passing plane to see. Too smoky to cook over, and no light to
//! see by: they're for being seen.

use crate::assets::SpriteIds;
use crate::assets::palettes::{OBJ_CRAB, OBJ_FX, OBJ_WATER_FX};
use crate::ppu::oam::ObjEntry;
use crate::render::camera::Camera;
use crate::render::sprites::DrawList;
use crate::world::TILE_SIZE;

/// How long a signal fire smokes once lit (in-game minutes).
pub const BURN_MINUTES: u32 = 24 * 60;
/// Frames per flame animation step.
const FLAME_FRAME: u32 = 5;
/// Puffs of smoke in the column at once, how long each takes to rise to
/// the top (frames) and how high that is (px).
const PUFFS: u32 = 12;
const PUFF_RISE: u32 = 360;
const SMOKE_HEIGHT: f32 = 150.0;

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SignalFire {
    /// The cell it stands in.
    pub cell: (i32, i32),
    /// When it was lit (in-game minutes since the start), while it burns.
    pub lit_at: Option<u32>,
}

impl SignalFire {
    /// One just set down, unlit.
    pub fn new(cell: (i32, i32)) -> Self {
        Self { cell, lit_at: None }
    }

    pub fn burning(&self) -> bool {
        self.lit_at.is_some()
    }

    /// Whether it has had its day of burning.
    pub fn burnt_out(&self, now: u32) -> bool {
        self.lit_at.is_some_and(|lit| now >= lit + BURN_MINUTES)
    }

    /// Where it stands (world px): the middle of its cell's foot.
    fn foot(&self) -> (i32, i32) {
        (self.cell.0 * TILE_SIZE + TILE_SIZE / 2, self.cell.1 * TILE_SIZE + TILE_SIZE - 1)
    }

    /// The hearth (cold, or lit with small flames on top of the stack), and
    /// while it burns the column of smoke rising and drifting off it.
    pub fn draw(&self, list: &mut DrawList, camera: &Camera, sprites: &SpriteIds, clock: u32) {
        let s = &sprites.signal_fire;
        let (fx, fy) = self.foot();
        let (wx, wy) = (fx - 16, fy - 28);
        if camera.sees(wx, wy, 32, 32) {
            let (sx, sy) = camera.to_screen(wx, wy);
            let hearth = if self.burning() { s.lit } else { s.hearth };
            list.actor(fy, ObjEntry::new(sx, sy, hearth, OBJ_FX, 2));
            if self.burning() {
                // The fire smoulders under the fronds on top of the stack.
                let flames = &sprites.fire.small;
                let frame = (clock / FLAME_FRAME) as usize % flames.len();
                list.actor(fy + 1, ObjEntry::new(sx + 8, sy - 4, flames[frame], OBJ_CRAB, 2));
            }
        }
        if !self.burning() {
            return;
        }
        // Puffs rise one after another, billowing out as they go and
        // leaning off with the wind, thinning away at the top.
        for k in 0..PUFFS {
            let age = (clock + k * PUFF_RISE / PUFFS) % PUFF_RISE;
            let t = age as f32 / PUFF_RISE as f32;
            let (frame, size) = match t {
                t if t < 0.1 => (0, 16),
                t if t < 0.28 => (1, 16),
                t if t < 0.78 => (2, 32),
                // Flickering out at the very top.
                t if t > 0.92 && (clock + k).is_multiple_of(2) => continue,
                _ => (3, 32),
            };
            let rise = t.powf(0.8) * SMOKE_HEIGHT;
            let lean = t * t * 34.0 + (t * 9.0 + k as f32 * 1.7).sin() * 3.0 * t;
            let (px, py) = (fx + lean as i32 - size / 2, fy - 22 - size / 2 - rise as i32);
            if !camera.sees(px, py, size, size) {
                continue;
            }
            let (sx, sy) = camera.to_screen(px, py);
            // Over the trees' crowns, as smoke rises above them.
            list.actor(fy - 2 - k as i32, ObjEntry::new(sx, sy, s.smoke[frame], OBJ_WATER_FX, 3));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_signal_fire_burns_for_a_day_once_lit() {
        let mut fire = SignalFire::new((3, 4));
        assert!(!fire.burning());
        fire.lit_at = Some(100);
        assert!(fire.burning());
        assert!(!fire.burnt_out(100 + BURN_MINUTES - 1));
        assert!(fire.burnt_out(100 + BURN_MINUTES));
    }
}

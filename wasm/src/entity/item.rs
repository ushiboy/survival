//! Things lying on the ground (or on their way there): coconuts knocked out
//! of palms, timber from felled trees. They drop or are tossed, bounce, roll
//! a little and then wait to be picked up. Simulated in pseudo-3D (ground
//! position + height) like particles.

use crate::assets::SpriteIds;
use crate::assets::palettes::{OBJ_CRAB, OBJ_CRAB_NIGHT, OBJ_FX, OBJ_GULL, OBJ_SHADOW};
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
/// How close (px) an item must be to the castaway's feet or reach point to
/// be picked up.
const PICKUP_RANGE: f32 = 13.0;

pub use crate::items::ItemKind;

#[derive(Clone, Debug)]
pub struct Item {
    pub kind: ItemKind,
    /// How many lie here together (a stack put down whole).
    pub count: u32,
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

impl Item {
    /// Already lying still on the ground at `at` (world px), e.g. placed when
    /// the island is made.
    pub fn lying(kind: ItemKind, at: (f32, f32)) -> Self {
        Self { kind, count: 1, x: at.0, y: at.1, z: 0.0, vx: 0.0, vy: 0.0, vz: 0.0, bounces: BOUNCES.len() + 1, roll: (0.0, 0.0), anim: 0 }
    }

    /// Drops from `from` (world px, as seen on screen) to land on the ground
    /// at `to`, then rolls along `roll` (px per frame).
    pub fn falling(kind: ItemKind, from: (f32, f32), to: (f32, f32), roll: (f32, f32)) -> Self {
        let z = (to.1 - from.1).max(0.0);
        let frames = (2.0 * z / GRAVITY).sqrt().max(1.0);
        Self { kind, count: 1, x: from.0, y: to.1, z, vx: (to.0 - from.0) / frames, vy: 0.0, vz: 0.0, bounces: 0, roll, anim: 0 }
    }

    /// The same, as a stack of `count`.
    pub fn with_count(self, count: u32) -> Self {
        Self { count, ..self }
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

    /// Squared distance from a world point, if it lies within pickup range
    /// of it and has come to rest.
    pub fn reach(&self, (px, py): (i32, i32)) -> Option<f32> {
        let (dx, dy) = (px as f32 - self.x, py as f32 - self.y);
        let d = dx * dx + dy * dy;
        (self.resting() && d < PICKUP_RANGE * PICKUP_RANGE).then_some(d)
    }

    pub fn ground(&self) -> (i32, i32) {
        (self.x as i32, self.y as i32)
    }

    pub fn draw(&self, list: &mut DrawList, camera: &Camera, sprites: &SpriteIds, night: bool) {
        let (w, gfx, shadow) = match self.kind {
            ItemKind::Coconut => (8, sprites.coconut, sprites.shadow_coconut),
            ItemKind::OpenedCoconut => (8, [sprites.opened_coconut; 2], sprites.shadow_coconut),
            ItemKind::Timber => (16, [sprites.timber; 2], sprites.shadow_player),
            ItemKind::StoneAxe => (16, [sprites.axe_side; 2], sprites.shadow_player),
            ItemKind::FireDrill => (16, [sprites.fire_drill; 2], sprites.shadow_player),
            ItemKind::WoodSpear => (32, [sprites.spear_side; 2], sprites.shadow_lying),
            ItemKind::Crab => (16, [sprites.crab_caught; 2], sprites.shadow_crab),
            ItemKind::GrilledCrab => (16, [sprites.crab_grilled; 2], sprites.shadow_crab),
            ItemKind::Leaf => (8, [sprites.leaves; 2], sprites.shadow_coconut),
            ItemKind::Bark => (8, [sprites.bark; 2], sprites.shadow_coconut),
            ItemKind::LauanTimber => (16, [sprites.lauan_timber; 2], sprites.shadow_player),
            ItemKind::PineTimber => (16, [sprites.pine_timber; 2], sprites.shadow_player),
            ItemKind::Bamboo => (32, [sprites.bamboo_lying; 2], sprites.shadow_lying),
            ItemKind::PackFrame => (16, [sprites.pack_frame; 2], sprites.shadow_player),
            ItemKind::BarkContainer | ItemKind::BoiledWater => (8, [sprites.bark_pot; 2], sprites.shadow_coconut),
            ItemKind::Branch => (16, [sprites.branch; 2], sprites.shadow_player),
            ItemKind::Stone => (8, [sprites.stone; 2], sprites.shadow_coconut),
            ItemKind::Vine => (8, [sprites.vine_coil; 2], sprites.shadow_coconut),
            ItemKind::Husk => (8, [sprites.husk; 2], sprites.shadow_coconut),
            ItemKind::Rope => (8, [sprites.rope; 2], sprites.shadow_coconut),
            ItemKind::StoneKnife => (8, [sprites.stone_knife; 2], sprites.shadow_coconut),
            ItemKind::PalmFrond => (16, [sprites.palm_frond; 2], sprites.shadow_player),
            ItemKind::Thatch => (16, [sprites.thatch; 2], sprites.shadow_player),
            ItemKind::Shelter => (16, [sprites.shelter_roll; 2], sprites.shadow_player),
            ItemKind::SeaShell => (8, [sprites.sea_shell; 2], sprites.shadow_coconut),
            ItemKind::ShellHook => (8, [sprites.shell_hook; 2], sprites.shadow_coconut),
            ItemKind::PineCone => (8, [sprites.pine_cone; 2], sprites.shadow_coconut),
            ItemKind::FishingRod => (32, [sprites.rod_side; 2], sprites.shadow_lying),
            ItemKind::Ladder => (32, [sprites.ladder_lying; 2], sprites.shadow_lying),
            ItemKind::Fish => (16, [sprites.fish; 2], sprites.shadow_crab),
            ItemKind::GrilledFish => (16, [sprites.grilled_fish; 2], sprites.shadow_crab),
            // Water and a burning torch never lie on the ground.
            ItemKind::RawWater | ItemKind::Torch => return,
        };
        let (wx, wy) = (self.x as i32 - w / 2, (self.y - self.z) as i32 - 7);
        if !camera.sees(wx, wy, w, 8 + self.z as i32) {
            return;
        }
        let (sx, sy) = camera.to_screen(wx, wy);
        let airborne = self.z > 0.0 || self.vx != 0.0 || self.vy != 0.0;
        let frame = if airborne { (self.anim / 4 % 2) as usize } else { 0 };
        let palette = match self.kind {
            ItemKind::StoneAxe | ItemKind::FireDrill | ItemKind::WoodSpear | ItemKind::Vine | ItemKind::Leaf | ItemKind::BarkContainer | ItemKind::BoiledWater | ItemKind::PackFrame | ItemKind::PalmFrond | ItemKind::Thatch | ItemKind::Shelter | ItemKind::StoneKnife | ItemKind::FishingRod | ItemKind::Fish | ItemKind::Ladder | ItemKind::Bamboo => OBJ_FX,
            ItemKind::LauanTimber => OBJ_GULL,
            _ if night => OBJ_CRAB_NIGHT,
            _ => OBJ_CRAB,
        };
        let priority = if self.z > IN_CANOPY_Z { 3 } else { 2 };
        let flip = airborne && self.anim / 8 % 2 == 1;
        list.actor(self.y as i32, ObjEntry::new(sx, sy, gfx[frame], palette, priority).flipped(flip));
        if !night {
            let (gx, gy) = camera.to_screen(self.x as i32 - w / 2, self.y as i32 - 4);
            list.shadow(ObjEntry::new(gx, gy, shadow, OBJ_SHADOW, 2));
        }
    }
}

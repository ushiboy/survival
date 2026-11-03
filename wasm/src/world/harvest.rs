//! Resource gathering on the map: what a tool strike does to the cell it hits.

use std::collections::HashMap;

use super::map::{TILE_SIZE, WorldMap};
use super::terrain::{Feature, PalmPart, StumpPart};
use crate::common::rng::hash_unit;

/// Axe hits needed to fell a palm.
pub const PALM_HP: u8 = 3;
/// Wood gained from a felled palm.
pub const PALM_WOOD: u32 = 3;
/// Most coconuts a palm carries (they don't grow back yet).
pub const PALM_COCONUTS: u8 = 3;
/// Shakes it takes to work one coconut loose: this many, plus up to
/// `SHAKES_JITTER` more depending on the tree and coconut.
const SHAKES_MIN: u8 = 3;
const SHAKES_JITTER: u8 = 2;

/// What an axe strike connected with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Strike {
    /// Hit nothing solid.
    Miss,
    /// Hit something the axe can't cut (stone, stump, ...).
    Hard { kind: Feature },
    /// Hit foliage that doesn't yield (bushes).
    Brush,
    /// Damaged a palm; `anchor` is its top-left cell.
    TreeHit { anchor: (i32, i32), hp_left: u8 },
    /// Felled a palm; the listed cells changed and need redrawing. The
    /// coconuts it still carried (`fruit`) come down with it.
    TreeFelled { anchor: (i32, i32), changed: Vec<(i32, i32)>, wood: u32, fruit: u8 },
}

/// What shaking a trunk did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Shaken {
    /// Nothing there to shake.
    Miss,
    /// The palm swayed but nothing came loose (yet, or ever: `fruit_left`).
    Rustle { anchor: (i32, i32), fruit_left: u8 },
    /// A coconut fell: `slot` is the one that dropped (see the palm art);
    /// the crown cell `changed` needs redrawing.
    Drop { anchor: (i32, i32), slot: u8, changed: (i32, i32) },
}

/// Damage and shaking state of trees being worked.
#[derive(Default, Debug)]
pub struct Harvest {
    damage: HashMap<(i32, i32), u8>,
    shakes: HashMap<(i32, i32), u8>,
}

/// The palm whose trunk base is at the world pixel, as (anchor, part).
fn trunk_at(map: &WorldMap, (px, py): (i32, i32)) -> Option<((i32, i32), PalmPart)> {
    let (x, y) = (px.div_euclid(TILE_SIZE), py.div_euclid(TILE_SIZE));
    match map.feature(x, y) {
        Feature::Palm(p) if p.is_trunk_base() => Some((p.anchor(x, y), p)),
        _ => None,
    }
}

/// Whether there's a trunk at the world pixel to take hold of and shake.
pub fn can_shake(map: &WorldMap, point: (i32, i32)) -> bool {
    trunk_at(map, point).is_some()
}

impl Harvest {
    /// Resolves an axe strike at a world pixel.
    pub fn axe(&mut self, map: &mut WorldMap, (px, py): (i32, i32)) -> Strike {
        if let Some((anchor, p)) = trunk_at(map, (px, py)) {
            // Only the trunk base can be reached and cut.
            return self.chop_palm(map, anchor, p.flip);
        }
        let (x, y) = (px.div_euclid(TILE_SIZE), py.div_euclid(TILE_SIZE));
        match map.feature(x, y) {
            Feature::Bush => Strike::Brush,
            f @ (Feature::Rock | Feature::Driftwood | Feature::Stump(_)) => Strike::Hard { kind: f },
            _ => Strike::Miss,
        }
    }

    fn chop_palm(&mut self, map: &mut WorldMap, anchor: (i32, i32), flip: bool) -> Strike {
        let hits = self.damage.entry(anchor).or_insert(0);
        *hits += 1;
        if *hits < PALM_HP {
            return Strike::TreeHit { anchor, hp_left: PALM_HP - *hits };
        }
        self.damage.remove(&anchor);
        self.shakes.remove(&anchor);
        let fruit = map.fruit(anchor);
        map.set_fruit(anchor, 0);
        let mut changed = Vec::new();
        for dy in 0..PalmPart::H {
            for dx in 0..PalmPart::W {
                let (x, y) = (anchor.0 + dx, anchor.1 + dy);
                let part = PalmPart { dx: dx as u8, dy: dy as u8, flip };
                let f = if part.is_trunk_base() { Feature::Stump(StumpPart { flip }) } else { Feature::None };
                map.set_feature(x, y, f);
                changed.push((x, y));
            }
        }
        Strike::TreeFelled { anchor, changed, wood: PALM_WOOD, fruit }
    }

    /// Resolves one shake of the trunk at a world pixel. Every few shakes a
    /// coconut works loose, until the palm has none left.
    pub fn shake(&mut self, map: &mut WorldMap, point: (i32, i32)) -> Shaken {
        let Some((anchor, _)) = trunk_at(map, point) else {
            return Shaken::Miss;
        };
        let fruit = map.fruit(anchor);
        if fruit == 0 {
            return Shaken::Rustle { anchor, fruit_left: 0 };
        }
        let shakes = self.shakes.entry(anchor).or_insert(0);
        *shakes += 1;
        let jitter = (hash_unit(anchor.0, anchor.1 * 4 + fruit as i32, 0x5EA7) * (SHAKES_JITTER + 1) as f32) as u8;
        if *shakes < SHAKES_MIN + jitter.min(SHAKES_JITTER) {
            return Shaken::Rustle { anchor, fruit_left: fruit };
        }
        self.shakes.remove(&anchor);
        map.set_fruit(anchor, fruit - 1);
        Shaken::Drop { anchor, slot: fruit - 1, changed: (anchor.0 + PalmPart::W / 2, anchor.1 + 1) }
    }
}

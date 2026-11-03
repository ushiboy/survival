//! Resource gathering on the map: what a tool strike does to the cell it hits.

use std::collections::HashMap;

use super::map::{TILE_SIZE, WorldMap};
use super::terrain::{Feature, PalmPart, StumpPart};

/// Axe hits needed to fell a palm.
pub const PALM_HP: u8 = 3;
/// Wood gained from a felled palm.
pub const PALM_WOOD: u32 = 3;

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
    /// Felled a palm; the listed cells changed and need redrawing.
    TreeFelled { anchor: (i32, i32), changed: Vec<(i32, i32)>, wood: u32 },
}

/// Damage state of trees being cut.
#[derive(Default, Debug)]
pub struct Harvest {
    damage: HashMap<(i32, i32), u8>,
}

impl Harvest {
    /// Resolves an axe strike at a world pixel.
    pub fn axe(&mut self, map: &mut WorldMap, (px, py): (i32, i32)) -> Strike {
        let (x, y) = (px.div_euclid(TILE_SIZE), py.div_euclid(TILE_SIZE));
        match map.feature(x, y) {
            // Only the trunk base can be reached and cut.
            Feature::Palm(p) if p.dy as i32 == PalmPart::H - 1 => self.chop_palm(map, (x - p.dx as i32, y - p.dy as i32), p.flip),
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
        let mut changed = Vec::new();
        for dy in 0..PalmPart::H {
            for dx in 0..PalmPart::W {
                let (x, y) = (anchor.0 + dx, anchor.1 + dy);
                let f = if dy == PalmPart::H - 1 { Feature::Stump(StumpPart { dx: dx as u8, flip }) } else { Feature::None };
                map.set_feature(x, y, f);
                changed.push((x, y));
            }
        }
        Strike::TreeFelled { anchor, changed, wood: PALM_WOOD }
    }
}

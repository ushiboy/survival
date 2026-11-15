//! Resource gathering on the map: what a tool strike does to the cell it hits.

use std::collections::HashMap;

use super::map::{TILE_SIZE, WorldMap};
use super::terrain::{BambooPart, BroadleafPart, ClayPart, DriftwoodPart, Feature, PalmPart, PinePart, StumpPart, patch_anchor, patch_cells};
use crate::items::ItemKind;
use crate::common::rng::hash_unit;

/// Axe hits needed to fell a palm.
pub const PALM_HP: u8 = 3;
/// Pieces of timber a felled palm breaks into.
pub const PALM_WOOD: u32 = 3;
/// Axe hits needed to fell a broadleaf tree (hardwood), and the lauan
/// timber it breaks into.
pub const BROADLEAF_HP: u8 = 5;
pub const BROADLEAF_WOOD: u32 = 3;
/// Axe hits needed to fell a pine (softwood: more than a palm, fewer than
/// hardwood), and the pine timber it breaks into (long lengths, so fewer).
pub const PINE_HP: u8 = 4;
pub const PINE_WOOD: u32 = 2;
/// Axe hits needed to fell a bamboo culm (hollow and thin: fewer than a
/// palm); it gives a single length.
pub const BAMBOO_HP: u8 = 2;
/// Most cones a pine carries (they don't grow back yet).
pub const PINE_CONES: u8 = 3;
/// Axe hits needed to split a driftwood log, and the branches it breaks
/// into (at least, and up to one more).
pub const DRIFTWOOD_HP: u8 = 2;
pub const DRIFTWOOD_BRANCHES: u32 = 2;
/// Strips of bark a broadleaf tree has to give (they don't grow back).
pub const BARK_STRIPS: u8 = 3;
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
    /// Cracked a driftwood log; `anchor` is its left cell.
    LogHit { anchor: (i32, i32) },
    /// Split a driftwood log into branches; its cells (now bare sand) need
    /// redrawing.
    LogSplit { anchor: (i32, i32), changed: Vec<(i32, i32)> },
    /// Damaged a palm; `anchor` is its top-left cell.
    TreeHit { anchor: (i32, i32), hp_left: u8 },
    /// Felled a tree; the listed cells changed and need redrawing. It falls
    /// apart into `wood` lengths of `timber`. What a palm still carried
    /// comes down with it: its coconuts (`fruit`) and fronds off its crown
    /// (`fronds`). A broadleaf's bark not yet stripped (`bark` strips) can
    /// be peeled off the fallen trunk. A pine's cones still on it (`cones`)
    /// come down too.
    TreeFelled { anchor: (i32, i32), changed: Vec<(i32, i32)>, timber: ItemKind, wood: u32, fruit: u8, fronds: bool, bark: u8, cones: u8 },
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
    /// A cone fell from the pine at `anchor`.
    Cone { anchor: (i32, i32) },
}

/// Damage and shaking state of trees being worked.
#[derive(Default, Debug)]
pub struct Harvest {
    damage: HashMap<(i32, i32), u8>,
    shakes: HashMap<(i32, i32), u8>,
    /// The day each bush was last picked; it grows new leaves overnight.
    picked: HashMap<(i32, i32), u32>,
}

/// What `Harvest` keeps, as lists (for saving).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct HarvestState {
    pub damage: Vec<((i32, i32), u8)>,
    pub shakes: Vec<((i32, i32), u8)>,
    pub picked: Vec<((i32, i32), u32)>,
}

impl Harvest {
    pub fn state(&self) -> HarvestState {
        fn list<V: Copy>(m: &HashMap<(i32, i32), V>) -> Vec<((i32, i32), V)> {
            let mut v: Vec<_> = m.iter().map(|(&k, &n)| (k, n)).collect();
            v.sort_by_key(|&(k, _)| k);
            v
        }
        HarvestState { damage: list(&self.damage), shakes: list(&self.shakes), picked: list(&self.picked) }
    }

    pub fn from_state(s: &HarvestState) -> Self {
        Self {
            damage: s.damage.iter().copied().collect(),
            shakes: s.shakes.iter().copied().collect(),
            picked: s.picked.iter().copied().collect(),
        }
    }
}

/// The palm whose trunk base is at the world pixel, as (anchor, part).
fn trunk_at(map: &WorldMap, (px, py): (i32, i32)) -> Option<((i32, i32), PalmPart)> {
    let (x, y) = (px.div_euclid(TILE_SIZE), py.div_euclid(TILE_SIZE));
    match map.feature(x, y) {
        Feature::Palm(p) if p.is_trunk_base() => Some((p.anchor(x, y), p)),
        _ => None,
    }
}

/// Whether there's a vine at the world pixel to pull off.
pub fn can_pull_vine(map: &WorldMap, (px, py): (i32, i32)) -> bool {
    map.feature(px.div_euclid(TILE_SIZE), py.div_euclid(TILE_SIZE)) == Feature::Vine
}

/// Pulls the vine off a climbing plant at the world pixel, leaving its bare
/// stake. Returns the cell, which needs redrawing.
pub fn pull_vine(map: &mut WorldMap, point: (i32, i32)) -> Option<(i32, i32)> {
    if !can_pull_vine(map, point) {
        return None;
    }
    let cell = (point.0.div_euclid(TILE_SIZE), point.1.div_euclid(TILE_SIZE));
    map.set_feature(cell.0, cell.1, Feature::VineStake);
    Some(cell)
}

/// Lumps of clay a bamboo spade lifts out of a clay bed (once a day).
pub const CLAY_PER_DIG: u32 = 2;
/// Handfuls of soil a digging stick loosens out of bare earth each time.
pub const SOIL_PER_DIG: u32 = 1;

/// What lies at a cell to dig: bare earth, or a clay bed (dug today or not).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Diggable {
    Earth,
    Clay { dug: bool },
}

/// What can be dug at the cell.
pub fn diggable(map: &WorldMap, (x, y): (i32, i32)) -> Option<Diggable> {
    match map.feature(x, y) {
        Feature::BareEarth(_) => Some(Diggable::Earth),
        Feature::ClayBed(c) => Some(Diggable::Clay { dug: c.dug }),
        _ => None,
    }
}

/// Lifts clay out of the bed at the cell, leaving the whole bed dug until
/// the morning. Returns its cells (to redraw), or `None` if there's no bed
/// there or it's been dug today.
pub fn dig_clay(map: &mut WorldMap, (x, y): (i32, i32)) -> Option<[(i32, i32); 4]> {
    let Feature::ClayBed(c) = map.feature(x, y) else { return None };
    if c.dug {
        return None;
    }
    let cells = patch_cells(patch_anchor(x, y, c.part));
    for (part, (cx, cy)) in cells.into_iter().enumerate() {
        map.set_feature(cx, cy, Feature::ClayBed(ClayPart { part: part as u8, dug: true }));
    }
    Some(cells)
}

/// A new morning: the lake's water has seeped back into every clay bed dug
/// since, softening it again. Returns the cells changed.
pub fn refill_clay_beds(map: &mut WorldMap) -> Vec<(i32, i32)> {
    let mut changed = Vec::new();
    for y in 0..map.height() {
        for x in 0..map.width() {
            if let Feature::ClayBed(c) = map.feature(x, y)
                && c.dug
            {
                map.set_feature(x, y, Feature::ClayBed(ClayPart { dug: false, ..c }));
                changed.push((x, y));
            }
        }
    }
    changed
}

/// Leaves a bush gives each time it's picked.
pub const BUSH_LEAVES: u32 = 5;

/// Whether there's a bush at the world pixel to pick leaves from (it may
/// have been stripped already today).
pub fn is_bush(map: &WorldMap, (px, py): (i32, i32)) -> bool {
    map.feature(px.div_euclid(TILE_SIZE), py.div_euclid(TILE_SIZE)) == Feature::Bush
}

/// The broadleaf tree whose trunk base is at the world pixel, as (anchor,
/// part).
fn broadleaf_at(map: &WorldMap, (px, py): (i32, i32)) -> Option<((i32, i32), BroadleafPart)> {
    let (x, y) = (px.div_euclid(TILE_SIZE), py.div_euclid(TILE_SIZE));
    match map.feature(x, y) {
        Feature::Broadleaf(p) if p.is_trunk_base() => Some((p.anchor(x, y), p)),
        _ => None,
    }
}

/// The pine whose trunk base is at the world pixel, as (anchor, part).
fn pine_at(map: &WorldMap, (px, py): (i32, i32)) -> Option<((i32, i32), PinePart)> {
    let (x, y) = (px.div_euclid(TILE_SIZE), py.div_euclid(TILE_SIZE));
    match map.feature(x, y) {
        Feature::Pine(p) if p.is_trunk_base() => Some((p.anchor(x, y), p)),
        _ => None,
    }
}

/// The bamboo culm whose foot is at the world pixel, as (anchor, part).
/// Its anchor is the top-left of a palm-sized box (3 cells wide) centred
/// on the culm, so what falls from it lands around it as from a tree.
fn bamboo_at(map: &WorldMap, (px, py): (i32, i32)) -> Option<((i32, i32), BambooPart)> {
    let (x, y) = (px.div_euclid(TILE_SIZE), py.div_euclid(TILE_SIZE));
    match map.feature(x, y) {
        Feature::Bamboo(p) if p.is_foot() => {
            let (ax, ay) = p.anchor(x, y);
            Some(((ax - 1, ay), p))
        }
        _ => None,
    }
}

/// Whether there's a bamboo culm's foot at the world pixel.
pub fn is_bamboo(map: &WorldMap, point: (i32, i32)) -> bool {
    bamboo_at(map, point).is_some()
}

/// Whether there's a broadleaf trunk at the world pixel to strip bark from
/// (it may have none left).
pub fn is_broadleaf_trunk(map: &WorldMap, point: (i32, i32)) -> bool {
    broadleaf_at(map, point).is_some()
}

/// Whether there's a broadleaf trunk at the world pixel with bark still
/// on it.
pub fn has_bark(map: &WorldMap, point: (i32, i32)) -> bool {
    broadleaf_at(map, point).is_some_and(|(anchor, _)| map.bark(anchor) > 0)
}

/// Strips a piece of bark off the broadleaf trunk at the world pixel: the
/// trunk-base cell, which needs redrawing, or `None` if there's no trunk
/// or no bark left on it.
pub fn strip_bark(map: &mut WorldMap, point: (i32, i32)) -> Option<(i32, i32)> {
    let (anchor, _) = broadleaf_at(map, point)?;
    let left = map.bark(anchor);
    if left == 0 {
        return None;
    }
    map.set_bark(anchor, left - 1);
    Some((anchor.0 + BroadleafPart::W / 2, anchor.1 + BroadleafPart::H - 1))
}

/// Whether there's a trunk at the world pixel to take hold of and shake (a
/// palm's or a pine's).
pub fn can_shake(map: &WorldMap, point: (i32, i32)) -> bool {
    trunk_at(map, point).is_some() || pine_at(map, point).is_some()
}

impl Harvest {
    /// Picks leaves from the bush at the world pixel on `day`: the cell
    /// picked, or `None` if there's no bush or it was stripped today.
    pub fn pick_leaves(&mut self, map: &WorldMap, point: (i32, i32), day: u32) -> Option<(i32, i32)> {
        if !is_bush(map, point) {
            return None;
        }
        let cell = (point.0.div_euclid(TILE_SIZE), point.1.div_euclid(TILE_SIZE));
        if self.picked.get(&cell) == Some(&day) {
            return None;
        }
        self.picked.insert(cell, day);
        Some(cell)
    }

    /// Resolves an axe strike at a world pixel.
    pub fn axe(&mut self, map: &mut WorldMap, (px, py): (i32, i32)) -> Strike {
        if let Some((anchor, p)) = trunk_at(map, (px, py)) {
            // Only the trunk base can be reached and cut.
            return self.chop_palm(map, anchor, p.flip);
        }
        if let Some((anchor, p)) = broadleaf_at(map, (px, py)) {
            return self.chop_broadleaf(map, anchor, p.flip);
        }
        if let Some((anchor, p)) = pine_at(map, (px, py)) {
            return self.chop_pine(map, anchor, p.flip);
        }
        if let Some((anchor, p)) = bamboo_at(map, (px, py)) {
            return self.chop_bamboo(map, anchor, p.flip);
        }
        let (x, y) = (px.div_euclid(TILE_SIZE), py.div_euclid(TILE_SIZE));
        match map.feature(x, y) {
            Feature::Driftwood(p) => self.split_log(map, p.anchor(x, y)),
            Feature::Bush | Feature::Vine => Strike::Brush,
            f @ (Feature::Rock | Feature::Stump(_) | Feature::BroadleafStump(_) | Feature::PineStump(_) | Feature::BambooStump(_)) => Strike::Hard { kind: f },
            _ => Strike::Miss,
        }
    }

    fn split_log(&mut self, map: &mut WorldMap, anchor: (i32, i32)) -> Strike {
        let hits = self.damage.entry(anchor).or_insert(0);
        *hits += 1;
        if *hits < DRIFTWOOD_HP {
            return Strike::LogHit { anchor };
        }
        self.damage.remove(&anchor);
        let changed: Vec<(i32, i32)> = (0..DriftwoodPart::W).map(|dx| (anchor.0 + dx, anchor.1)).collect();
        for &(x, y) in &changed {
            map.set_feature(x, y, Feature::None);
        }
        Strike::LogSplit { anchor, changed }
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
        Strike::TreeFelled { anchor, changed, timber: ItemKind::Timber, wood: PALM_WOOD, fruit, fronds: true, bark: 0, cones: 0 }
    }

    fn chop_broadleaf(&mut self, map: &mut WorldMap, anchor: (i32, i32), flip: bool) -> Strike {
        let hits = self.damage.entry(anchor).or_insert(0);
        *hits += 1;
        if *hits < BROADLEAF_HP {
            return Strike::TreeHit { anchor, hp_left: BROADLEAF_HP - *hits };
        }
        self.damage.remove(&anchor);
        // The bark left on the trunk comes away with it.
        let bark = map.bark(anchor);
        map.set_bark(anchor, 0);
        let mut changed = Vec::new();
        for dy in 0..BroadleafPart::H {
            for dx in 0..BroadleafPart::W {
                let (x, y) = (anchor.0 + dx, anchor.1 + dy);
                let part = BroadleafPart { dx: dx as u8, dy: dy as u8, flip };
                let f = if part.is_trunk_base() { Feature::BroadleafStump(StumpPart { flip }) } else { Feature::None };
                map.set_feature(x, y, f);
                changed.push((x, y));
            }
        }
        Strike::TreeFelled { anchor, changed, timber: ItemKind::LauanTimber, wood: BROADLEAF_WOOD, fruit: 0, fronds: false, bark, cones: 0 }
    }

    fn chop_pine(&mut self, map: &mut WorldMap, anchor: (i32, i32), flip: bool) -> Strike {
        let hits = self.damage.entry(anchor).or_insert(0);
        *hits += 1;
        if *hits < PINE_HP {
            return Strike::TreeHit { anchor, hp_left: PINE_HP - *hits };
        }
        self.damage.remove(&anchor);
        self.shakes.remove(&anchor);
        // The cones still on it come down with it.
        let cones = map.fruit(anchor);
        map.set_fruit(anchor, 0);
        let mut changed = Vec::new();
        for dy in 0..PinePart::H {
            for dx in 0..PinePart::W {
                let (x, y) = (anchor.0 + dx, anchor.1 + dy);
                let part = PinePart { dx: dx as u8, dy: dy as u8, flip };
                let f = if part.is_trunk_base() { Feature::PineStump(StumpPart { flip }) } else { Feature::None };
                map.set_feature(x, y, f);
                changed.push((x, y));
            }
        }
        Strike::TreeFelled { anchor, changed, timber: ItemKind::PineTimber, wood: PINE_WOOD, fruit: 0, fronds: false, bark: 0, cones }
    }

    fn chop_bamboo(&mut self, map: &mut WorldMap, anchor: (i32, i32), flip: bool) -> Strike {
        let hits = self.damage.entry(anchor).or_insert(0);
        *hits += 1;
        if *hits < BAMBOO_HP {
            return Strike::TreeHit { anchor, hp_left: BAMBOO_HP - *hits };
        }
        self.damage.remove(&anchor);
        let x = anchor.0 + 1;
        let mut changed = Vec::new();
        for dy in 0..BambooPart::H {
            let y = anchor.1 + dy;
            let f = if dy == BambooPart::H - 1 { Feature::BambooStump(StumpPart { flip }) } else { Feature::None };
            map.set_feature(x, y, f);
            changed.push((x, y));
        }
        Strike::TreeFelled { anchor, changed, timber: ItemKind::Bamboo, wood: 1, fruit: 0, fronds: false, bark: 0, cones: 0 }
    }

    /// Resolves one shake of the trunk at a world pixel. Every few shakes a
    /// coconut (or a pine's cone) works loose, until the tree has none left.
    pub fn shake(&mut self, map: &mut WorldMap, point: (i32, i32)) -> Shaken {
        let (anchor, pine) = match (trunk_at(map, point), pine_at(map, point)) {
            (Some((anchor, _)), _) => (anchor, false),
            (_, Some((anchor, _))) => (anchor, true),
            _ => return Shaken::Miss,
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
        if pine {
            return Shaken::Cone { anchor };
        }
        Shaken::Drop { anchor, slot: fruit - 1, changed: (anchor.0 + PalmPart::W / 2, anchor.1 + 1) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::terrain::Terrain;

    /// The top-left cell trees are planted at on the test map.
    const AT: (i32, i32) = (4, 4);

    fn ground() -> WorldMap {
        WorldMap::new(16, 16, Terrain::Grass)
    }

    /// The middle of a cell, in world px.
    fn px((x, y): (i32, i32)) -> (i32, i32) {
        (x * TILE_SIZE + TILE_SIZE / 2, y * TILE_SIZE + TILE_SIZE / 2)
    }

    /// Where a tree planted at `AT` is struck or shaken: its trunk base.
    fn trunk() -> (i32, i32) {
        px((AT.0 + PalmPart::W / 2, AT.1 + PalmPart::H - 1))
    }

    fn with_palm(coconuts: u8) -> WorldMap {
        let mut map = ground();
        for dy in 0..PalmPart::H {
            for dx in 0..PalmPart::W {
                map.set_feature(AT.0 + dx, AT.1 + dy, Feature::Palm(PalmPart { dx: dx as u8, dy: dy as u8, flip: false }));
            }
        }
        map.set_fruit(AT, coconuts);
        map
    }

    fn with_broadleaf(bark: u8) -> WorldMap {
        let mut map = ground();
        for dy in 0..BroadleafPart::H {
            for dx in 0..BroadleafPart::W {
                map.set_feature(AT.0 + dx, AT.1 + dy, Feature::Broadleaf(BroadleafPart { dx: dx as u8, dy: dy as u8, flip: false }));
            }
        }
        map.set_bark(AT, bark);
        map
    }

    fn with_pine(cones: u8) -> WorldMap {
        let mut map = ground();
        for dy in 0..PinePart::H {
            for dx in 0..PinePart::W {
                map.set_feature(AT.0 + dx, AT.1 + dy, Feature::Pine(PinePart { dx: dx as u8, dy: dy as u8, flip: false }));
            }
        }
        map.set_fruit(AT, cones);
        map
    }

    /// A bamboo culm standing in column `AT.0` from row `AT.1`; struck at
    /// its foot.
    fn with_bamboo() -> WorldMap {
        let mut map = ground();
        for dy in 0..BambooPart::H {
            map.set_feature(AT.0, AT.1 + dy, Feature::Bamboo(BambooPart { dy: dy as u8, v: 0, flip: false }));
        }
        map
    }

    fn bamboo_foot() -> (i32, i32) {
        (AT.0, AT.1 + BambooPart::H - 1)
    }

    fn with(cell: (i32, i32), f: Feature) -> WorldMap {
        let mut map = ground();
        map.set_feature(cell.0, cell.1, f);
        map
    }

    /// Strikes until something other than a hit comes back; the strikes
    /// it took and what came of the last.
    fn chop_down(h: &mut Harvest, map: &mut WorldMap, at: (i32, i32)) -> (u32, Strike) {
        for n in 1..=20 {
            match h.axe(map, at) {
                Strike::TreeHit { .. } | Strike::LogHit { .. } => continue,
                last => return (n, last),
            }
        }
        panic!("never came down");
    }

    #[test]
    fn a_palm_falls_after_a_few_axe_strikes_leaving_a_stump() {
        let mut map = with_palm(0);
        let (strikes, last) = chop_down(&mut Harvest::default(), &mut map, trunk());
        assert_eq!(strikes, PALM_HP as u32);
        assert!(matches!(last, Strike::TreeFelled { timber: ItemKind::Timber, .. }));
        let base = (AT.0 + PalmPart::W / 2, AT.1 + PalmPart::H - 1);
        assert!(matches!(map.feature(base.0, base.1), Feature::Stump(_)));
        assert_eq!(map.feature(AT.0, AT.1), Feature::None);
    }

    #[test]
    fn hardwood_takes_more_strikes_than_a_palm() {
        let (palm, _) = chop_down(&mut Harvest::default(), &mut with_palm(0), trunk());
        let (broadleaf, last) = chop_down(&mut Harvest::default(), &mut with_broadleaf(0), trunk());
        assert!(broadleaf > palm);
        assert!(matches!(last, Strike::TreeFelled { timber: ItemKind::LauanTimber, .. }));
    }

    #[test]
    fn a_felled_palm_brings_down_its_coconuts_and_fronds() {
        let mut map = with_palm(2);
        let (_, last) = chop_down(&mut Harvest::default(), &mut map, trunk());
        assert!(matches!(last, Strike::TreeFelled { fruit: 2, fronds: true, .. }));
        assert_eq!(map.fruit(AT), 0);
    }

    #[test]
    fn bark_left_on_a_felled_broadleaf_comes_with_it() {
        let mut map = with_broadleaf(2);
        let (_, last) = chop_down(&mut Harvest::default(), &mut map, trunk());
        assert!(matches!(last, Strike::TreeFelled { bark: 2, fronds: false, .. }));
        assert_eq!(map.bark(AT), 0);
    }

    #[test]
    fn only_the_trunk_base_can_be_cut() {
        let mut map = with_palm(0);
        assert_eq!(Harvest::default().axe(&mut map, px(AT)), Strike::Miss);
    }

    #[test]
    fn the_axe_cant_cut_rock_or_stumps_and_brushes_through_bushes() {
        let cell = (3, 3);
        for f in [Feature::Rock, Feature::Stump(StumpPart { flip: false }), Feature::BroadleafStump(StumpPart { flip: false })] {
            assert_eq!(Harvest::default().axe(&mut with(cell, f), px(cell)), Strike::Hard { kind: f });
        }
        for f in [Feature::Bush, Feature::Vine] {
            assert_eq!(Harvest::default().axe(&mut with(cell, f), px(cell)), Strike::Brush);
        }
        assert_eq!(Harvest::default().axe(&mut ground(), px(cell)), Strike::Miss);
    }

    #[test]
    fn driftwood_splits_into_branches_and_is_gone() {
        let mut map = ground();
        for dx in 0..DriftwoodPart::W {
            map.set_feature(3 + dx, 3, Feature::Driftwood(DriftwoodPart { dx: dx as u8, flip: false }));
        }
        let (strikes, last) = chop_down(&mut Harvest::default(), &mut map, px((4, 3)));
        assert_eq!(strikes, DRIFTWOOD_HP as u32);
        assert!(matches!(last, Strike::LogSplit { anchor: (3, 3), .. }));
        assert!((0..DriftwoodPart::W).all(|dx| map.feature(3 + dx, 3) == Feature::None));
    }

    #[test]
    fn strikes_on_a_tree_are_remembered_through_a_save() {
        let mut map = with_palm(0);
        let mut h = Harvest::default();
        h.axe(&mut map, trunk());
        let mut back = Harvest::from_state(&h.state());
        let (more, _) = chop_down(&mut back, &mut map, trunk());
        assert_eq!(more, PALM_HP as u32 - 1);
    }

    #[test]
    fn a_vine_can_be_pulled_off_once() {
        let cell = (3, 3);
        let mut map = with(cell, Feature::Vine);
        assert!(can_pull_vine(&map, px(cell)));
        assert_eq!(pull_vine(&mut map, px(cell)), Some(cell));
        assert_eq!(map.feature(cell.0, cell.1), Feature::VineStake);
        assert!(!can_pull_vine(&map, px(cell)));
        assert_eq!(pull_vine(&mut map, px(cell)), None);
    }

    #[test]
    fn bark_strips_off_until_there_is_none_left() {
        let mut map = with_broadleaf(BARK_STRIPS);
        for _ in 0..BARK_STRIPS {
            assert!(has_bark(&map, trunk()));
            assert!(strip_bark(&mut map, trunk()).is_some());
        }
        assert!(!has_bark(&map, trunk()));
        assert!(is_broadleaf_trunk(&map, trunk()));
        assert_eq!(strip_bark(&mut map, trunk()), None);
    }

    #[test]
    fn a_bush_is_picked_once_a_day() {
        let cell = (3, 3);
        let map = with(cell, Feature::Bush);
        let mut h = Harvest::default();
        assert_eq!(h.pick_leaves(&map, px(cell), 1), Some(cell));
        assert_eq!(h.pick_leaves(&map, px(cell), 1), None);
        assert_eq!(h.pick_leaves(&map, px(cell), 2), Some(cell));
        assert_eq!(h.pick_leaves(&ground(), px(cell), 3), None);
    }

    #[test]
    fn shaking_a_palm_works_a_coconut_loose_every_few_shakes() {
        let mut map = with_palm(2);
        let mut h = Harvest::default();
        let mut drops = 0;
        for _ in 0..30 {
            match h.shake(&mut map, trunk()) {
                Shaken::Drop { .. } => drops += 1,
                Shaken::Rustle { fruit_left, .. } => assert_eq!(fruit_left, map.fruit(AT)),
                Shaken::Miss | Shaken::Cone { .. } => panic!("a palm drops coconuts"),
            }
        }
        assert_eq!(drops, 2, "no more than it carried");
        assert_eq!(map.fruit(AT), 0);
    }

    #[test]
    fn no_coconut_falls_on_the_first_shake() {
        let mut map = with_palm(PALM_COCONUTS);
        assert!(matches!(Harvest::default().shake(&mut map, trunk()), Shaken::Rustle { .. }));
    }

    #[test]
    fn a_broadleaf_cant_be_shaken() {
        let mut map = with_broadleaf(0);
        assert!(!can_shake(&map, trunk()));
        assert_eq!(Harvest::default().shake(&mut map, trunk()), Shaken::Miss);
        assert!(can_shake(&with_palm(0), trunk()));
        assert!(can_shake(&with_pine(0), trunk()));
    }

    #[test]
    fn a_pine_falls_to_more_strikes_than_a_palm_and_fewer_than_hardwood_leaving_a_stump() {
        let (palm, _) = chop_down(&mut Harvest::default(), &mut with_palm(0), trunk());
        let (broadleaf, _) = chop_down(&mut Harvest::default(), &mut with_broadleaf(0), trunk());
        let mut map = with_pine(0);
        let (pine, last) = chop_down(&mut Harvest::default(), &mut map, trunk());
        assert!(palm < pine && pine < broadleaf, "palm {palm}, pine {pine}, broadleaf {broadleaf}");
        assert!(matches!(last, Strike::TreeFelled { timber: ItemKind::PineTimber, wood: PINE_WOOD, fronds: false, bark: 0, .. }));
        let base = (AT.0 + PinePart::W / 2, AT.1 + PinePart::H - 1);
        assert!(matches!(map.feature(base.0, base.1), Feature::PineStump(_)));
        assert_eq!(map.feature(AT.0, AT.1), Feature::None);
    }

    #[test]
    fn the_cones_left_on_a_felled_pine_come_down_with_it() {
        let mut map = with_pine(2);
        let (_, last) = chop_down(&mut Harvest::default(), &mut map, trunk());
        assert!(matches!(last, Strike::TreeFelled { cones: 2, fruit: 0, .. }));
        assert_eq!(map.fruit(AT), 0);
    }

    #[test]
    fn shaking_a_pine_works_a_cone_loose_every_few_shakes_until_none_are_left() {
        let mut map = with_pine(PINE_CONES);
        let mut h = Harvest::default();
        assert!(matches!(h.shake(&mut map, trunk()), Shaken::Rustle { .. }), "nothing on the first shake");
        let mut cones = 0;
        for _ in 0..40 {
            match h.shake(&mut map, trunk()) {
                Shaken::Cone { .. } => cones += 1,
                Shaken::Rustle { fruit_left, .. } => assert_eq!(fruit_left, map.fruit(AT)),
                other => panic!("a pine gives cones, not {other:?}"),
            }
        }
        assert_eq!(cones, PINE_CONES, "no more than it carried");
        assert_eq!(map.fruit(AT), 0);
    }

    #[test]
    fn the_axe_cant_cut_a_pine_stump() {
        let cell = (3, 3);
        let f = Feature::PineStump(StumpPart { flip: false });
        assert_eq!(Harvest::default().axe(&mut with(cell, f), px(cell)), Strike::Hard { kind: f });
    }

    #[test]
    fn a_bamboo_culm_falls_to_fewer_strikes_than_a_palm_giving_one_length_and_leaving_a_cut_stalk() {
        let (palm, _) = chop_down(&mut Harvest::default(), &mut with_palm(0), trunk());
        let mut map = with_bamboo();
        let (bamboo, last) = chop_down(&mut Harvest::default(), &mut map, px(bamboo_foot()));
        assert!(bamboo < palm);
        assert!(matches!(last, Strike::TreeFelled { timber: ItemKind::Bamboo, wood: 1, fruit: 0, fronds: false, bark: 0, cones: 0, .. }));
        let foot = bamboo_foot();
        assert!(matches!(map.feature(foot.0, foot.1), Feature::BambooStump(_)));
        assert!((0..BambooPart::H - 1).all(|dy| map.feature(AT.0, AT.1 + dy) == Feature::None));
    }

    #[test]
    fn only_the_foot_of_a_bamboo_culm_can_be_cut() {
        let mut map = with_bamboo();
        assert_eq!(Harvest::default().axe(&mut map, px(AT)), Strike::Miss);
    }

    #[test]
    fn the_axe_cant_cut_a_bamboo_stump() {
        let cell = (3, 3);
        let f = Feature::BambooStump(StumpPart { flip: false });
        assert_eq!(Harvest::default().axe(&mut with(cell, f), px(cell)), Strike::Hard { kind: f });
    }

    #[test]
    fn bamboo_cant_be_shaken() {
        assert!(!can_shake(&with_bamboo(), px(bamboo_foot())));
    }
}

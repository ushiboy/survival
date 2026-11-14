//! Driftwood the tide leaves on the west coast's beach, a morning now and
//! then. The first area's beaches get none, so its logs, once split, are
//! gone for good.

use super::terrain::DriftwoodPart;
use super::{Feature, Terrain, WorldMap};
use crate::common::rng::Rng;

/// How likely the tide is to leave a log on a given morning.
pub const WASH_UP_CHANCE: f32 = 1.0 / 3.0;

/// Where the west coast ends, row by row: cells west of a row's `x` lie
/// beyond the river.
pub struct WestCoast(Vec<Option<i32>>);

impl WestCoast {
    /// Each row's westmost river cell. Rows the river doesn't cross (by the
    /// falls, or south of where it meets the sea) take the nearest row's.
    pub fn of(map: &WorldMap) -> Self {
        let river: Vec<Option<i32>> = (0..map.height()).map(|y| (0..map.width()).find(|&x| map.terrain(x, y) == Terrain::River)).collect();
        let nearest = |y: usize| (0..river.len()).filter_map(|r| river[r].map(|x| (r.abs_diff(y), x))).min().map(|(_, x)| x);
        Self((0..river.len()).map(|y| river[y].or_else(|| nearest(y))).collect())
    }

    pub fn contains(&self, x: i32, y: i32) -> bool {
        usize::try_from(y).ok().and_then(|y| self.0.get(y).copied().flatten()).is_some_and(|rx| x < rx)
    }
}

/// The driftwood logs (counted once each) lying on the west coast.
pub fn logs_on_west_coast(map: &WorldMap) -> usize {
    let west = WestCoast::of(map);
    (0..map.height())
        .flat_map(|y| (0..map.width()).map(move |x| (x, y)))
        .filter(|&(x, y)| west.contains(x, y) && matches!(map.feature(x, y), Feature::Driftwood(p) if p.dx == 0))
        .count()
}

/// A new morning: perhaps the tide leaves a log on the west coast's sand,
/// unless as many lie there as `cap`. `keep_clear` says which cells must
/// stay free (where the castaway, a crab or a dropped item is). Returns the
/// cells changed.
pub fn wash_up(map: &mut WorldMap, rng: &mut Rng, cap: usize, keep_clear: &dyn Fn(i32, i32) -> bool) -> Vec<(i32, i32)> {
    if !rng.chance(WASH_UP_CHANCE) || logs_on_west_coast(map) >= cap {
        return Vec::new();
    }
    let west = WestCoast::of(map);
    let (w, h) = (map.width(), map.height());
    for _ in 0..400 {
        let (x, y) = (rng.between(0, w - 2), rng.between(0, h - 1));
        if fits(map, &west, x, y, keep_clear) {
            let flip = rng.chance(0.5);
            let cells: Vec<(i32, i32)> = (0..DriftwoodPart::W).map(|dx| (x + dx, y)).collect();
            for (dx, &(cx, cy)) in cells.iter().enumerate() {
                map.set_feature(cx, cy, Feature::Driftwood(DriftwoodPart { dx: dx as u8, flip }));
            }
            return cells;
        }
    }
    Vec::new()
}

/// Whether a log (its left cell at `(x, y)`) can lie here: on open sand of
/// the west coast near the sea, off the riverside, with ground to walk
/// around it on every side (so it never blocks a narrow beach).
fn fits(map: &WorldMap, west: &WestCoast, x: i32, y: i32, keep_clear: &dyn Fn(i32, i32) -> bool) -> bool {
    let log = |cx: i32, cy: i32| cy == y && (x..x + DriftwoodPart::W).contains(&cx);
    let open_sand = |cx: i32, cy: i32| {
        map.terrain(cx, cy) == Terrain::Sand
            && !map.is_high(cx, cy)
            && map.feature(cx, cy) == Feature::None
            && !map.is_riverside(cx, cy)
            && west.contains(cx, cy)
            && !keep_clear(cx, cy)
    };
    let near_sea = |cx: i32, cy: i32| (-2..=2).any(|dy| (-2..=2).any(|dx| map.in_bounds(cx + dx, cy + dy) && map.terrain(cx + dx, cy + dy).is_water()));
    let around = (y - 1..=y + 1).flat_map(|cy| (x - 1..=x + DriftwoodPart::W).map(move |cx| (cx, cy))).filter(|&(cx, cy)| !log(cx, cy));
    (0..DriftwoodPart::W).all(|dx| open_sand(x + dx, y))
        && (0..DriftwoodPart::W).any(|dx| near_sea(x + dx, y))
        && around.into_iter().all(|(cx, cy)| !map.props(cx, cy).solid && !keep_clear(cx, cy))
}

#[cfg(test)]
mod tests {
    use super::*;

    const W: i32 = 24;
    const H: i32 = 12;
    const RIVER_X: i32 = 12;

    /// Sea at both ends, a beach either side of a river running north to
    /// south (except across its last rows, as where it meets the sea).
    fn coast() -> WorldMap {
        let mut map = WorldMap::new(W, H, Terrain::Sand);
        for y in 0..H {
            for x in [0, W - 1] {
                map.set_terrain(x, y, Terrain::DeepWater);
            }
            for x in [1, W - 2] {
                map.set_terrain(x, y, Terrain::ShallowWater);
            }
            if y < H - 2 {
                map.set_terrain(RIVER_X, y, Terrain::River);
            }
        }
        map
    }

    fn lucky() -> Rng {
        Rng::new(7)
    }

    /// Mornings enough for the tide to have left whatever it would.
    fn mornings(map: &mut WorldMap, cap: usize, keep_clear: &dyn Fn(i32, i32) -> bool) -> Vec<(i32, i32)> {
        let mut rng = lucky();
        (0..200).flat_map(|_| wash_up(map, &mut rng, cap, keep_clear)).collect()
    }

    fn logs(map: &WorldMap) -> Vec<(i32, i32)> {
        (0..H)
            .flat_map(|y| (0..W).map(move |x| (x, y)))
            .filter(|&(x, y)| matches!(map.feature(x, y), Feature::Driftwood(p) if p.dx == 0))
            .collect()
    }

    #[test]
    fn the_west_coast_lies_beyond_the_river() {
        let map = coast();
        let west = WestCoast::of(&map);
        assert!(west.contains(4, 3));
        assert!(!west.contains(RIVER_X + 3, 3));
    }

    #[test]
    fn rows_the_river_does_not_cross_take_the_nearest_rows_west_coast() {
        let map = coast();
        let west = WestCoast::of(&map);
        assert!(west.contains(4, H - 1));
        assert!(!west.contains(RIVER_X + 3, H - 1));
    }

    #[test]
    fn driftwood_washes_up_on_the_west_coast_only() {
        let mut map = coast();
        mornings(&mut map, 3, &|_, _| false);
        let west = WestCoast::of(&map);
        assert!(!logs(&map).is_empty(), "some driftwood washed up");
        assert!(logs(&map).iter().all(|&(x, y)| west.contains(x, y) && west.contains(x + 1, y)));
    }

    #[test]
    fn driftwood_washes_up_only_some_mornings() {
        let mut map = coast();
        let mut rng = lucky();
        let washed = (0..30).filter(|_| !wash_up(&mut map, &mut rng, 100, &|_, _| false).is_empty()).count();
        assert!(washed > 0 && washed < 30, "{washed} of 30 mornings");
    }

    #[test]
    fn no_more_driftwood_washes_up_than_the_west_coast_held_at_first() {
        let mut map = coast();
        mornings(&mut map, 3, &|_, _| false);
        assert_eq!(logs_on_west_coast(&map), 3);
    }

    #[test]
    fn split_driftwood_comes_back_on_a_later_tide() {
        let mut map = coast();
        mornings(&mut map, 3, &|_, _| false);
        let (x, y) = logs(&map)[0];
        map.set_feature(x, y, Feature::None);
        map.set_feature(x + 1, y, Feature::None);
        assert_eq!(logs_on_west_coast(&map), 2);
        mornings(&mut map, 3, &|_, _| false);
        assert_eq!(logs_on_west_coast(&map), 3);
    }

    #[test]
    fn driftwood_washes_up_near_the_water() {
        let mut map = coast();
        mornings(&mut map, 3, &|_, _| false);
        for (x, y) in logs(&map) {
            assert!(x <= 4, "log at {:?}, away from the sea", (x, y));
        }
    }

    #[test]
    fn driftwood_does_not_wash_up_where_it_must_stay_clear() {
        let mut map = coast();
        let clear = |x: i32, _: i32| x < RIVER_X / 2 + 1;
        mornings(&mut map, 3, &clear);
        assert!(logs(&map).iter().all(|&(x, y)| !clear(x, y) && !clear(x + 1, y)));
    }

    #[test]
    fn driftwood_leaves_a_way_around_it() {
        // A beach two cells wide: a log across it would block the way north.
        let mut map = coast();
        for y in 0..H {
            for x in 4..RIVER_X {
                map.set_terrain(x, y, Terrain::DeepWater);
            }
        }
        assert!(mornings(&mut map, 3, &|_, _| false).is_empty());
    }

    #[test]
    fn driftwood_does_not_wash_up_on_a_feature_or_the_riverside() {
        let mut map = coast();
        for y in 0..H {
            for x in 2..RIVER_X {
                if (x + y) % 2 == 0 {
                    map.set_feature(x, y, Feature::Shell);
                } else {
                    map.set_riverside(x, y, true);
                }
            }
        }
        assert!(mornings(&mut map, 3, &|_, _| false).is_empty());
    }
}

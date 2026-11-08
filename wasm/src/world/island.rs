//! Builds the island from its plan.
//!
//! The island is the same every game. Its rough shape is drawn by hand in
//! `island_plan.txt`, one character for each `PLAN_BLOCK`×`PLAN_BLOCK`
//! cells:
//!
//! | char | ground |
//! |---|---|
//! | `~` | deep sea |
//! | `-` | shallows |
//! | `.` | beach |
//! | `,` | grassland |
//! | `#` | jungle |
//!
//! Each character sets a height for its block; heights are smoothed between
//! blocks and roughened with noise, so coasts and the jungle's edge curve
//! freely instead of following the blocks. Places with a part to play (where
//! the castaway washes up, the river's course) are set here by hand, and the
//! details (trees, stones, shells...) are scattered from a fixed seed.

use super::harvest::{BARK_STRIPS, PALM_COCONUTS};
use crate::items::ItemKind;
use super::map::{TILE_SIZE, WorldMap};
use super::noise::fbm;
use super::river::River;
use super::terrain::{BroadleafPart, DriftwoodPart, Feature, PalmPart, Terrain};
use crate::common::rng::{Rng, hash_unit};

/// The island's plan, and the cells each of its characters stands for.
const PLAN: &str = include_str!("island_plan.txt");
pub const PLAN_BLOCK: i32 = 4;

/// Elevation contours (in tiles above the waterline) separating terrains.
/// The renderer draws transitions along these contours per pixel.
pub const DEEP_BELOW: f32 = -5.0;
pub const SAND_BELOW: f32 = 7.0;
/// Props keep this far (in elevation) from contours so they never sit on a
/// transition.
const PROP_MARGIN: f32 = 1.2;
const JUNGLE_ABOVE: f32 = 17.0;
/// Vines grow within this much elevation below the jungle.
const VINE_BELT: f32 = 5.0;
/// Crabs, dead branches and loose stones for each stretch of island the
/// size of the first area (64×48 cells).
const CRAB_COUNT: usize = 7;
const BRANCH_COUNT: usize = 16;
const STONE_COUNT: usize = 16;
const AREA_CELLS: i32 = 64 * 48;

/// Where the castaway washes up (a cell on the lower beach; the nearest
/// clear spot to it is used).
const LANDING: (i32, i32) = (32, 19);
/// The river: from beyond the north edge down into the cove (cells).
const RIVER_SOURCE: (f32, f32) = (27.46, -4.0);
const RIVER_MOUTH: (f32, f32) = (17.92, 29.14);

/// Steepest the sea floor may fall below the shallows (height per cell),
/// and the depth below which that is kept to.
const SEA_SLOPE: f32 = 1.0;
const SEA_EASED: f32 = DEEP_BELOW + 2.0;

/// Depth at which the noise roughening the land has faded most.
const SEA_CALM: f32 = -8.0;

/// Height a plan character asks for (tiles above the waterline).
fn plan_height(c: char) -> f32 {
    match c {
        '~' => -8.0,
        '-' => -2.5,
        '.' => 3.5,
        ',' => 12.0,
        '#' => 24.0,
        _ => panic!("unknown character {c:?} in the island plan"),
    }
}

/// The plan read into block heights.
struct Plan {
    w: i32,
    h: i32,
    heights: Vec<f32>,
}

impl Plan {
    fn parse(text: &str) -> Self {
        let rows: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
        let w = rows[0].chars().count() as i32;
        assert!(rows.iter().all(|r| r.chars().count() as i32 == w), "island plan rows differ in length");
        let heights = rows.iter().flat_map(|r| r.chars().map(plan_height)).collect();
        Self { w, h: rows.len() as i32, heights }
    }

    fn block(&self, bx: i32, by: i32) -> f32 {
        let (bx, by) = (bx.clamp(0, self.w - 1), by.clamp(0, self.h - 1));
        self.heights[(by * self.w + bx) as usize]
    }

    /// Smooth height at a cell, passing through each block's height at its
    /// centre (Catmull-Rom between block centres).
    fn at(&self, x: f32, y: f32) -> f32 {
        let b = PLAN_BLOCK as f32;
        let (u, v) = ((x + 0.5) / b - 0.5, (y + 0.5) / b - 0.5);
        let (i, j) = (u.floor() as i32, v.floor() as i32);
        let (tu, tv) = (u - i as f32, v - j as f32);
        let row = |by: i32| catmull(self.block(i - 1, by), self.block(i, by), self.block(i + 1, by), self.block(i + 2, by), tu);
        catmull(row(j - 1), row(j), row(j + 1), row(j + 2), tv)
    }
}

fn catmull(p0: f32, p1: f32, p2: f32, p3: f32, t: f32) -> f32 {
    let (t2, t3) = (t * t, t * t * t);
    0.5 * (2.0 * p1 + (p2 - p0) * t + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2 + (3.0 * p1 - p0 - 3.0 * p2 + p3) * t3)
}

/// Height of the land relative to the shoreline, in tiles (negative = sea).
struct HeightField {
    plan: Plan,
    seed: u32,
}

impl HeightField {
    fn at(&self, x: i32, y: i32) -> f32 {
        let (fx, fy) = (x as f32, y as f32);
        let base = self.plan.at(fx, fy);
        // Out at sea the noise fades, so no shallows rise far from land
        // (and the sea floor slopes as gently as along a real shore).
        let rough = ((base - SEA_CALM) / (-2.5 - SEA_CALM)).clamp(0.35, 1.0);
        base + fbm(fx * 0.11, fy * 0.11, self.seed, 3) * 3.0 * rough
    }
}

/// Marks the river's cells (impassable water) and the cells along it.
fn lay_river(map: &mut WorldMap, river: &River) {
    let ts = TILE_SIZE as f32;
    for y in 0..map.height() {
        for x in 0..map.width() {
            let depth = river.depth_px((x as f32 + 0.5) * ts, (y as f32 + 0.5) * ts);
            // Anything within half a cell's diagonal (+ the bank lip) of the water.
            map.set_riverside(x, y, depth > -24.0);
            if depth > -2.0 && map.terrain(x, y).is_land() {
                map.set_terrain(x, y, Terrain::River);
            }
        }
    }
}

/// Picks a bank of the upper river, in the open grassland (the jungle is too
/// thick to reach the water), where water can be drawn.
fn water_spot(map: &WorldMap, river: &River, heights: &[f32]) -> Option<(i32, i32)> {
    let ts = TILE_SIZE as f32;
    for i in 0..60 {
        let t = 0.1 + i as f32 * 0.01;
        let ((cx, cy), (nx, ny)) = river.point(t);
        for side in [1.0, -1.0] {
            let off = River::half_width(t) + 0.45;
            let (bx, by) = (cx + nx * side * off, cy + ny * side * off);
            let (gx, gy) = (bx.round() as i32, by.round() as i32);
            if !map.in_bounds(gx, gy) || !map.terrain(gx, gy).is_land() {
                continue;
            }
            let h = heights[(gy * map.width() + gx) as usize];
            if (SAND_BELOW + 1.0..JUNGLE_ABOVE - 1.0).contains(&h) {
                return Some((((bx + 0.5) * ts) as i32, ((by + 0.5) * ts) as i32));
            }
        }
    }
    None
}

fn classify(h: f32) -> Terrain {
    if h < DEEP_BELOW {
        Terrain::DeepWater
    } else if h < 0.0 {
        Terrain::ShallowWater
    } else if h < SAND_BELOW {
        Terrain::Sand
    } else {
        Terrain::Grass
    }
}

pub fn generate(seed: u32) -> WorldMap {
    let plan = Plan::parse(PLAN);
    let (w, h) = (plan.w * PLAN_BLOCK, plan.h * PLAN_BLOCK);
    let field = HeightField { plan, seed };
    let river = River::new(RIVER_SOURCE, RIVER_MOUTH, seed);
    let mut map = WorldMap::new(w, h, Terrain::DeepWater);
    let mut heights = vec![0.0f32; (w * h) as usize];
    for y in 0..h {
        for x in 0..w {
            heights[(y * w + x) as usize] = field.at(x, y);
        }
    }
    ease_sea_floor(&mut heights, w, h);
    for y in 0..h {
        for x in 0..w {
            map.set_terrain(x, y, classify(heights[(y * w + x) as usize]));
        }
    }
    enforce_transitions(&mut map);
    // Cells turned to sand above keep sand-level elevation, so the grass
    // fringe is drawn around them instead of a square edge.
    for y in 0..h {
        for x in 0..w {
            let e = &mut heights[(y * w + x) as usize];
            if map.terrain(x, y) == Terrain::Sand && *e >= SAND_BELOW {
                *e = SAND_BELOW - 0.8;
            }
        }
    }
    lay_river(&mut map, &river);
    map.water_spot = water_spot(&map, &river, &heights);
    map.river = Some(river);

    let spawn_cell = find_spawn(&map, &heights);
    map.spawn = (spawn_cell.0 * TILE_SIZE, spawn_cell.1 * TILE_SIZE);
    let near_spawn = |x: i32, y: i32| (x - spawn_cell.0).abs() <= 3 && (y - spawn_cell.1).abs() <= 3;

    place_palms(&mut map, &heights, seed, &near_spawn);
    place_broadleafs(&mut map, &heights, seed, &near_spawn);
    place_scatter(&mut map, &heights, seed, &near_spawn);
    map.set_elevation(heights);
    map.crab_spawns = pick_crab_spawns(&map, seed, spawn_cell);
    map.loot = scatter_loot(&map, seed);
    map
}

/// Keeps the sea floor from dropping away faster than `SEA_SLOPE` a cell
/// below the shallows: the shallows and the deep sea are drawn with
/// different palettes, and a steep drop leaves the shallows' cells too deep
/// to blend into their deep neighbours, so the edge between them turns
/// square. (A two-pass sweep, like a distance transform.)
fn ease_sea_floor(heights: &mut [f32], w: i32, h: i32) {
    let diag = SEA_SLOPE * std::f32::consts::SQRT_2;
    let mut sweep = |order: &mut dyn Iterator<Item = (i32, i32)>, back: &[(i32, i32, f32)]| {
        for (x, y) in order {
            let i = (y * w + x) as usize;
            if heights[i] >= SEA_EASED {
                continue;
            }
            for &(dx, dy, step) in back {
                let (nx, ny) = (x + dx, y + dy);
                if (0..w).contains(&nx) && (0..h).contains(&ny) {
                    let floor = (heights[(ny * w + nx) as usize] - step).min(SEA_EASED);
                    heights[i] = heights[i].max(floor);
                }
            }
        }
    };
    let ahead = [(-1, 0, SEA_SLOPE), (0, -1, SEA_SLOPE), (-1, -1, diag), (1, -1, diag)];
    let behind = [(1, 0, SEA_SLOPE), (0, 1, SEA_SLOPE), (1, 1, diag), (-1, 1, diag)];
    sweep(&mut (0..h).flat_map(|y| (0..w).map(move |x| (x, y))), &ahead);
    sweep(&mut (0..h).rev().flat_map(|y| (0..w).rev().map(move |x| (x, y))), &behind);
}

/// How many of something for an island this size, at `per_area` for each
/// stretch the size of the first area.
pub fn for_area(map: &WorldMap, per_area: usize) -> usize {
    (per_area as i64 * (map.width() * map.height()) as i64 / AREA_CELLS as i64) as usize
}

/// Guarantees the sea → shallows → sand → grass ordering so every edge has a
/// matching transition tile.
fn enforce_transitions(map: &mut WorldMap) {
    let snapshot = map.clone();
    let neighbours = |x: i32, y: i32, pred: &dyn Fn(Terrain) -> bool| {
        (-1..=1).any(|dy| (-1..=1).any(|dx| (dx, dy) != (0, 0) && snapshot.in_bounds(x + dx, y + dy) && pred(snapshot.terrain(x + dx, y + dy))))
    };
    for y in 0..map.height() {
        for x in 0..map.width() {
            match snapshot.terrain(x, y) {
                Terrain::DeepWater if neighbours(x, y, &|t| t.is_land()) => {
                    map.set_terrain(x, y, Terrain::ShallowWater)
                }
                Terrain::Grass if neighbours(x, y, &|t| t.is_water()) => map.set_terrain(x, y, Terrain::Sand),
                _ => {}
            }
        }
    }
}

/// The clear spot on the lower beach nearest the landing.
fn find_spawn(map: &WorldMap, heights: &[f32]) -> (i32, i32) {
    let fits = |x: i32, y: i32| {
        map.in_bounds(x, y) && {
            let h = heights[(y * map.width() + x) as usize];
            let clear = (-1..=1).all(|oy| (-1..=1).all(|ox| map.terrain(x + ox, y + oy) == Terrain::Sand));
            (1.5..3.5).contains(&h) && clear
        }
    };
    for r in 0..map.width().max(map.height()) {
        for dy in -r..=r {
            for dx in -r..=r {
                let (x, y) = (LANDING.0 + dx, LANDING.1 + dy);
                if dx.abs().max(dy.abs()) == r && fits(x, y) {
                    return (x, y);
                }
            }
        }
    }
    LANDING
}

fn place_palms(map: &mut WorldMap, heights: &[f32], seed: u32, near_spawn: &dyn Fn(i32, i32) -> bool) {
    let (pw, ph) = (PalmPart::W, PalmPart::H);
    for y in 0..map.height() - ph + 1 {
        for x in 0..map.width() - pw + 1 {
            // Anchor on the trunk base (bottom row), which sits on the upper beach.
            let h = heights[((y + ph - 1) * map.width() + x) as usize];
            if !(2.0..SAND_BELOW).contains(&h) || hash_unit(x, y, seed ^ 0xFA1A) > 0.16 {
                continue;
            }
            // The trunk base and canopy-shadow rows sit on clean sand; the
            // canopy rows above only need dry, empty ground to overhang.
            let fits = (0..ph).all(|dy| {
                (0..pw).all(|dx| {
                    let (cx, cy) = (x + dx, y + dy);
                    let h = heights[(cy * map.width() + cx) as usize];
                    let ground_ok = if dy >= ph - 2 {
                        (PROP_MARGIN..SAND_BELOW - PROP_MARGIN).contains(&h) && map.terrain(cx, cy) == Terrain::Sand
                    } else {
                        map.terrain(cx, cy).is_land()
                    };
                    ground_ok && map.feature(cx, cy) == Feature::None && !near_spawn(cx, cy) && !map.is_riverside(cx, cy)
                })
            });
            // Keep a gap between trees.
            let crowded = (-1..=ph).any(|dy| (-1..=pw).any(|dx| matches!(map.feature(x + dx, y + dy), Feature::Palm(_))));
            if fits && !crowded {
                let flip = hash_unit(x, y, seed ^ 0xF11B) < 0.5;
                // Most palms carry a full bunch; a few have little or none.
                let r = hash_unit(x, y, seed ^ 0xC0C0);
                let coconuts = if r < 0.08 { 0 } else if r < 0.2 { 1 } else if r < 0.5 { 2 } else { PALM_COCONUTS };
                map.set_fruit((x, y), coconuts);
                for dy in 0..ph {
                    for dx in 0..pw {
                        map.set_feature(x + dx, y + dy, Feature::Palm(PalmPart { dx: dx as u8, dy: dy as u8, flip }));
                    }
                }
            }
        }
    }
}

/// Broadleaf trees stand on the grass between the beach and the jungle,
/// thickening toward the jungle's edge.
fn place_broadleafs(map: &mut WorldMap, heights: &[f32], seed: u32, near_spawn: &dyn Fn(i32, i32) -> bool) {
    let (tw, th) = (BroadleafPart::W, BroadleafPart::H);
    for y in 0..map.height() - th + 1 {
        for x in 0..map.width() - tw + 1 {
            let h = heights[((y + th - 1) * map.width() + x + 1) as usize];
            if h < SAND_BELOW + PROP_MARGIN + 1.0 || h > JUNGLE_ABOVE + 2.0 {
                continue;
            }
            let chance = if h > JUNGLE_ABOVE - VINE_BELT { 0.16 } else { 0.07 };
            if hash_unit(x, y, seed ^ 0xB40A) > chance {
                continue;
            }
            // All on clear grass, the crown included.
            let fits = (0..th).all(|dy| {
                (0..tw).all(|dx| {
                    let (cx, cy) = (x + dx, y + dy);
                    let h = heights[(cy * map.width() + cx) as usize];
                    map.terrain(cx, cy) == Terrain::Grass
                        && h > SAND_BELOW + PROP_MARGIN
                        && map.feature(cx, cy) == Feature::None
                        && !near_spawn(cx, cy)
                        && !map.is_riverside(cx, cy)
                })
            });
            let crowded =
                (-1..=th).any(|dy| (-1..=tw).any(|dx| matches!(map.feature(x + dx, y + dy), Feature::Palm(_) | Feature::Broadleaf(_))));
            if fits && !crowded {
                let flip = hash_unit(x, y, seed ^ 0xB41F) < 0.5;
                map.set_bark((x, y), BARK_STRIPS);
                for dy in 0..th {
                    for dx in 0..tw {
                        map.set_feature(x + dx, y + dy, Feature::Broadleaf(BroadleafPart { dx: dx as u8, dy: dy as u8, flip }));
                    }
                }
            }
        }
    }
}

fn place_scatter(map: &mut WorldMap, heights: &[f32], seed: u32, near_spawn: &dyn Fn(i32, i32) -> bool) {
    // A free cell, away from the spawn, the river and terrain edges.
    let open = |map: &WorldMap, x: i32, y: i32| {
        if !(0..map.width()).contains(&x) || map.feature(x, y) != Feature::None || near_spawn(x, y) || map.is_riverside(x, y) {
            return false;
        }
        let h = heights[(y * map.width() + x) as usize];
        h > SAND_BELOW + PROP_MARGIN || (PROP_MARGIN..SAND_BELOW - PROP_MARGIN).contains(&h)
    };
    for y in 0..map.height() {
        for x in 0..map.width() {
            if !open(map, x, y) {
                continue;
            }
            let h = heights[(y * map.width() + x) as usize];
            let r = hash_unit(x, y, seed ^ 0xF00D);
            let feature = match map.terrain(x, y) {
                Terrain::Grass if h > JUNGLE_ABOVE => {
                    if r < 0.6 { Feature::Bush } else if r < 0.66 { Feature::Flower } else { Feature::None }
                }
                // Climbers thrive at the jungle's edge.
                Terrain::Grass if r > 0.93 && h > JUNGLE_ABOVE - VINE_BELT => Feature::Vine,
                Terrain::Grass => {
                    if r < 0.05 { Feature::Bush } else if r < 0.12 { Feature::Flower } else { Feature::None }
                }
                Terrain::Sand => {
                    if r < 0.016 {
                        Feature::Rock
                    } else if r < 0.026 {
                        // A log two cells long, if there's sand for it.
                        if map.terrain(x + 1, y) == Terrain::Sand && open(map, x + 1, y) {
                            let flip = hash_unit(x, y, seed ^ 0xD81F) < 0.5;
                            map.set_feature(x + 1, y, Feature::Driftwood(DriftwoodPart { dx: 1, flip }));
                            Feature::Driftwood(DriftwoodPart { dx: 0, flip })
                        } else {
                            Feature::None
                        }
                    } else if r < 0.05 {
                        Feature::Shell
                    } else {
                        Feature::None
                    }
                }
                _ => Feature::None,
            };
            map.set_feature(x, y, feature);
        }
    }
}

/// Dead branches (mostly on the grass, fallen from the jungle) and loose
/// stones (mostly on the beach and the riverbanks), on open ground.
fn scatter_loot(map: &WorldMap, seed: u32) -> Vec<(ItemKind, (i32, i32))> {
    let mut rng = Rng::new(seed ^ 0x100F);
    let mut out: Vec<(ItemKind, (i32, i32))> = Vec::new();
    for (kind, count, prefer) in [(ItemKind::Branch, for_area(map, BRANCH_COUNT), Terrain::Grass), (ItemKind::Stone, for_area(map, STONE_COUNT), Terrain::Sand)] {
        let mut placed = 0;
        for _ in 0..4000 {
            if placed >= count {
                break;
            }
            let (x, y) = (rng.between(0, map.width() - 1), rng.between(0, map.height() - 1));
            let t = map.terrain(x, y);
            let liked = t == prefer || (kind == ItemKind::Stone && map.is_riverside(x, y));
            let open = t.is_land() && !map.props(x, y).solid && matches!(map.feature(x, y), Feature::None | Feature::Flower | Feature::Shell);
            let taken = out.iter().any(|&(_, (px, py))| px.div_euclid(TILE_SIZE) == x && py.div_euclid(TILE_SIZE) == y);
            if !open || taken || (!liked && !rng.chance(0.25)) {
                continue;
            }
            let at = (x * TILE_SIZE + 8 + rng.between(-4, 4), y * TILE_SIZE + 10 + rng.between(-3, 3));
            out.push((kind, at));
            placed += 1;
        }
    }
    out
}

fn pick_crab_spawns(map: &WorldMap, seed: u32, spawn: (i32, i32)) -> Vec<(i32, i32)> {
    let mut rng = Rng::new(seed ^ 0xC4AB);
    let mut out = Vec::new();
    for _ in 0..2000 {
        if out.len() >= for_area(map, CRAB_COUNT) {
            break;
        }
        let (x, y) = (rng.between(0, map.width() - 1), rng.between(0, map.height() - 1));
        let ok = map.terrain(x, y) == Terrain::Sand
            && !map.props(x, y).solid
            && ((x - spawn.0).abs() > 4 || (y - spawn.1).abs() > 4);
        if ok {
            out.push((x * TILE_SIZE, y * TILE_SIZE));
        }
    }
    out
}

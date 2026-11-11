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
//! | `#` | jungle (a thicket) |
//! | `T` | forest |
//! | `^` | high ground above a cliff |
//!
//! Each character sets a height for its block; heights are smoothed between
//! blocks and roughened with noise, so coasts and the jungle's edge curve
//! freely instead of following the blocks. The high ground is a layer of its
//! own: where it ends the ground drops away in a cliff face, and below it the
//! land (or sea) carries on as drawn around it. Places with a part to play (where
//! the castaway washes up, the river's course) are set here by hand, and the
//! details (trees, stones, shells...) are scattered from a fixed seed.

use super::harvest::{BARK_STRIPS, PALM_COCONUTS, PINE_CONES};
use crate::items::ItemKind;
use super::map::{Spots, TILE_SIZE, WorldMap};
use super::noise::fbm;
use super::river::River;
use super::terrain::{BroadleafPart, DriftwoodPart, Feature, PalmPart, PinePart, Terrain};
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
/// Dry land in the first area (sand and grass cells), the stretch those
/// counts are for.
const AREA_LAND: i32 = 1531;

/// Where the first area's top-left cell lies on the map. Places below are
/// given in the first area's cells (as they were before the map grew north),
/// and details are scattered by those cells, so the first area stays as it
/// was drawn.
const ORIGIN: (i32, i32) = (0, 24);
/// Where the castaway washes up (a cell on the lower beach; the nearest
/// clear spot to it is used).
const LANDING: (i32, i32) = (32, 19);
/// The river: winding from the forest (where the stream out of the pool
/// becomes a river) down into the cove (cells).
const RIVER_SOURCE: (f32, f32) = (27.46, -4.0);
const RIVER_MOUTH: (f32, f32) = (17.92, 29.14);
/// The stream rises on the high ground beyond the north edge and falls
/// over the cliff into a pool (centre and radius, cells).
const SPRING: (f32, f32) = (POOL.0, -30.0);
const POOL: (f32, f32) = (26.5, -6.3);
const POOL_RADIUS: f32 = 2.9;
/// Where the river narrows (a place a bridge could span) and how narrow.
const BRIDGE_AT: (f32, f32) = (22.0, 12.0);
const BRIDGE_HALF_WIDTH: f32 = 0.7;
/// How far the current runs out past the mouth at most (cells).
const OUTFLOW_MAX: f32 = 16.0;
/// How fast the current fans out past the mouth (see `River::with_outflow`).
const OUTFLOW_SPREAD: f32 = 0.14;
/// How far from a cell's centre the current may run and still make the
/// cell unwadable (px; half a cell's diagonal).
const SEA_CURRENT_REACH: f32 = 11.5;

/// Cells a cliff face drops below the high ground's edge, and the low
/// stretch of the north cliff (a place a ladder could reach the top),
/// from its left cell, cells wide. The low stretch is a dip in the
/// cliff's top: its edge is a row lower than the rock beside it, its foot
/// in line with the rest, and its middle cell is where a ladder stands.
const CLIFF_HEIGHT: i32 = 3;
const LOW_CLIFF: (i32, i32) = (62, 3);
const LOW_CLIFF_HEIGHT: i32 = 2;
/// How far either side of the low stretch the cliff's edge is drawn
/// straight (cells), so the recess stands out from the rock around it.
const LOW_CLIFF_STRAIGHT: i32 = 3;
/// Radius (cells) of the clearing in the forest in front of the recess.
const LOW_CLIFF_CLEARING: f32 = 3.7;

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
        'T' => 11.0,
        '#' => 24.0,
        _ => panic!("unknown character {c:?} in the island plan"),
    }
}

/// The plan read into blocks: the height of the land, and which blocks
/// are high ground and forest.
struct Plan {
    w: i32,
    h: i32,
    heights: Vec<f32>,
    high: Vec<f32>,
    forest: Vec<f32>,
}

impl Plan {
    fn parse(text: &str) -> Self {
        let rows: Vec<Vec<char>> = text.lines().filter(|l| !l.trim().is_empty()).map(|r| r.chars().collect()).collect();
        let (w, h) = (rows[0].len() as i32, rows.len() as i32);
        assert!(rows.iter().all(|r| r.len() as i32 == w), "island plan rows differ in length");
        let chars: Vec<char> = rows.concat();
        // Under the high ground the land (or sea) is the nearest drawn
        // beside it, so it meets the cliff as it would carry on.
        let mut low = chars.clone();
        let mut left = low.iter().filter(|&&c| c == '^').count();
        while left > 0 {
            let before = low.clone();
            for i in 0..low.len() {
                if before[i] != '^' {
                    continue;
                }
                let (x, y) = (i as i32 % w, i as i32 / w);
                // South first: a cliff facing the sea stands in the sea.
                let near = [(0, 1), (-1, 0), (1, 0), (0, -1)]
                    .iter()
                    .map(|&(dx, dy)| (x + dx, y + dy))
                    .filter(|&(nx, ny)| (0..w).contains(&nx) && (0..h).contains(&ny))
                    .map(|(nx, ny)| before[(ny * w + nx) as usize])
                    .find(|&c| c != '^');
                if let Some(c) = near {
                    low[i] = c;
                    left -= 1;
                }
            }
            assert!(low != before, "the island plan is all high ground");
        }
        let mark = |want: char| chars.iter().map(|&c| if c == want { 1.0 } else { 0.0 }).collect();
        Self { w, h, heights: low.iter().map(|&c| plan_height(c)).collect(), high: mark('^'), forest: mark('T') }
    }

    /// Smooth value of `blocks` at a cell, passing through each block's
    /// value at its centre (Catmull-Rom between block centres).
    fn smooth(&self, blocks: &[f32], x: f32, y: f32) -> f32 {
        let block = |bx: i32, by: i32| blocks[(by.clamp(0, self.h - 1) * self.w + bx.clamp(0, self.w - 1)) as usize];
        let b = PLAN_BLOCK as f32;
        let (u, v) = ((x + 0.5) / b - 0.5, (y + 0.5) / b - 0.5);
        let (i, j) = (u.floor() as i32, v.floor() as i32);
        let (tu, tv) = (u - i as f32, v - j as f32);
        let row = |by: i32| catmull(block(i - 1, by), block(i, by), block(i + 1, by), block(i + 2, by), tu);
        catmull(row(j - 1), row(j), row(j + 1), row(j + 2), tv)
    }

    /// Smooth height of the land at a cell.
    fn at(&self, x: f32, y: f32) -> f32 {
        self.smooth(&self.heights, x, y)
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
        let base = self.plan.at(x as f32, y as f32);
        let (nx, ny) = ((x - ORIGIN.0) as f32, (y - ORIGIN.1) as f32);
        // Out at sea the noise fades, so no shallows rise far from land
        // (and the sea floor slopes as gently as along a real shore).
        let rough = ((base - SEA_CALM) / (-2.5 - SEA_CALM)).clamp(0.35, 1.0);
        base + fbm(nx * 0.11, ny * 0.11, self.seed, 3) * 3.0 * rough
    }

    /// Whether a cell is on the high ground.
    fn is_high(&self, x: i32, y: i32) -> bool {
        let (nx, ny) = ((x - ORIGIN.0) as f32, (y - ORIGIN.1) as f32);
        self.plan.smooth(&self.plan.high, x as f32, y as f32) + fbm(nx * 0.13, ny * 0.13, self.seed ^ 0x41C4, 2) * 0.4 > 0.5
    }

    /// How much a cell is in the forest (0 to 1).
    fn forest(&self, x: i32, y: i32) -> f32 {
        self.plan.smooth(&self.plan.forest, x as f32, y as f32).clamp(0.0, 1.0)
    }
}

/// A cell in the first area's cells (see `ORIGIN`).
fn at(c: (i32, i32)) -> (i32, i32) {
    (c.0 + ORIGIN.0, c.1 + ORIGIN.1)
}

fn at_f(c: (f32, f32)) -> (f32, f32) {
    (c.0 + ORIGIN.0 as f32, c.1 + ORIGIN.1 as f32)
}

/// A detail's roll for a cell, by the first area's cells.
fn roll(x: i32, y: i32, seed: u32) -> f32 {
    hash_unit(x - ORIGIN.0, y - ORIGIN.1, seed)
}

/// Marks the high ground: the cells above the cliff, its edge, and the
/// rock face dropping below the edge. Returns which cells are high.
fn raise_high_ground(map: &mut WorldMap, field: &HeightField) -> Vec<bool> {
    let (w, h) = (map.width(), map.height());
    let mut high: Vec<bool> = (0..h).flat_map(|y| (0..w).map(move |x| (x, y))).map(|(x, y)| field.is_high(x, y)).collect();
    let idx = |x: i32, y: i32| (y * w + x) as usize;
    // No holes or overhangs: high ground runs unbroken down from the north
    // edge, and gaps a cell wide are filled.
    for x in 0..w {
        for y in 1..h {
            if !high[idx(x, y - 1)] {
                high[idx(x, y)] = false;
            }
        }
    }
    for y in 0..h {
        for x in 1..w - 1 {
            if !high[idx(x, y)] && high[idx(x - 1, y)] && high[idx(x + 1, y)] && (y == 0 || high[idx(x, y - 1)]) {
                high[idx(x, y)] = true;
            }
        }
    }
    // Around the low stretch the edge runs straight, at the row it has
    // over the stretch's first cell.
    let around = LOW_CLIFF.0 - LOW_CLIFF_STRAIGHT..LOW_CLIFF.0 + LOW_CLIFF.1 + LOW_CLIFF_STRAIGHT;
    if let Some(edge) = (0..h / 2).find(|&y| high[idx(LOW_CLIFF.0, y)] && !high[idx(LOW_CLIFF.0, y + 1)]) {
        for x in around {
            for y in 0..h / 2 {
                high[idx(x, y)] = y <= edge;
            }
        }
        // The low stretch's edge is a row lower: its face is shorter, and
        // stands on the same ground as the rest of the cliff.
        for x in LOW_CLIFF.0..LOW_CLIFF.0 + LOW_CLIFF.1 {
            high[idx(x, edge + 1)] = true;
        }
    }
    let is_high = |x: i32, y: i32| !map.in_bounds(x, y) || high[idx(x, y)];
    let mut terrain = Vec::new();
    for y in 0..h {
        for x in 0..w {
            if !high[idx(x, y)] {
                continue;
            }
            let edge = (-1..=1).any(|dy| (-1..=1).any(|dx| !is_high(x + dx, y + dy)));
            terrain.push((x, y, if edge { Terrain::CliffTop } else { Terrain::Highland }));
            if !is_high(x, y + 1) {
                let low = (LOW_CLIFF.0..LOW_CLIFF.0 + LOW_CLIFF.1).contains(&x) && y < h / 2;
                let drop = if low { LOW_CLIFF_HEIGHT } else { CLIFF_HEIGHT };
                for k in 1..=drop {
                    if map.in_bounds(x, y + k) && !is_high(x, y + k) {
                        terrain.push((x, y + k, Terrain::Cliff));
                    }
                }
            }
        }
    }
    for (x, y, t) in terrain {
        map.set_terrain(x, y, t);
    }
    high
}

/// Marks the river's cells (impassable water) and the cells along it.
fn lay_river(map: &mut WorldMap, river: &River) {
    let ts = TILE_SIZE as f32;
    for y in 0..map.height() {
        for x in 0..map.width() {
            let depth = river.depth_px((x as f32 + 0.5) * ts, (y as f32 + 0.5) * ts);
            // Anything within half a cell's diagonal (+ the bank lip) of the water.
            map.set_riverside(x, y, depth > -24.0);
            let t = map.terrain(x, y);
            // On land the banks are cut back to the water; out at sea the
            // current is too strong to wade where it runs out to the deep
            // (every cell it reaches into, as it is drawn to the pixel).
            if (depth > -2.0 && (t.is_land() || t == Terrain::CliffTop)) || (depth > -SEA_CURRENT_REACH && t == Terrain::ShallowWater) {
                map.set_terrain(x, y, Terrain::River);
            }
        }
    }
}

/// Picks a bank of the pool under the falls, on the grass of the clearing
/// around it (toward the south-east first), where water can be drawn.
fn water_spot(map: &WorldMap, river: &River) -> Option<(i32, i32)> {
    let ts = TILE_SIZE as f32;
    let ((cx, cy), r) = river.pool()?;
    for i in 0..32 {
        // Swing out either way from south-east.
        let turn = (i / 2) as f32 * 0.2 * if i % 2 == 0 { 1.0 } else { -1.0 };
        let a = std::f32::consts::FRAC_PI_4 + turn;
        let (bx, by) = (cx + a.cos() * (r + 0.45), cy + a.sin() * (r + 0.45));
        let (gx, gy) = (bx.round() as i32, by.round() as i32);
        let clear = (-1..=1).all(|dy| (-1..=1).all(|dx| map.terrain(gx + dx, gy + dy) != Terrain::Cliff));
        if map.terrain(gx, gy) == Terrain::Grass && clear && river.nearest(bx, by).depth() < -0.3 {
            return Some((((bx + 0.5) * ts) as i32, ((by + 0.5) * ts) as i32));
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
    raise_high_ground(&mut map, &field);
    let below = |x: f32, y: f32, level: f32| {
        let (cx, cy) = (x.round() as i32, y.round() as i32);
        !map.in_bounds(cx, cy) || heights[(cy * w + cx) as usize] < level
    };
    let sea = |x: f32, y: f32| below(x, y, 0.0);
    let deep = |x: f32, y: f32| below(x, y, DEEP_BELOW);
    let river = River::new(at_f(RIVER_SOURCE), at_f(RIVER_MOUTH), seed)
        .with_head(at_f(SPRING), at_f(POOL), POOL_RADIUS)
        .narrowed(at_f(BRIDGE_AT), BRIDGE_HALF_WIDTH, 2.5)
        .with_outflow(OUTFLOW_MAX, OUTFLOW_SPREAD, sea, deep);
    lay_river(&mut map, &river);
    map.water_spot = water_spot(&map, &river);
    map.spots = Spots {
        bridge: Some(nearest_on_river(&river, at_f(BRIDGE_AT))),
        ladder: low_cliff_foot(&map),
        falls: river.pool().map(|((x, y), _)| (x.round() as i32, y.round() as i32)),
    };
    map.river = Some(river);

    let spawn_cell = find_spawn(&map, &heights);
    map.spawn = (spawn_cell.0 * TILE_SIZE, spawn_cell.1 * TILE_SIZE);
    // Nothing grows close to the spawn, and the forest opens out in front
    // of the recess in the cliff, drawing the eye to it.
    let clearing = map.spots.ladder.map(|(x, y)| (x as f32, y as f32 + 2.0));
    let in_clearing = |x: i32, y: i32| clearing.is_some_and(|(cx, cy)| (x as f32 - cx).hypot(y as f32 - cy) < LOW_CLIFF_CLEARING);
    let near_spawn = |x: i32, y: i32| (x - spawn_cell.0).abs() <= 3 && (y - spawn_cell.1).abs() <= 3 || in_clearing(x, y);
    let forest: Vec<f32> = (0..h).flat_map(|y| (0..w).map(move |x| (x, y))).map(|(x, y)| field.forest(x, y)).collect();

    place_pine(&mut map);
    place_palms(&mut map, &heights, seed, &near_spawn);
    place_broadleafs(&mut map, &heights, &forest, seed, &near_spawn);
    place_scatter(&mut map, &heights, &forest, seed, &near_spawn);
    drop_pine_cones(&mut map);
    fill_cliff_feet(&mut map);
    map.set_elevation(heights);
    map.crab_spawns = pick_crab_spawns(&map, seed, spawn_cell);
    map.loot = scatter_loot(&map, seed);
    map
}

/// Share of the trees on the high ground that are pines (the rest are
/// broadleafs): low ground grows broadleafs, high ground conifers.
const HIGHLAND_PINES: f32 = 0.8;

/// Where the pine above the cliff's recess stands, from the recess's middle
/// (cells): just right of it, its foot on the row behind the edge, so its
/// tiers show over the cliff from below.
const PINE_FROM_RECESS: (i32, i32) = (1, -1);

/// The pine on the high ground above the recess in the north cliff (see
/// `LOW_CLIFF`): a sign from below of what grows up there. It goes in
/// before the other trees, which keep clear of it.
fn place_pine(map: &mut WorldMap) {
    let Some((x, foot)) = map.spots.ladder else { return };
    // The edge either side of the recess, a row above its lowered edge.
    let edge = foot - LOW_CLIFF_HEIGHT - 2;
    let (bx, by) = (x + PINE_FROM_RECESS.0, edge + PINE_FROM_RECESS.1);
    let (ax, ay) = (bx - PinePart::W / 2, by - PinePart::H + 1);
    let fits = (0..PinePart::H).all(|dy| (0..PinePart::W).all(|dx| map.terrain(ax + dx, ay + dy) == Terrain::Highland));
    if !fits {
        return;
    }
    for dy in 0..PinePart::H {
        for dx in 0..PinePart::W {
            map.set_feature(ax + dx, ay + dy, Feature::Pine(PinePart { dx: dx as u8, dy: dy as u8, flip: false }));
        }
    }
    map.set_fruit((ax, ay), PINE_CONES);
}

/// Pine cones fallen from the pine above the recess: a few on the high
/// ground under it, and one that rolled down the crumbled edge to lie at
/// the foot of the recess's middle, the only one below the cliff: picking
/// it up, the castaway looks up and thinks of climbing (see
/// `Spots::ladder`).
fn drop_pine_cones(map: &mut WorldMap) {
    let Some((x, foot)) = map.spots.ladder else { return };
    let edge = foot - LOW_CLIFF_HEIGHT - 2;
    let above = [(-1, -1), (-2, -2), (3, -1)].map(|(dx, dy)| (x + dx, edge + dy));
    for (cx, cy) in above {
        if map.terrain(cx, cy) == Terrain::Highland && map.feature(cx, cy) == Feature::None {
            map.set_feature(cx, cy, Feature::PineCone);
        }
    }
    if map.terrain(x, foot) == Terrain::Grass && map.feature(x, foot) == Feature::None {
        map.set_feature(x, foot, Feature::PineCone);
    }
}

/// Fills the inner corners of the stairs a slanting cliff makes: a bare
/// ground cell with a cliff face above it and beside it gets the rock of
/// the face carried down into its top half on that side, so the cliff's
/// foot runs on a slant. Not by the water (the surf, the river) or under
/// trees and bushes.
fn fill_cliff_feet(map: &mut WorldMap) {
    let mut fills = Vec::new();
    for y in 0..map.height() {
        for x in 0..map.width() {
            let wet = (-1..=1).any(|dy| (-1..=1).any(|dx| map.terrain(x + dx, y + dy).holds_water()));
            if !matches!(map.terrain(x, y), Terrain::Grass | Terrain::Sand)
                || map.terrain(x, y - 1) != Terrain::Cliff
                || map.feature(x, y) != Feature::None
                || map.is_riverside(x, y)
                || wet
            {
                continue;
            }
            fills.push((x, y, [map.terrain(x - 1, y) == Terrain::Cliff, map.terrain(x + 1, y) == Terrain::Cliff]));
        }
    }
    for (x, y, fill) in fills {
        map.set_foot_fill(x, y, fill);
    }
}

/// The cell of the river's centreline nearest a point.
fn nearest_on_river(river: &River, p: (f32, f32)) -> (i32, i32) {
    // Walk across the channel to its middle.
    let (mut x, mut y) = p;
    for _ in 0..40 {
        let here = river.nearest(x, y);
        if here.dist < 0.05 {
            break;
        }
        let step = 0.05;
        let probe = |dx: f32, dy: f32| river.nearest(x + dx, y + dy).dist;
        let (gx, gy) = (probe(step, 0.0) - probe(-step, 0.0), probe(0.0, step) - probe(0.0, -step));
        let g = (gx * gx + gy * gy).sqrt().max(1e-4);
        x -= gx / g * here.dist;
        y -= gy / g * here.dist;
    }
    (x.round() as i32, y.round() as i32)
}

/// The cell at the foot of the low stretch of the north cliff.
fn low_cliff_foot(map: &WorldMap) -> Option<(i32, i32)> {
    // The middle of the recess.
    let x = LOW_CLIFF.0 + LOW_CLIFF.1 / 2;
    (0..map.height() / 2).find(|&y| map.terrain(x, y) == Terrain::Cliff && map.terrain(x, y + 1) != Terrain::Cliff).map(|y| (x, y + 1))
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
/// stretch of dry land the size of the first area's (the high ground above
/// the cliffs not counted).
pub fn for_area(map: &WorldMap, per_area: usize) -> usize {
    let land = (0..map.height()).flat_map(|y| (0..map.width()).map(move |x| (x, y))).filter(|&(x, y)| matches!(map.terrain(x, y), Terrain::Sand | Terrain::Grass)).count();
    per_area * land / AREA_LAND as usize
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
                let (x, y) = (at(LANDING).0 + dx, at(LANDING).1 + dy);
                if dx.abs().max(dy.abs()) == r && fits(x, y) {
                    return (x, y);
                }
            }
        }
    }
    at(LANDING)
}

fn place_palms(map: &mut WorldMap, heights: &[f32], seed: u32, near_spawn: &dyn Fn(i32, i32) -> bool) {
    let (pw, ph) = (PalmPart::W, PalmPart::H);
    for y in 0..map.height() - ph + 1 {
        for x in 0..map.width() - pw + 1 {
            // Anchor on the trunk base (bottom row), which sits on the upper beach.
            let h = heights[((y + ph - 1) * map.width() + x) as usize];
            if !(2.0..SAND_BELOW).contains(&h) || roll(x, y, seed ^ 0xFA1A) > 0.16 {
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
                let flip = roll(x, y, seed ^ 0xF11B) < 0.5;
                // Most palms carry a full bunch; a few have little or none.
                let r = roll(x, y, seed ^ 0xC0C0);
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
/// thickening toward the jungle's edge, and close together in the forest;
/// on the high ground most of the trees are pines (`HIGHLAND_PINES`).
fn place_broadleafs(map: &mut WorldMap, heights: &[f32], forest: &[f32], seed: u32, near_spawn: &dyn Fn(i32, i32) -> bool) {
    let (tw, th) = (BroadleafPart::W, BroadleafPart::H);
    for y in 0..map.height() - th + 1 {
        for x in 0..map.width() - tw + 1 {
            let foot = ((y + th - 1) * map.width() + x + 1) as usize;
            let ground = map.terrain(x + 1, y + th - 1);
            let h = heights[foot];
            let low = h < SAND_BELOW + PROP_MARGIN + 1.0 || h > JUNGLE_ABOVE + 2.0;
            if ground != Terrain::Highland && low {
                continue;
            }
            let edge: f32 = if h > JUNGLE_ABOVE - VINE_BELT { 0.16 } else { 0.07 };
            let chance = if ground == Terrain::Highland { 0.12 } else { edge.max(forest[foot] * 0.4) };
            if roll(x, y, seed ^ 0xB40A) > chance {
                continue;
            }
            // All on clear grass (or all on the high ground), the crown
            // included.
            let fits = (0..th).all(|dy| {
                (0..tw).all(|dx| {
                    let (cx, cy) = (x + dx, y + dy);
                    let h = heights[(cy * map.width() + cx) as usize];
                    let on = map.terrain(cx, cy);
                    let ground_ok = if ground == Terrain::Highland { on == Terrain::Highland } else { on == Terrain::Grass && h > SAND_BELOW + PROP_MARGIN };
                    ground_ok
                        && map.feature(cx, cy) == Feature::None
                        && !near_spawn(cx, cy)
                        && !map.is_riverside(cx, cy)
                })
            });
            let crowded = (-1..=th).any(|dy| {
                (-1..=tw).any(|dx| matches!(map.feature(x + dx, y + dy), Feature::Palm(_) | Feature::Broadleaf(_) | Feature::Pine(_)))
            });
            if fits && !crowded {
                let flip = roll(x, y, seed ^ 0xB41F) < 0.5;
                // Up on the high ground most trees are pines.
                let pine = ground == Terrain::Highland && roll(x, y, seed ^ 0x914A) < HIGHLAND_PINES;
                if pine {
                    map.set_fruit((x, y), PINE_CONES);
                } else {
                    map.set_bark((x, y), BARK_STRIPS);
                }
                for dy in 0..th {
                    for dx in 0..tw {
                        let (dx8, dy8) = (dx as u8, dy as u8);
                        let f = if pine { Feature::Pine(PinePart { dx: dx8, dy: dy8, flip }) } else { Feature::Broadleaf(BroadleafPart { dx: dx8, dy: dy8, flip }) };
                        map.set_feature(x + dx, y + dy, f);
                    }
                }
            }
        }
    }
}

fn place_scatter(map: &mut WorldMap, heights: &[f32], forest: &[f32], seed: u32, near_spawn: &dyn Fn(i32, i32) -> bool) {
    // A free cell, away from the spawn, the river and terrain edges.
    let open = |map: &WorldMap, x: i32, y: i32| {
        if !(0..map.width()).contains(&x) || map.feature(x, y) != Feature::None || near_spawn(x, y) || map.is_riverside(x, y) {
            return false;
        }
        let h = heights[(y * map.width() + x) as usize];
        map.terrain(x, y) == Terrain::Highland || h > SAND_BELOW + PROP_MARGIN || (PROP_MARGIN..SAND_BELOW - PROP_MARGIN).contains(&h)
    };
    for y in 0..map.height() {
        for x in 0..map.width() {
            if !open(map, x, y) {
                continue;
            }
            let h = heights[(y * map.width() + x) as usize];
            let r = roll(x, y, seed ^ 0xF00D);
            let feature = match map.terrain(x, y) {
                // Thick growth on the high ground.
                Terrain::Highland => {
                    if r < 0.45 { Feature::Bush } else if r < 0.5 { Feature::Flower } else { Feature::None }
                }
                Terrain::Grass if h > JUNGLE_ABOVE => {
                    if r < 0.6 { Feature::Bush } else if r < 0.66 { Feature::Flower } else { Feature::None }
                }
                // Climbers thrive at the jungle's edge.
                Terrain::Grass if r > 0.93 && h > JUNGLE_ABOVE - VINE_BELT => Feature::Vine,
                // Undergrowth in the forest, light enough to walk through.
                Terrain::Grass if forest[(y * map.width() + x) as usize] > 0.5 => {
                    if r < 0.06 { Feature::Bush } else if r < 0.09 { Feature::Flower } else { Feature::None }
                }
                Terrain::Grass => {
                    if r < 0.05 { Feature::Bush } else if r < 0.12 { Feature::Flower } else { Feature::None }
                }
                Terrain::Sand => {
                    if r < 0.016 {
                        Feature::Rock
                    } else if r < 0.026 {
                        // A log two cells long, if there's sand for it.
                        if map.terrain(x + 1, y) == Terrain::Sand && open(map, x + 1, y) {
                            let flip = roll(x, y, seed ^ 0xD81F) < 0.5;
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
            let open = matches!(t, Terrain::Sand | Terrain::Grass)
                && !map.props(x, y).solid
                && map.foot_fill(x, y) == [false, false]
                && matches!(map.feature(x, y), Feature::None | Feature::Flower | Feature::Shell);
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
            && map.foot_fill(x, y) == [false, false]
            && ((x - spawn.0).abs() > 4 || (y - spawn.1).abs() > 4);
        if ok {
            out.push((x * TILE_SIZE, y * TILE_SIZE));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use super::*;
    use crate::world::map::LadderFit;

    /// Cells that can be walked to from `start` (stepping diagonally only
    /// where both sides are open, as the castaway can).
    fn reachable(map: &WorldMap, start: (i32, i32)) -> Vec<bool> {
        let w = map.width();
        let open = |x: i32, y: i32| map.in_bounds(x, y) && !map.props(x, y).solid;
        let mut seen = vec![false; (w * map.height()) as usize];
        let mut queue = VecDeque::from([start]);
        seen[(start.1 * w + start.0) as usize] = true;
        while let Some((x, y)) = queue.pop_front() {
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (1, -1), (-1, 1), (-1, -1)] {
                let (nx, ny) = (x + dx, y + dy);
                let diagonal_ok = dx == 0 || dy == 0 || (open(x + dx, y) && open(x, y + dy));
                if open(nx, ny) && diagonal_ok && !seen[(ny * w + nx) as usize] {
                    seen[(ny * w + nx) as usize] = true;
                    queue.push_back((nx, ny));
                }
            }
        }
        seen
    }

    /// The island, and what can be walked to from where the castaway
    /// washes up.
    struct Walk {
        map: WorldMap,
        reach: Vec<bool>,
    }

    impl Walk {
        fn from_landing(map: WorldMap) -> Self {
            // The castaway's feet are in the cell below the spawn point.
            let landing = (map.spawn.0 / TILE_SIZE, map.spawn.1 / TILE_SIZE + 1);
            let reach = reachable(&map, landing);
            Self { map, reach }
        }

        fn can(&self, (x, y): (i32, i32)) -> bool {
            self.map.in_bounds(x, y) && self.reach[(y * self.map.width() + x) as usize]
        }

        /// Whether a cell can be reached or stood beside.
        fn near(&self, (x, y): (i32, i32)) -> bool {
            (-1..=1).any(|dy| (-1..=1).any(|dx| self.can((x + dx, y + dy))))
        }

        fn reaches_high_ground(&self) -> bool {
            let w = self.map.width();
            (0..self.map.height()).any(|y| (0..w).any(|x| self.can((x, y)) && self.map.terrain(x, y) == Terrain::Highland))
        }
    }

    fn island() -> WorldMap {
        generate(1)
    }

    /// The cell of rock face a ladder stands against at the low cliff.
    fn ladder_face(map: &WorldMap) -> ((i32, i32), i32) {
        let (x, y) = map.spots.ladder.expect("the island has a low cliff");
        (y - 3..=y)
            .find_map(|fy| match map.ladder_fit(x, fy) {
                LadderFit::Reaches { foot, rows } => Some((foot, rows)),
                LadderFit::No => None,
            })
            .expect("a ladder fits at the low cliff")
    }

    #[test]
    fn the_island_is_made_the_same_from_the_same_seed() {
        let (a, b) = (island(), island());
        assert_eq!(a.spawn, b.spawn);
        assert_eq!(a.loot, b.loot);
        let w = a.width();
        assert!((0..a.height()).all(|y| (0..w).all(|x| a.terrain(x, y) == b.terrain(x, y) && a.feature(x, y) == b.feature(x, y))));
    }

    #[test]
    fn the_castaway_washes_up_on_open_ground() {
        let walk = Walk::from_landing(island());
        let w = walk.map.width();
        let cells = walk.reach.iter().filter(|&&r| r).count();
        assert!(cells > (w * 4) as usize, "more than a pocket to walk in");
    }

    #[test]
    fn the_cliff_cant_be_climbed_without_a_ladder() {
        assert!(!Walk::from_landing(island()).reaches_high_ground());
    }

    #[test]
    fn the_water_spot_can_be_reached() {
        let walk = Walk::from_landing(island());
        let spot = walk.map.water_spot.expect("the island has a water spot");
        assert!(walk.near((spot.0 / TILE_SIZE, spot.1 / TILE_SIZE)));
    }

    #[test]
    fn the_foot_of_the_low_cliff_can_be_reached() {
        let walk = Walk::from_landing(island());
        assert!(walk.can(walk.map.spots.ladder.expect("the island has a low cliff")));
    }

    #[test]
    fn the_river_cant_be_crossed() {
        let walk = Walk::from_landing(island());
        let (bx, by) = walk.map.spots.bridge.expect("the river has a narrow");
        let bank = |side: i32| (1..8).map(|k| (bx + side * k, by)).find(|&(x, y)| walk.map.terrain(x, y) != Terrain::River).unwrap();
        let (west, east) = (walk.near(bank(-1)), walk.near(bank(1)));
        assert!(west != east, "one bank of the narrow is reached and the other isn't (west {west}, east {east})");
    }

    #[test]
    fn a_ladder_fits_only_in_the_middle_of_the_low_cliff() {
        let map = island();
        let ((x, y), _) = ladder_face(&map);
        assert_eq!(map.ladder_fit(x - 1, y), LadderFit::No);
        assert_eq!(map.ladder_fit(x + 1, y), LadderFit::No);
    }

    #[test]
    fn a_ladder_stood_at_the_low_cliff_leads_up_to_the_high_ground() {
        let mut map = island();
        let (foot, rows) = ladder_face(&map);
        map.stand_ladder(foot, rows);
        assert!(Walk::from_landing(map).reaches_high_ground());
    }
}

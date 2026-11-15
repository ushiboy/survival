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
//! | `B` | bamboo grove (on grassland) |
//! | `W` | the lake's deep water (on the high ground) |
//! | `w` | the lake's shallows (on the high ground) |
//! | `s` | the lake's shore (on the high ground) |
//! | `R` | crags closing off the high ground |
//! | `b` | a bamboo grove on the high ground |
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
use super::terrain::{BambooPart, BroadleafPart, ClayPart, DriftwoodPart, Feature, PalmPart, PinePart, Soil, Terrain, patch_cells};
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
/// The same up on the high ground, for each stretch of it the size of the
/// first area's dry land: branches fall as thick under its trees, but
/// stones are fewer, with no beaches or riverbanks to gather them.
const HIGH_BRANCH_COUNT: usize = 16;
const HIGH_STONE_COUNT: usize = 8;
/// Share of the high ground's cells by its trees that grow vines.
const HIGH_VINES: f32 = 0.015;
/// The share of cells beside a tree or a bamboo culm on the west coast
/// (beyond the river) where a vine grows.
const WEST_VINES: f32 = 0.05;
/// The share of jungle cells a bush grows in; on the west coast (beyond
/// the river) fewer, so its thickets can be walked through.
const JUNGLE_BUSHES: f32 = 0.6;
const WEST_JUNGLE_BUSHES: f32 = 0.25;
/// Dry land in the first area (sand and grass cells), the stretch those
/// counts are for.
const AREA_LAND: i32 = 1531;

/// Where the first area's top-left cell lies on the map. Places below are
/// given in the first area's cells (as they were before the map grew north),
/// and details are scattered by those cells, so the first area stays as it
/// was drawn.
const ORIGIN: (i32, i32) = (32, 88);
/// Where the castaway washes up (a cell on the lower beach; the nearest
/// clear spot to it is used).
const LANDING: (i32, i32) = (32, 19);
/// The river: winding from the forest (where the stream out of the pool
/// becomes a river) down into the cove (cells).
const RIVER_SOURCE: (f32, f32) = (26.9, -4.0);
const RIVER_MOUTH: (f32, f32) = (17.92, 29.14);
/// The stream rises on the high ground beyond the north edge and falls
/// over the cliff into a pool (centre and radius, cells).
const SPRING: (f32, f32) = (POOL.0, -42.0);
/// The grassy slope where the west end of the north cliff, slanting down
/// to the south-east, gives way: the rows of the cliff (first area's
/// cells) that are slope instead, ground and high ground meeting there in
/// a hillside.
const WEST_SLOPE_ROWS: (i32, i32) = (-21, -13);
/// How far (cells) the slope's foot runs out onto the ground below, and
/// its top onto the high ground: a gentle hillside.
const SLOPE_RUN_OUT: i32 = 4;
const SLOPE_RUN_UP: i32 = 3;
/// Over how many cells the cliff east of the slope's foot rises to its
/// full height, a cell of face at a time.
const SLOPE_CLIFF_RISE: i32 = 2;
/// How far (cells) around the slope nothing grows, so its way stays open.
const SLOPE_CLEARING: i32 = 2;

/// How much wider each side the stream's mouth is where it leaves the
/// lake, and over how many cells it narrows to the stream.
const MOUTH_FLARE: f32 = 2.2;
const MOUTH_FLARE_REACH: f32 = 4.0;
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
/// Its left cell is in the first area's cells (see `ORIGIN`).
const CLIFF_HEIGHT: i32 = 3;
const LOW_CLIFF: (i32, i32) = (62, 3);
/// The low stretch's left cell on the map.
const LOW_CLIFF_X: i32 = LOW_CLIFF.0 + ORIGIN.0;
const LOW_CLIFF_HEIGHT: i32 = 2;
/// How far either side of the low stretch the cliff's edge is drawn
/// straight (cells), so the recess stands out from the rock around it.
const LOW_CLIFF_STRAIGHT: i32 = 3;
/// Radius (cells) of the clearing in the forest in front of the recess.
const LOW_CLIFF_CLEARING: f32 = 3.7;
/// Radius (cells, across the river) of the clearing on the banks at the
/// narrow, so the narrowing reads from afar.
const BRIDGE_CLEARING: f32 = 6.0;
/// How much flatter than round the clearing at the narrow is (its height
/// is the radius divided by this), keeping the bushes north of it.
const BRIDGE_CLEARING_SQUASH: f32 = 1.3;
/// The row (first area's cells) where the water mill is to stand, on the
/// river's west bank below the pool, where the stream runs out of the pool
/// straight and fast.
const MILL_ROW: i32 = 0;
/// The clearing kept for the mill and its house, west of the bank: round
/// patches of open ground, each its middle (from the mill site) and radius
/// (cells). The second carries it south-west, through the bushes there,
/// toward the open banks at the narrow.
const MILL_CLEARING: [((f32, f32), f32); 2] = [((-3.0, 0.0), 6.5), ((-7.0, 7.0), 4.5)];

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
        'B' => 11.0,
        '#' => 24.0,
        _ => panic!("unknown character {c:?} in the island plan"),
    }
}

/// Height of a character of the high ground's layer: the lake sits in it
/// as the sea does below, its shallows and shore drawn the same way.
fn upper_height(c: char) -> f32 {
    match c {
        'W' => -8.0,
        'w' => -2.5,
        's' => 3.5,
        _ => 12.0,
    }
}

/// Characters of the high ground's layer.
fn is_upper(c: char) -> bool {
    matches!(c, '^' | 'W' | 'w' | 's' | 'R' | 'b')
}

/// The plan read into blocks: the height of the land, and which blocks
/// are high ground (and its height there), forest, bamboo grove and rocks.
struct Plan {
    w: i32,
    h: i32,
    heights: Vec<f32>,
    high: Vec<f32>,
    upper: Vec<f32>,
    rocks: Vec<f32>,
    forest: Vec<f32>,
    bamboo: Vec<f32>,
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
        let mut left = low.iter().filter(|&&c| is_upper(c)).count();
        while left > 0 {
            let before = low.clone();
            for i in 0..low.len() {
                if !is_upper(before[i]) {
                    continue;
                }
                let (x, y) = (i as i32 % w, i as i32 / w);
                // South first: a cliff facing the sea stands in the sea.
                let near = [(0, 1), (-1, 0), (1, 0), (0, -1)]
                    .iter()
                    .map(|&(dx, dy)| (x + dx, y + dy))
                    .filter(|&(nx, ny)| (0..w).contains(&nx) && (0..h).contains(&ny))
                    .map(|(nx, ny)| before[(ny * w + nx) as usize])
                    .find(|&c| !is_upper(c));
                if let Some(c) = near {
                    low[i] = c;
                    left -= 1;
                }
            }
            assert!(low != before, "the island plan is all high ground");
        }
        let mark = |want: char| chars.iter().map(|&c| if c == want { 1.0 } else { 0.0 }).collect();
        Self {
            w,
            h,
            heights: low.iter().map(|&c| plan_height(c)).collect(),
            high: chars.iter().map(|&c| if is_upper(c) { 1.0 } else { 0.0 }).collect(),
            upper: chars.iter().map(|&c| upper_height(c)).collect(),
            rocks: mark('R'),
            forest: mark('T'),
            bamboo: chars.iter().map(|&c| if matches!(c, 'B' | 'b') { 1.0 } else { 0.0 }).collect(),
        }
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

    /// Height of the high ground's layer at a cell (below 0 is the lake),
    /// roughened like the land below.
    fn upper_at(&self, x: i32, y: i32) -> f32 {
        let base = self.plan.smooth(&self.plan.upper, x as f32, y as f32);
        let (nx, ny) = ((x - ORIGIN.0) as f32, (y - ORIGIN.1) as f32);
        base + fbm(nx * 0.11, ny * 0.11, self.seed ^ 0x1A4E, 3) * 3.0
    }

    /// Whether a cell is in the rocks closing off the high ground.
    fn is_rocks(&self, x: i32, y: i32) -> bool {
        let (nx, ny) = ((x - ORIGIN.0) as f32, (y - ORIGIN.1) as f32);
        self.plan.smooth(&self.plan.rocks, x as f32, y as f32) + fbm(nx * 0.21, ny * 0.21, self.seed ^ 0x40C5, 2) * 0.35 > 0.5
    }

    /// Whether a cell is in a bamboo grove (its edge roughened).
    fn is_bamboo(&self, x: i32, y: i32) -> bool {
        let (nx, ny) = ((x - ORIGIN.0) as f32, (y - ORIGIN.1) as f32);
        self.plan.smooth(&self.plan.bamboo, x as f32, y as f32) + fbm(nx * 0.21, ny * 0.21, self.seed ^ 0xBA3B, 2) * 0.35 > 0.5
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
    let around = LOW_CLIFF_X - LOW_CLIFF_STRAIGHT..LOW_CLIFF_X + LOW_CLIFF.1 + LOW_CLIFF_STRAIGHT;
    if let Some(edge) = (0..h - 1).find(|&y| high[idx(LOW_CLIFF_X, y)] && !high[idx(LOW_CLIFF_X, y + 1)]) {
        for x in around {
            for y in edge - LOW_CLIFF_STRAIGHT..ORIGIN.1 {
                high[idx(x, y)] = y <= edge;
            }
        }
        // The low stretch's edge is a row lower: its face is shorter, and
        // stands on the same ground as the rest of the cliff.
        for x in LOW_CLIFF_X..LOW_CLIFF_X + LOW_CLIFF.1 {
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
                let low = (LOW_CLIFF_X..LOW_CLIFF_X + LOW_CLIFF.1).contains(&x) && y < ORIGIN.1;
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
            let t = map.terrain(x, y);
            let depth = river.depth_px((x as f32 + 0.5) * ts, (y as f32 + 0.5) * ts);
            // In the lake the stream is lost in the lake's water: it starts
            // at the shore, where the shallows across its mouth run with its
            // current (too strong to wade, as off the sea's river mouth).
            if map.is_high(x, y) && t.is_water() {
                if t == Terrain::ShallowWater && depth > -2.0 {
                    map.set_terrain(x, y, Terrain::River);
                    map.set_riverside(x, y, true);
                }
                continue;
            }
            // Anything within half a cell's diagonal (+ the bank lip) of the water.
            map.set_riverside(x, y, depth > -24.0);
            // On land the banks are cut back to the water; out at sea the
            // current is too strong to wade where it runs out to the deep
            // (every cell it reaches into, as it is drawn to the pixel).
            if (depth > -2.0 && (t.is_land() || t == Terrain::CliffTop)) || (depth > -SEA_CURRENT_REACH && t == Terrain::ShallowWater) {
                map.set_terrain(x, y, Terrain::River);
            }
        }
    }
}

/// Where the water mill is to stand: the ground just west of the river on
/// `MILL_ROW`.
fn mill_site(map: &WorldMap) -> Option<(i32, i32)> {
    let y = at((0, MILL_ROW)).1;
    let river = (0..map.width()).find(|&x| map.terrain(x, y) == Terrain::River)?;
    Some((river - 1, y))
}

/// Picks a bank of the pool under the falls, on the grass of the clearing
/// around it, where water can be drawn: on the east bank (toward the
/// south-east first), or with `west` on the west bank beyond the river
/// (toward the south-west first).
fn water_spot(map: &WorldMap, river: &River, west: bool) -> Option<(i32, i32)> {
    let ts = TILE_SIZE as f32;
    let ((cx, cy), r) = river.pool()?;
    let (start, side) = if west { (3.0 * std::f32::consts::FRAC_PI_4, -1.0) } else { (std::f32::consts::FRAC_PI_4, 1.0) };
    for i in 0..32 {
        // Swing out either way from the start.
        let turn = (i / 2) as f32 * 0.2 * if i % 2 == 0 { 1.0 } else { -1.0 };
        let a = start + turn;
        let (bx, by) = (cx + a.cos() * (r + 0.45), cy + a.sin() * (r + 0.45));
        let (gx, gy) = (bx.round() as i32, by.round() as i32);
        let clear = (-1..=1).all(|dy| (-1..=1).all(|dx| map.terrain(gx + dx, gy + dy) != Terrain::Cliff));
        let this_side = (bx - cx) * side > 0.5;
        if this_side && map.terrain(gx, gy) == Terrain::Grass && clear && river.nearest(bx, by).depth() < -0.3 {
            return Some((((bx + 0.5) * ts) as i32, ((by + 0.5) * ts) as i32));
        }
    }
    None
}

/// Fills the lake on the high ground: where the layer's height dips below
/// the waterline there is deep water, shallows and a shore, ordered as by
/// the sea (deep water never meets dry ground). Their heights are kept,
/// so they are drawn as the sea's are.
fn fill_lake(map: &mut WorldMap, field: &HeightField, high: &[bool], heights: &mut [f32]) -> Option<i32> {
    let w = map.width();
    let mut lake = Vec::new();
    for y in 0..map.height() {
        for x in 0..w {
            if map.terrain(x, y) != Terrain::Highland {
                continue;
            }
            let mut e = field.upper_at(x, y);
            // In the rocks the lake is deep right up to them.
            if e < 0.0 && (-1..=1).any(|d| field.is_rocks(x + d, y) || field.is_rocks(x, y + d)) {
                e = e.min(DEEP_BELOW - 0.5);
            }
            heights[(y * w + x) as usize] = e;
        }
    }
    // Where the stream leaves the lake: the shore below its source.
    let (sx, sy) = at_f(SPRING);
    let col = sx.floor() as i32;
    let mouth = (sy as i32..map.height()).find(|&y| heights[(y * w + col) as usize] >= 0.0);
    for y in 0..map.height() {
        for x in 0..w {
            if map.terrain(x, y) != Terrain::Highland {
                continue;
            }
            let e = heights[(y * w + x) as usize];
            if e < SAND_BELOW {
                lake.push((x, y, classify(e)));
            }
        }
    }
    // One lake: puddles cut off from it by the noise are left dry.
    let wet: std::collections::HashSet<(i32, i32)> = lake.iter().filter(|l| l.2.is_water()).map(|&(x, y, _)| (x, y)).collect();
    let mut body: std::collections::HashSet<(i32, i32)> = std::collections::HashSet::new();
    for &(x, y, _) in lake.iter().filter(|l| l.2.is_water()) {
        if body.contains(&(x, y)) {
            continue;
        }
        let mut part = vec![(x, y)];
        let mut seen: std::collections::HashSet<(i32, i32)> = part.iter().copied().collect();
        let mut i = 0;
        while i < part.len() {
            let (cx, cy) = part[i];
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let n = (cx + dx, cy + dy);
                if wet.contains(&n) && seen.insert(n) {
                    part.push(n);
                }
            }
            i += 1;
        }
        if part.len() > body.len() {
            body = seen;
        }
    }
    for &(x, y, t) in &lake {
        if !t.is_water() || body.contains(&(x, y)) {
            map.set_terrain(x, y, t);
        } else {
            // A puddle's bed is the shore's sand.
            map.set_terrain(x, y, Terrain::Sand);
            heights[(y * w + x) as usize] = SAND_BELOW - 2.0;
        }
    }
    let snapshot = map.clone();
    let near = |x: i32, y: i32, pred: &dyn Fn(Terrain) -> bool| {
        (-1..=1).any(|dy| (-1..=1).any(|dx| (dx, dy) != (0, 0) && snapshot.in_bounds(x + dx, y + dy) && pred(snapshot.terrain(x + dx, y + dy))))
    };
    for y in 0..map.height() {
        for x in 0..w {
            if !high[(y * w + x) as usize] {
                continue;
            }
            // Against the rocks the water stays deep: no shallows to wade
            // around the lake's ends.
            let shore = |x: i32, y: i32| {
                (-1..=1).any(|dy| {
                    (-1..=1).any(|dx| {
                        let (nx, ny) = (x + dx, y + dy);
                        snapshot.in_bounds(nx, ny) && matches!(snapshot.terrain(nx, ny), Terrain::Highland | Terrain::Sand | Terrain::CliffTop) && !field.is_rocks(nx, ny)
                    })
                })
            };
            let at_rocks = |x: i32, y: i32| (-2..=2).any(|dy| (-2..=2).any(|dx| field.is_rocks(x + dx, y + dy)));
            match snapshot.terrain(x, y) {
                Terrain::DeepWater if shore(x, y) && !at_rocks(x, y) => map.set_terrain(x, y, Terrain::ShallowWater),
                Terrain::Highland if near(x, y, &|t| t.is_water()) => {
                    map.set_terrain(x, y, Terrain::Sand);
                    let e = &mut heights[(y * w + x) as usize];
                    *e = e.min(SAND_BELOW - 0.8);
                }
                _ => {}
            }
        }
    }
    // The shore and the bed are wet soil, not the sea's sand.
    for y in 0..map.height() {
        for x in 0..w {
            if high[(y * w + x) as usize] && matches!(map.terrain(x, y), Terrain::Sand | Terrain::ShallowWater | Terrain::DeepWater) {
                map.set_soil(x, y, Soil::Mud);
            }
        }
    }
    mouth
}

/// Cuts the slope up through the west cliff's side.
fn cut_slope(map: &mut WorldMap) {
    let (top, bottom) = (WEST_SLOPE_ROWS.0 + ORIGIN.1, WEST_SLOPE_ROWS.1 + ORIGIN.1);
    let mut cells = Vec::new();
    let mut foot = None;
    for y in top..=bottom {
        // The cliff across the row, coming from the west, up to the high
        // ground.
        let Some(start) = (0..map.width()).find(|&x| map.terrain(x, y).is_cliff()) else { continue };
        let end = (start..map.width()).find(|&x| !map.terrain(x, y).is_cliff()).unwrap_or(start);
        // Its foot runs out a little onto the ground below, so it comes
        // down gently.
        let run = (1..=SLOPE_RUN_OUT).take_while(|&k| map.terrain(start - k, y) == Terrain::Grass).count() as i32;
        let up = (0..SLOPE_RUN_UP).take_while(|&k| map.terrain(end + k, y) == Terrain::Highland).count() as i32;
        cells.extend((start - run..end + up).map(|x| (x, y)));
        if y == (top + bottom) / 2 {
            foot = Some((start - run - 1, y));
        }
    }
    // East of its lowest row the cliff carries on: its west end rises out
    // of the slope a cell at a time instead of standing up at once.
    if let Some(&(_, y)) = cells.last() {
        let below = y + 1;
        let from = cells.iter().filter(|c| c.1 == y).map(|c| c.0).min().unwrap_or(0);
        if let Some(x0) = (from..map.width()).find(|&x| map.terrain(x, below) == Terrain::Cliff) {
            for k in 0..SLOPE_CLIFF_RISE {
                let x = x0 + k;
                let face = (below..map.height()).take_while(|&fy| map.terrain(x, fy) == Terrain::Cliff).count() as i32;
                let keep = k + 1;
                cells.extend((below..below + (face - keep).max(0)).map(|fy| (x, fy)));
            }
        }
    }
    for &(x, y) in &cells {
        map.set_terrain(x, y, Terrain::Slope);
    }
    // Its foot: the ground just west of its middle row.
    map.spots.slope = foot;
    let ts = TILE_SIZE;
    let low = cells.iter().map(|&(x, y)| x * ts - (y * ts + ts - 1)).min();
    let high = cells.iter().map(|&(x, y)| (x * ts + ts - 1) - y * ts).max();
    map.spots.slope_span = low.zip(high);
}

/// Whether a cell is by the slope (kept clear).
fn by_slope(map: &WorldMap, x: i32, y: i32) -> bool {
    (-SLOPE_CLEARING..=SLOPE_CLEARING).any(|dy| (-SLOPE_CLEARING..=SLOPE_CLEARING).any(|dx| map.terrain(x + dx, y + dy) == Terrain::Slope))
}

/// How many cells of rock face drop from a crag's southern edge.
const CRAG_FACE: i32 = 3;

/// How far in from a crag's edge (cells) its top rises another tier, and
/// the most tiers it rises: the middle of a crag stands highest.
pub const CRAG_TIER_CELLS: i32 = 4;
const CRAG_TIERS: u8 = 5;

/// Heaps the rocks that close off the high ground into crags: bare rock,
/// with a face of `CRAG_FACE` cells below its southern edge, like the
/// cliffs', so it reads at a glance as something that can't be climbed.
fn raise_crags(map: &mut WorldMap, field: &HeightField) {
    let (w, h) = (map.width(), map.height());
    let crag = |map: &WorldMap, x: i32, y: i32| map.is_high(x, y) && field.is_rocks(x, y) && matches!(map.terrain(x, y), Terrain::Highland | Terrain::Sand);
    let cells: Vec<(i32, i32)> = (0..h).flat_map(|y| (0..w).map(move |x| (x, y))).filter(|&(x, y)| crag(map, x, y)).collect();
    for &(x, y) in &cells {
        map.set_terrain(x, y, Terrain::Crag);
    }
    // Tiers by how far each cell is from the crag's edge (the map's edge
    // counted as one, so its peak stands in sight), measured round
    // (diagonal steps count as 1.4), so the tiers' edges curve with the
    // crag's.
    let mut dist = vec![f32::MAX; (w * h) as usize];
    for y in 0..h {
        for x in 0..w {
            let edge = map.terrain(x, y) == Terrain::Crag
                && [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().any(|&(dx, dy)| !map.in_bounds(x + dx, y + dy) || map.terrain(x + dx, y + dy) != Terrain::Crag);
            if edge {
                dist[(y * w + x) as usize] = 0.0;
            }
        }
    }
    let steps = [(-1, 0, 1.0), (0, -1, 1.0), (-1, -1, 1.414), (1, -1, 1.414)];
    for pass in 0..2 {
        let order: Vec<(i32, i32)> = if pass == 0 {
            (0..h).flat_map(|y| (0..w).map(move |x| (x, y))).collect()
        } else {
            (0..h).rev().flat_map(|y| (0..w).rev().map(move |x| (x, y))).collect()
        };
        for (x, y) in order {
            if map.terrain(x, y) != Terrain::Crag {
                continue;
            }
            let i = (y * w + x) as usize;
            for &(dx, dy, c) in &steps {
                let (nx, ny) = if pass == 0 { (x + dx, y + dy) } else { (x - dx, y - dy) };
                if map.in_bounds(nx, ny) && map.terrain(nx, ny) == Terrain::Crag {
                    dist[i] = dist[i].min(dist[(ny * w + nx) as usize] + c);
                }
            }
        }
    }
    // The tiers' edges waver, like a hill's contours, not the plan's
    // blocks.
    for &(x, y) in &cells {
        let d = dist[(y * w + x) as usize];
        if d < f32::MAX {
            let (nx, ny) = ((x - ORIGIN.0) as f32, (y - ORIGIN.1) as f32);
            let waver = fbm(nx * 0.07, ny * 0.07, field.seed ^ 0x7E7A, 2) * CRAG_TIER_CELLS as f32 * 1.4;
            let d = (d + waver.min(d)).max(0.0);
            map.set_rise(x, y, ((d / CRAG_TIER_CELLS as f32) as u8).min(CRAG_TIERS));
        }
    }
    for &(x, y) in &cells {
        if map.terrain(x, y + 1) == Terrain::Crag {
            continue;
        }
        // Where the crag stands right at the high ground's own edge, there's
        // no ground below it for its face: its foot is cut into a face
        // instead, so it still rises from the high ground (rather than its
        // top sitting flat on the cliff's edge).
        let (top, bottom) = if map.terrain(x, y + 1) == Terrain::CliffTop { (y - CRAG_FACE + 1, y) } else { (y + 1, y + CRAG_FACE) };
        for fy in top..=bottom {
            if map.is_high(x, fy) && matches!(map.terrain(x, fy), Terrain::Crag | Terrain::Highland | Terrain::Sand | Terrain::ShallowWater | Terrain::DeepWater) {
                map.set_terrain(x, fy, Terrain::Cliff);
            }
        }
    }
}

/// Picks a bank of the river up on the high ground, between the lake and
/// the falls, where water can be drawn: on open ground beside the stream
/// partway down it.
fn upper_water_spot(map: &WorldMap, river: &River) -> Option<(i32, i32)> {
    let ts = TILE_SIZE as f32;
    let (sx, sy) = at_f(SPRING);
    let ((_, py), _) = river.pool()?;
    // Down from the middle of the stream's run, either bank.
    let mid = (sy + py) / 2.0;
    for k in 0..40 {
        let y = mid + (k / 2) as f32 * if k % 2 == 0 { 1.0 } else { -1.0 };
        for side in [1.0f32, -1.0] {
            let (bx, by) = (sx + side * 1.6, y);
            let (gx, gy) = (bx.round() as i32, by.round() as i32);
            let clear = (-1..=1).all(|dy| (-1..=1).all(|dx| map.terrain(gx + dx, gy + dy) != Terrain::CliffTop && map.feature(gx + dx, gy + dy) == Feature::None));
            if map.terrain(gx, gy) == Terrain::Highland && clear && river.nearest(bx, by).depth() < -0.3 {
                return Some((((bx + 0.5) * ts) as i32, ((by + 0.5) * ts) as i32));
            }
        }
    }
    None
}

/// Whether a cell of the high ground is by the lake's shore or water.
fn by_lake(map: &WorldMap, x: i32, y: i32) -> bool {
    (-1..=1).any(|dy| {
        (-1..=1).any(|dx| map.is_high(x + dx, y + dy) && matches!(map.terrain(x + dx, y + dy), Terrain::Sand | Terrain::ShallowWater | Terrain::DeepWater))
    })
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
    let high = raise_high_ground(&mut map, &field);
    let mouth = fill_lake(&mut map, &field, &high, &mut heights);
    map.set_high(high);
    let below = |x: f32, y: f32, level: f32| {
        let (cx, cy) = (x.round() as i32, y.round() as i32);
        !map.in_bounds(cx, cy) || heights[(cy * w + cx) as usize] < level
    };
    let sea = |x: f32, y: f32| below(x, y, 0.0);
    let deep = |x: f32, y: f32| below(x, y, DEEP_BELOW);
    // The stream starts at the lake's shore, its mouth flared so the
    // lake's water gathers into it from either side.
    let spring = mouth.map_or(at_f(SPRING), |m| (at_f(SPRING).0, m as f32 - 1.0));
    let river = River::new(at_f(RIVER_SOURCE), at_f(RIVER_MOUTH), seed)
        .with_head(spring, at_f(POOL), POOL_RADIUS)
        .flared(MOUTH_FLARE, MOUTH_FLARE_REACH)
        .narrowed(at_f(BRIDGE_AT), BRIDGE_HALF_WIDTH, 2.5)
        .with_outflow(OUTFLOW_MAX, OUTFLOW_SPREAD, sea, deep);
    lay_river(&mut map, &river);
    map.water_spots = water_spot(&map, &river, false).into_iter().chain(upper_water_spot(&map, &river)).chain(water_spot(&map, &river, true)).collect();
    map.spots = Spots {
        bridge: Some(nearest_on_river(&river, at_f(BRIDGE_AT))),
        ladder: low_cliff_foot(&map),
        falls: river.pool().map(|((x, y), _)| (x.round() as i32, y.round() as i32)),
        mill: mill_site(&map),
        slope: None,
        slope_span: None,
    };
    map.river = Some(river);

    let spawn_cell = find_spawn(&map, &heights);
    map.spawn = (spawn_cell.0 * TILE_SIZE, spawn_cell.1 * TILE_SIZE);
    // Nothing grows close to the spawn, and the forest opens out in front
    // of the recess in the cliff, drawing the eye to it.
    let clearing = map.spots.ladder.map(|(x, y)| (x as f32, y as f32 + 2.0));
    let in_clearing = |x: i32, y: i32| clearing.is_some_and(|(cx, cy)| (x as f32 - cx).hypot(y as f32 - cy) < LOW_CLIFF_CLEARING);
    // The banks at the narrow are open too, so the river's narrowing shows.
    let narrow = map.spots.bridge;
    let at_narrow = |x: i32, y: i32| {
        narrow.is_some_and(|(nx, ny)| ((x - nx) as f32).hypot((y - ny) as f32 * BRIDGE_CLEARING_SQUASH) < BRIDGE_CLEARING)
    };
    // So is the ground kept for the mill, west of the bank it stands on.
    let mill = map.spots.mill;
    let at_mill = |x: i32, y: i32| {
        mill.is_some_and(|(mx, my)| MILL_CLEARING.iter().any(|&((dx, dy), r)| (x as f32 - (mx as f32 + dx)).hypot(y as f32 - (my as f32 + dy)) < r))
    };
    raise_crags(&mut map, &field);
    cut_slope(&mut map);
    // The way up the slope is kept clear.
    let slope_clear: Vec<bool> = (0..h).flat_map(|y| (0..w).map(move |x| (x, y))).map(|(x, y)| by_slope(&map, x, y)).collect();
    let near_spawn = |x: i32, y: i32| {
        (x - spawn_cell.0).abs() <= 3 && (y - spawn_cell.1).abs() <= 3 || in_clearing(x, y) || at_narrow(x, y) || at_mill(x, y) || ((0..w).contains(&x) && (0..h).contains(&y) && slope_clear[(y * w + x) as usize])
    };
    let forest: Vec<f32> = (0..h).flat_map(|y| (0..w).map(move |x| (x, y))).map(|(x, y)| field.forest(x, y)).collect();

    place_pine(&mut map);
    let bamboo: Vec<bool> = (0..h).flat_map(|y| (0..w).map(move |x| (x, y))).map(|(x, y)| field.is_bamboo(x, y)).collect();
    place_bamboo(&mut map, &heights, &bamboo, seed);
    place_palms(&mut map, &heights, seed, &near_spawn);
    // No trees in the bamboo groves: the bamboo has them to itself.
    let in_grove = |x: i32, y: i32| near_spawn(x, y) || bamboo[(y * w + x) as usize];
    place_broadleafs(&mut map, &heights, &forest, seed, &in_grove);
    place_scatter(&mut map, &heights, &forest, seed, &in_grove);
    drop_pine_cones(&mut map);
    fill_cliff_feet(&mut map);
    map.set_elevation(heights.clone());
    map.crab_spawns = pick_crab_spawns(&map, seed, spawn_cell);
    map.loot = scatter_loot(&map, seed);
    // Last of all, so nothing else on the island moves for them.
    place_clay_beds(&mut map, seed);
    place_bare_earth(&mut map, &heights, seed, &in_grove);
    map
}

/// Clay beds lying in the mud of the lake's south shore.
const CLAY_BEDS: usize = 5;
/// How far either side of the ladder's column the shore north of it is
/// left open (the beach a raft would be launched from), and how far out
/// either side the clay beds lie.
pub const LAUNCH_HALF_WIDTH: i32 = 3;
const CLAY_REACH: i32 = 14;
/// Least distance (cells, either way) between the top-left cells of two
/// clay beds, and of two patches of bare earth.
const CLAY_SPACING: i32 = 3;
const EARTH_SPACING: i32 = 14;
/// Patches of bare earth in the grass, for each stretch of dry land the
/// size of the first area's: below the cliffs, and on the high ground.
const EARTH_PATCHES: usize = 3;
const HIGH_EARTH_PATCHES: usize = 2;

/// Picks up to `count` two-by-two patches from the top-left cells `fits`
/// allows, in an order shuffled by `seed`, no two closer than `spacing`
/// (nor to any of `taken`).
fn pick_patches(map: &WorldMap, seed: u32, count: usize, spacing: i32, taken: &[(i32, i32)], fits: impl Fn(i32, i32) -> bool) -> Vec<(i32, i32)> {
    let w = map.width();
    let mut spots: Vec<(i32, i32)> = (0..map.height() - 1).flat_map(|y| (0..w - 1).map(move |x| (x, y))).filter(|&(x, y)| fits(x, y)).collect();
    spots.sort_by(|&a, &b| roll(a.0, a.1, seed).total_cmp(&roll(b.0, b.1, seed)));
    let mut out: Vec<(i32, i32)> = Vec::new();
    for (x, y) in spots {
        if out.len() >= count {
            break;
        }
        if out.iter().chain(taken).all(|&(px, py)| (px - x).abs().max((py - y).abs()) >= spacing) {
            out.push((x, y));
        }
    }
    out
}

/// The cells of a patch whose top-left is at `(x, y)`, and the ring of
/// cells round it.
fn patch_and_ring(x: i32, y: i32) -> impl Iterator<Item = (i32, i32)> {
    (y - 1..=y + 2).flat_map(move |cy| (x - 1..=x + 2).map(move |cx| (cx, cy)))
}

/// Lays clay beds in the mud of the lake's south shore, either side of
/// the beach north of the ladder (left open), each with mud all round it.
fn place_clay_beds(map: &mut WorldMap, seed: u32) {
    let Some((lx, ly)) = map.spots.ladder else { return };
    let mud = |x: i32, y: i32| map.in_bounds(x, y) && map.is_high(x, y) && map.terrain(x, y) == Terrain::Sand && map.soil(x, y) == Soil::Mud && map.feature(x, y) == Feature::None;
    let beds = pick_patches(map, seed ^ 0xC1A7, CLAY_BEDS, CLAY_SPACING, &[], |x, y| {
        let off = [x - lx, x + 1 - lx];
        y < ly && off.iter().all(|d| d.abs() > LAUNCH_HALF_WIDTH && d.abs() <= CLAY_REACH) && patch_and_ring(x, y).all(|(cx, cy)| mud(cx, cy))
    });
    for anchor in beds {
        for (part, (x, y)) in patch_cells(anchor).into_iter().enumerate() {
            map.set_feature(x, y, Feature::ClayBed(ClayPart { part: part as u8, dug: false }));
        }
    }
}

/// Lays patches of bare earth in open grass, below the cliffs and on the
/// high ground, well apart: clear of trees and everything else, the
/// riverside, the lake, the clearings kept open (`kept`) and the branches
/// and stones lying about.
fn place_bare_earth(map: &mut WorldMap, heights: &[f32], seed: u32, kept: &dyn Fn(i32, i32) -> bool) {
    let w = map.width();
    let loot: Vec<(i32, i32)> = map.loot.iter().map(|&(_, (px, py))| (px.div_euclid(TILE_SIZE), py.div_euclid(TILE_SIZE))).collect();
    let open = |x: i32, y: i32, high: bool| {
        map.in_bounds(x, y)
            && map.feature(x, y) == Feature::None
            && map.is_high(x, y) == high
            && map.terrain(x, y) == if high { Terrain::Highland } else { Terrain::Grass }
            && heights[(y * w + x) as usize] > SAND_BELOW + PROP_MARGIN
            && !map.is_riverside(x, y)
            && map.foot_fill(x, y) == [false, false]
            && !kept(x, y)
            && !(high && by_lake(map, x, y))
            && !loot.contains(&(x, y))
    };
    let low = pick_patches(map, seed ^ 0xEA27, for_area(map, EARTH_PATCHES), EARTH_SPACING, &[], |x, y| patch_and_ring(x, y).all(|(cx, cy)| open(cx, cy, false)));
    let high = pick_patches(map, seed ^ 0xEA28, for_high_area(map, HIGH_EARTH_PATCHES), EARTH_SPACING, &low, |x, y| patch_and_ring(x, y).all(|(cx, cy)| open(cx, cy, true)));
    for anchor in low.into_iter().chain(high) {
        for (part, (x, y)) in patch_cells(anchor).into_iter().enumerate() {
            map.set_feature(x, y, Feature::BareEarth(part as u8));
        }
    }
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
    let x = LOW_CLIFF_X + LOW_CLIFF.1 / 2;
    (0..ORIGIN.1).find(|&y| map.terrain(x, y) == Terrain::Cliff && map.terrain(x, y + 1) != Terrain::Cliff).map(|y| (x, y + 1))
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
/// How many of something for the high ground's open ground, at
/// `per_area` for each stretch of it the size of the first area's dry land.
pub fn for_high_area(map: &WorldMap, per_area: usize) -> usize {
    let land = (0..map.height()).flat_map(|y| (0..map.width()).map(move |x| (x, y))).filter(|&(x, y)| map.terrain(x, y) == Terrain::Highland).count();
    per_area * land / AREA_LAND as usize
}

pub fn for_area(map: &WorldMap, per_area: usize) -> usize {
    let land = (0..map.height()).flat_map(|y| (0..map.width()).map(move |x| (x, y))).filter(|&(x, y)| !map.is_high(x, y) && matches!(map.terrain(x, y), Terrain::Sand | Terrain::Grass)).count();
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

/// How often a palm takes a free spot on the upper beach; on the west
/// coast's narrow beach (beyond the river), more often.
const PALMS: f32 = 0.16;
const WEST_PALMS: f32 = 0.5;

/// Each row's westmost river cell: the land west of it lies beyond the
/// river, on the west coast.
fn river_from(map: &WorldMap) -> Vec<Option<i32>> {
    (0..map.height()).map(|y| (0..map.width()).find(|&x| map.terrain(x, y) == Terrain::River)).collect()
}

fn place_palms(map: &mut WorldMap, heights: &[f32], seed: u32, near_spawn: &dyn Fn(i32, i32) -> bool) {
    let (pw, ph) = (PalmPart::W, PalmPart::H);
    let river_from = river_from(map);
    for y in 0..map.height() - ph + 1 {
        for x in 0..map.width() - pw + 1 {
            // Anchor on the trunk base (bottom row), which sits on the upper beach.
            let h = heights[((y + ph - 1) * map.width() + x) as usize];
            let west = river_from[(y + ph - 1) as usize].is_some_and(|rx| x < rx);
            let share = if west { WEST_PALMS } else { PALMS };
            if !(2.0..SAND_BELOW).contains(&h) || roll(x, y, seed ^ 0xFA1A) > share {
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
                    ground_ok && map.feature(cx, cy) == Feature::None && !near_spawn(cx, cy) && !map.is_riverside(cx, cy) && !map.is_high(cx, cy)
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
/// How often a culm takes a free spot in a grove: thick enough to read as
/// a grove, with paths left among the stalks.
const BAMBOO_DENSITY: f32 = 0.6;

/// Bamboo culms standing in the groves drawn on the plan: on grass, away
/// from the river's banks, each wholly inside the grove, and not right
/// beside another (a cell between neighbours on the same rows).
fn place_bamboo(map: &mut WorldMap, heights: &[f32], grove: &[bool], seed: u32) {
    let th = BambooPart::H;
    let w = map.width();
    for y in 0..map.height() - th + 1 {
        for x in 1..w - 1 {
            if roll(x, y, seed ^ 0xBA30) > BAMBOO_DENSITY {
                continue;
            }
            let fits = (0..th).all(|dy| {
                let (cx, cy) = (x, y + dy);
                let i = (cy * w + cx) as usize;
                grove[i]
                    && matches!(map.terrain(cx, cy), Terrain::Grass | Terrain::Highland | Terrain::Slope)
                    && heights[i] > SAND_BELOW + PROP_MARGIN
                    && map.feature(cx, cy) == Feature::None
                    && !map.is_riverside(cx, cy)
                    && !matches!(map.feature(cx - 1, cy), Feature::Bamboo(_))
                    && !matches!(map.feature(cx + 1, cy), Feature::Bamboo(_))
            });
            if fits {
                let flip = roll(x, y, seed ^ 0xBA31) < 0.5;
                let v = (roll(x, y, seed ^ 0xBA32) * BambooPart::SHAPES as f32) as u8 % BambooPart::SHAPES;
                for dy in 0..th {
                    map.set_feature(x, y + dy, Feature::Bamboo(BambooPart { dy: dy as u8, v, flip }));
                }
            }
        }
    }
}

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
                    let ground_ok = if ground == Terrain::Highland { on == Terrain::Highland && !by_lake(map, cx, cy) } else { on == Terrain::Grass && h > SAND_BELOW + PROP_MARGIN };
                    ground_ok
                        && map.feature(cx, cy) == Feature::None
                        && !near_spawn(cx, cy)
                        && !map.is_riverside(cx, cy)
                })
            });
            let crowded = (-1..=th).any(|dy| {
                (-1..=tw).any(|dx| matches!(map.feature(x + dx, y + dy), Feature::Palm(_) | Feature::Broadleaf(_) | Feature::Pine(_) | Feature::Bamboo(_)))
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
    let river_from = river_from(map);
    let west_coast = |map: &WorldMap, x: i32, y: i32| !map.is_high(x, y) && river_from[y as usize].is_some_and(|rx| x < rx);
    let by_tree = |map: &WorldMap, x: i32, y: i32| (-2..=2).any(|dy| (-2..=2).any(|dx| matches!(map.feature(x + dx, y + dy), Feature::Pine(_) | Feature::Broadleaf(_))));
    let by_bamboo = |map: &WorldMap, x: i32, y: i32| (-1..=1).any(|dy| (-1..=1).any(|dx| matches!(map.feature(x + dx, y + dy), Feature::Bamboo(_))));
    for y in 0..map.height() {
        for x in 0..map.width() {
            if !open(map, x, y) {
                continue;
            }
            let h = heights[(y * map.width() + x) as usize];
            let r = roll(x, y, seed ^ 0xF00D);
            let feature = match map.terrain(x, y) {
                // The grass meeting the lake's shore is left bare, so it's
                // drawn running down to the soil.
                Terrain::Highland if by_lake(map, x, y) => Feature::None,
                // Undergrowth on the high ground: thicker than in the forest
                // below, but with room to walk.
                Terrain::Highland => {
                    let by_tree = by_tree(map, x, y);
                    if r < 0.15 {
                        Feature::Bush
                    } else if r < 0.2 {
                        Feature::Flower
                    } else if by_tree && r > 1.0 - HIGH_VINES {
                        // Climbers on the trees.
                        Feature::Vine
                    } else {
                        Feature::None
                    }
                }
                // The thickets on the west coast are thinner, with room to
                // walk among their bushes.
                Terrain::Grass if h > JUNGLE_ABOVE => {
                    let bushes = if west_coast(map, x, y) { WEST_JUNGLE_BUSHES } else { JUNGLE_BUSHES };
                    if r < bushes { Feature::Bush } else if r < bushes + 0.06 { Feature::Flower } else { Feature::None }
                }
                // Climbers thrive at the jungle's edge.
                Terrain::Grass if r > 0.93 && h > JUNGLE_ABOVE - VINE_BELT => Feature::Vine,
                // And on the trees and bamboo of the west coast.
                Terrain::Grass if r > 1.0 - WEST_VINES && west_coast(map, x, y) && (by_tree(map, x, y) || by_bamboo(map, x, y)) => Feature::Vine,
                // Undergrowth in the forest, light enough to walk through.
                Terrain::Grass if forest[(y * map.width() + x) as usize] > 0.5 => {
                    if r < 0.06 { Feature::Bush } else if r < 0.09 { Feature::Flower } else { Feature::None }
                }
                Terrain::Grass => {
                    if r < 0.05 { Feature::Bush } else if r < 0.12 { Feature::Flower } else { Feature::None }
                }
                // The lake's shore: nothing washed up from the sea.
                Terrain::Sand if map.is_high(x, y) => Feature::None,
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
                && !map.is_high(x, y)
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
    // Up on the high ground, under its trees.
    for (kind, count) in [(ItemKind::Branch, for_high_area(map, HIGH_BRANCH_COUNT)), (ItemKind::Stone, for_high_area(map, HIGH_STONE_COUNT))] {
        let mut placed = 0;
        for _ in 0..4000 {
            if placed >= count {
                break;
            }
            let (x, y) = (rng.between(0, map.width() - 1), rng.between(0, map.height() - 1));
            let open = map.terrain(x, y) == Terrain::Highland
                && !map.props(x, y).solid
                && matches!(map.feature(x, y), Feature::None | Feature::Flower);
            let taken = out.iter().any(|&(_, (px, py))| px.div_euclid(TILE_SIZE) == x && py.div_euclid(TILE_SIZE) == y);
            if !open || taken {
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
            && !map.is_high(x, y)
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
    use crate::world::map::{BRIDGE_CELLS, BRIDGE_REACH, LadderFit};

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
        let spot = *walk.map.water_spots.first().expect("the island has a water spot");
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
    fn a_bridge_laid_at_the_narrow_leads_to_the_far_bank() {
        let mut map = island();
        let (bx, by) = map.spots.bridge.expect("the river has a narrow");
        let span = map.bridge_spot(bx, by).expect("a bridge goes across the narrow");
        map.lay_bridge(span);
        let walk = Walk::from_landing(map);
        assert!(walk.can((span.west, span.row)) && walk.can((span.east, span.row)), "both ends of the bridge are reached");
    }

    /// The island with a log bridge laid at the narrow, walked from the
    /// landing.
    fn walk_with_bridge() -> Walk {
        let mut map = island();
        let (bx, by) = map.spots.bridge.expect("the river has a narrow");
        let span = map.bridge_spot(bx, by).expect("a bridge goes across the narrow");
        map.lay_bridge(span);
        Walk::from_landing(map)
    }

    /// High ground beside the top of the slope (the cell east of its top).
    fn above_the_slope(map: &WorldMap) -> (i32, i32) {
        let w = map.width();
        let top = (0..map.height()).flat_map(|y| (0..w).map(move |x| (x, y))).find(|&(x, y)| map.terrain(x, y) == Terrain::Slope).expect("the island has a slope");
        let x = (top.0..w).find(|&x| map.terrain(x, top.1) == Terrain::Highland && !map.props(x, top.1).solid).expect("high ground beside the slope's top");
        (x, top.1)
    }

    #[test]
    fn the_slope_leads_up_from_the_west_coast_to_the_high_ground() {
        let walk = walk_with_bridge();
        assert!(walk.near(walk.map.spots.slope.expect("the island has a slope")), "the slope's foot is reached over the bridge");
        assert!(walk.can(above_the_slope(&walk.map)), "the high ground at its top is reached");
    }

    #[test]
    fn the_slope_cant_be_reached_without_the_bridge() {
        let walk = Walk::from_landing(island());
        assert!(!walk.can(above_the_slope(&walk.map)));
        assert!(!walk.near(walk.map.spots.slope.expect("the island has a slope")));
    }

    #[test]
    fn the_high_ground_west_of_the_river_cant_be_reached_from_the_ladder() {
        // The river runs between the slope's side and the ladder's.
        let walk = walk_with_ladder();
        assert!(!walk.can(above_the_slope(&walk.map)));
    }

    #[test]
    fn branches_and_stones_lie_on_the_high_ground_too() {
        let map = island();
        let up = |kind: ItemKind| map.loot.iter().filter(|&&(k, (x, y))| k == kind && map.is_high(x / TILE_SIZE, y / TILE_SIZE)).count();
        let (branches, stones) = (up(ItemKind::Branch), up(ItemKind::Stone));
        assert!(branches > 0 && stones > 0, "{branches} branches, {stones} stones");
        assert!(stones < branches, "stones are fewer up there, away from beaches and riverbanks");
    }

    #[test]
    fn vines_grow_on_the_high_ground_by_its_trees() {
        let map = island();
        let w = map.width();
        let vines: Vec<(i32, i32)> = (0..map.height()).flat_map(|y| (0..w).map(move |x| (x, y))).filter(|&(x, y)| map.is_high(x, y) && map.feature(x, y) == Feature::Vine).collect();
        assert!(!vines.is_empty());
        let by_tree = |(x, y): (i32, i32)| (-2..=2).any(|dy| (-2..=2).any(|dx| matches!(map.feature(x + dx, y + dy), Feature::Pine(_) | Feature::Broadleaf(_))));
        assert!(vines.iter().all(|&v| by_tree(v)));
    }

    #[test]
    fn a_bridge_goes_only_across_the_narrow() {
        let map = island();
        let (_, by) = map.spots.bridge.expect("the river has a narrow");
        let elsewhere: Vec<_> = (0..map.height())
            .filter(|&y| (y - by).abs() > BRIDGE_REACH)
            .flat_map(|y| (0..map.width()).map(move |x| (x, y)))
            .filter(|&(x, y)| map.terrain(x, y) == Terrain::River && map.bridge_spot(x, y).is_some())
            .collect();
        assert!(elsewhere.is_empty(), "a bridge would go across the river away from the narrow at {elsewhere:?}");
    }

    #[test]
    fn a_bridge_held_anywhere_along_the_narrow_goes_across_its_middle() {
        let map = island();
        let (bx, by) = map.spots.bridge.expect("the river has a narrow");
        for y in by - BRIDGE_REACH..=by + BRIDGE_REACH {
            let x = (bx - BRIDGE_CELLS..=bx + BRIDGE_CELLS).find(|&x| map.terrain(x, y) == Terrain::River).expect("the river runs by the narrow");
            let span = map.bridge_spot(x, y).unwrap_or_else(|| panic!("no bridge held at row {y}"));
            assert_eq!(span.row, by, "held at row {y}");
        }
    }

    #[test]
    fn a_bridge_can_be_held_out_from_along_the_banks_beyond_the_narrow() {
        let map = island();
        let (bx, by) = map.spots.bridge.expect("the river has a narrow");
        let width = |y: i32| (bx - 4..=bx + 4).filter(|&x| map.terrain(x, y) == Terrain::River).count() as i32;
        assert!(
            (by - BRIDGE_REACH..=by + BRIDGE_REACH).any(|y| width(y) > BRIDGE_CELLS),
            "a bridge can be held out only where the river is already narrow enough to span, which is too easy to miss"
        );
    }

    #[test]
    fn nothing_grows_on_the_banks_either_side_of_the_narrow() {
        let map = island();
        let (bx, by) = map.spots.bridge.expect("the river has a narrow");
        let grown: Vec<_> = (by - BRIDGE_REACH..=by + BRIDGE_REACH)
            .flat_map(|y| (bx - 4..=bx + 4).map(move |x| (x, y)))
            .filter(|&(x, y)| map.terrain(x, y) != Terrain::River && map.feature(x, y) != Feature::None)
            .collect();
        assert!(grown.is_empty(), "the banks by the narrow are open ground, but these cells have something on them: {grown:?}");
    }

    #[test]
    fn a_ladder_fits_only_in_the_middle_of_the_low_cliff() {
        let map = island();
        let ((x, y), _) = ladder_face(&map);
        assert_eq!(map.ladder_fit(x - 1, y), LadderFit::No);
        assert_eq!(map.ladder_fit(x + 1, y), LadderFit::No);
    }

    /// The island with a ladder stood at the low cliff, walked from the
    /// landing.
    fn walk_with_ladder() -> Walk {
        let mut map = island();
        let (foot, rows) = ladder_face(&map);
        map.stand_ladder(foot, rows);
        Walk::from_landing(map)
    }

    #[test]
    fn the_lake_shore_can_be_reached_from_the_high_ground() {
        let walk = walk_with_ladder();
        let w = walk.map.width();
        let shore = (0..walk.map.height()).any(|y| (0..w).any(|x| walk.map.is_high(x, y) && walk.map.terrain(x, y) == Terrain::ShallowWater && walk.can((x, y))));
        assert!(shore, "the castaway can wade into the lake's shallows");
    }

    #[test]
    fn the_lake_cant_be_walked_around() {
        let walk = walk_with_ladder();
        let w = walk.map.width();
        // The far side: high ground north of the lake's deep water.
        let deep_rows: Vec<i32> = (0..walk.map.height()).filter(|&y| (0..w).any(|x| walk.map.is_high(x, y) && walk.map.terrain(x, y) == Terrain::DeepWater)).collect();
        let top = *deep_rows.first().expect("the lake has deep water");
        let beyond: Vec<(i32, i32)> = (0..top).flat_map(|y| (0..w).map(move |x| (x, y))).filter(|&c| walk.can(c)).collect();
        assert!(beyond.is_empty(), "nothing north of the lake can be walked to: {} cells, e.g. {:?}", beyond.len(), &beyond[..beyond.len().min(8)]);
    }

    #[test]
    fn the_river_on_the_high_ground_has_a_water_spot_in_reach() {
        let walk = walk_with_ladder();
        let spots: Vec<(i32, i32)> = walk.map.water_spots.iter().map(|&(x, y)| (x / TILE_SIZE, y / TILE_SIZE)).collect();
        assert!(spots.iter().any(|&(x, y)| walk.map.is_high(x, y) && walk.near((x, y))), "{spots:?}");
    }

    #[test]
    fn the_west_coast_has_a_water_spot_across_the_bridge() {
        let (walk, without) = (walk_with_bridge(), Walk::from_landing(island()));
        let spots: Vec<(i32, i32)> = walk.map.water_spots.iter().map(|&(x, y)| (x / TILE_SIZE, y / TILE_SIZE)).collect();
        let west = spots.iter().find(|&&(x, y)| !walk.map.is_high(x, y) && walk.near((x, y)) && !without.near((x, y)));
        assert!(west.is_some(), "a spot is reached only over the bridge: {spots:?}");
    }

    #[test]
    fn nothing_from_the_sea_is_on_the_lake_shore() {
        let map = island();
        let w = map.width();
        for y in 0..map.height() {
            for x in 0..w {
                if map.is_high(x, y) {
                    assert!(!matches!(map.feature(x, y), Feature::Shell | Feature::Driftwood(_) | Feature::Palm(_)), "{:?} at {:?}", map.feature(x, y), (x, y));
                }
            }
        }
        assert!(map.crab_spawns.iter().all(|&(x, y)| !map.is_high(x / TILE_SIZE, y / TILE_SIZE)));
    }

    #[test]
    fn the_high_ground_isnt_counted_as_dry_land() {
        let mut map = island();
        let before = for_area(&map, 100);
        let w = map.width();
        let cells: Vec<(i32, i32)> = (0..map.height()).flat_map(|y| (0..w).map(move |x| (x, y))).filter(|&(x, y)| map.is_high(x, y)).collect();
        for (x, y) in cells {
            map.set_terrain(x, y, Terrain::Sand);
        }
        assert_eq!(for_area(&map, 100), before);
    }

    /// Every cell of the patches `is` picks out.
    fn patch_cells(map: &WorldMap, is: impl Fn(Feature) -> bool) -> Vec<(i32, i32)> {
        let w = map.width();
        (0..map.height()).flat_map(|y| (0..w).map(move |x| (x, y))).filter(|&(x, y)| is(map.feature(x, y))).collect()
    }

    fn clay(map: &WorldMap) -> Vec<(i32, i32)> {
        patch_cells(map, |f| matches!(f, Feature::ClayBed(_)))
    }

    fn earth(map: &WorldMap) -> Vec<(i32, i32)> {
        patch_cells(map, |f| matches!(f, Feature::BareEarth(_)))
    }

    #[test]
    fn clay_beds_lie_in_the_mud_of_the_lake_shore() {
        let map = island();
        let beds = clay(&map);
        assert!(!beds.is_empty());
        for (x, y) in beds {
            assert!(map.is_high(x, y) && map.terrain(x, y) == Terrain::Sand && map.soil(x, y) == Soil::Mud, "{:?}", (x, y));
        }
    }

    #[test]
    fn the_clay_beds_are_reached_up_the_ladder() {
        let (with, without) = (walk_with_ladder(), Walk::from_landing(island()));
        let beds = clay(&with.map);
        assert!(beds.iter().all(|&c| with.can(c)), "every bed is reached from the ladder");
        assert!(beds.iter().all(|&c| !without.can(c)));
    }

    #[test]
    fn the_shore_north_of_the_ladder_is_left_open_to_launch_a_raft_from() {
        let map = island();
        let (lx, _) = map.spots.ladder.expect("the island has a low cliff");
        let in_the_way: Vec<_> = clay(&map).into_iter().filter(|&(x, _)| (x - lx).abs() <= LAUNCH_HALF_WIDTH).collect();
        assert!(in_the_way.is_empty(), "{in_the_way:?}");
    }

    #[test]
    fn bare_earth_shows_through_the_grass_in_the_first_area() {
        let walk = Walk::from_landing(island());
        assert!(earth(&walk.map).iter().any(|&c| walk.can(c)));
    }

    #[test]
    fn bare_earth_shows_through_the_grass_across_the_bridge_and_up_the_ladder_too() {
        let (bridge, ladder, first) = (walk_with_bridge(), walk_with_ladder(), Walk::from_landing(island()));
        let patches = earth(&first.map);
        assert!(patches.iter().any(|&c| bridge.can(c) && !first.can(c) && !bridge.map.is_high(c.0, c.1)), "on the west coast");
        assert!(patches.iter().any(|&c| ladder.can(c) && ladder.map.is_high(c.0, c.1)), "on the high ground");
    }

    #[test]
    fn bare_earth_is_grass_worn_away_and_can_be_walked_over() {
        let map = island();
        for (x, y) in earth(&map) {
            assert!(matches!(map.terrain(x, y), Terrain::Grass | Terrain::Highland), "{:?}", (x, y));
            assert!(!map.props(x, y).solid);
        }
    }

    #[test]
    fn patches_of_clay_and_earth_are_two_cells_by_two() {
        let map = island();
        for (x, y) in clay(&map).into_iter().chain(earth(&map)) {
            let part = match map.feature(x, y) {
                Feature::ClayBed(c) => c.part,
                Feature::BareEarth(p) => p,
                _ => unreachable!(),
            };
            let anchor = (x - (part % 2) as i32, y - (part / 2) as i32);
            for k in 0..4u8 {
                let c = (anchor.0 + (k % 2) as i32, anchor.1 + (k / 2) as i32);
                let same = match (map.feature(x, y), map.feature(c.0, c.1)) {
                    (Feature::ClayBed(_), Feature::ClayBed(o)) => o.part == k,
                    (Feature::BareEarth(_), Feature::BareEarth(o)) => o == k,
                    _ => false,
                };
                assert!(same, "the patch at {anchor:?} is whole");
            }
        }
    }

    #[test]
    fn a_ladder_stood_at_the_low_cliff_leads_up_to_the_high_ground() {
        let mut map = island();
        let (foot, rows) = ladder_face(&map);
        map.stand_ladder(foot, rows);
        assert!(Walk::from_landing(map).reaches_high_ground());
    }
}

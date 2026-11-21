use std::collections::HashMap;

use super::river::River;
use super::terrain::{Feature, Marking, Soil, Terrain, TileProps, tile_props};
use crate::common::math::Rect;
use crate::items::ItemKind;

/// Size of a map cell in pixels (one metatile).
pub const TILE_SIZE: i32 = 16;
/// How far (px) from the water-drawing spot the river can be reached.
const DRAW_WATER_RANGE: f32 = 30.0;

/// Fruit or bark left on trees, by the tree's top-left cell.
pub type TreeCounts = Vec<((i32, i32), u8)>;

/// Places on the island with a part to play later (cells).
#[derive(Clone, Copy, Debug, Default)]
pub struct Spots {
    /// The middle of the river where it runs narrow.
    pub bridge: Option<(i32, i32)>,
    /// The foot of the low stretch of the north cliff.
    pub ladder: Option<(i32, i32)>,
    /// The middle of the pool under the falls.
    pub falls: Option<(i32, i32)>,
    /// Where a water mill is to stand: the river's west bank below the
    /// pool, with open ground kept behind it for the mill house.
    pub mill: Option<(i32, i32)>,
    /// The foot of the slope up the west cliff (its middle row).
    pub slope: Option<(i32, i32)>,
    /// How far the slope reaches across itself, from its foot to its top,
    /// measured on the slant (world px `x - y`, the slope rising to the
    /// north-east): its shading runs from the one to the other.
    pub slope_span: Option<(i32, i32)>,
}

#[derive(Clone, Debug)]
pub struct WorldMap {
    w: i32,
    h: i32,
    terrain: Vec<Terrain>,
    features: Vec<Feature>,
    /// Smooth elevation per cell (tiles above the waterline), sampled at cell
    /// centres. Used for organic, non-grid-aligned terrain edges.
    elevation: Vec<f32>,
    pub river: Option<River>,
    /// Cells the river's water or banks reach into.
    riverside: Vec<bool>,
    /// Ground cells at an inner corner of a slanting cliff's foot whose top
    /// left (bit 0) or top right (bit 1) half is rock, carried down from
    /// the faces beside and above them: solid there, as drawn.
    foot: Vec<u8>,
    /// Banks of the river where water can be drawn (world px): by the pool
    /// under the falls, and up on the high ground.
    pub water_spots: Vec<(i32, i32)>,
    /// Cells of the high ground's layer (above the cliffs), lake and all.
    high: Vec<bool>,
    /// What each cell's ground is made of (see `Soil`).
    soil: Vec<Soil>,
    /// How many tiers a crag's top has risen at each of its cells (0 at
    /// its outer edge, higher toward its middle).
    rise: Vec<u8>,
    pub spots: Spots,
    /// Fruit still hanging on each tree, keyed by the tree's top-left cell.
    fruit: HashMap<(i32, i32), u8>,
    /// Strips of bark left on each broadleaf tree (by top-left cell).
    bark: HashMap<(i32, i32), u8>,
    /// Player start position (pixels, sprite top-left).
    pub spawn: (i32, i32),
    /// Suggested crab start cells.
    pub crab_spawns: Vec<(i32, i32)>,
    /// Things lying about when the castaway arrives (world px).
    pub loot: Vec<(ItemKind, (i32, i32))>,
}

/// How many cells of rock face a ladder spans.
pub const LADDER_ROWS: i32 = 2;

/// How many cells of river a log bridge spans.
pub const BRIDGE_CELLS: i32 = 2;
/// How many rows either side of the mill site a water wheel held facing
/// the river goes in from (the castaway steps along the bank to the site).
pub const WHEEL_REACH: i32 = 3;

/// The cells a water wheel takes, off the mill site's bank `site`: the two
/// river cells east of it, on its row and the row above, in the order of
/// `Feature::WaterWheel`'s parts.
pub fn wheel_cells((sx, sy): (i32, i32)) -> [(i32, i32); 4] {
    [(sx + 1, sy - 1), (sx + 2, sy - 1), (sx + 1, sy), (sx + 2, sy)]
}

/// The cells a wood lathe takes beside the water wheel off the mill
/// site's bank `site`: the bank cell itself and the one west of it, on the
/// wheel's two rows, so the wheel's axle reaches it; in the order of
/// `Feature::Lathe`'s parts.
pub fn lathe_cells((sx, sy): (i32, i32)) -> [(i32, i32); 4] {
    [(sx - 1, sy - 1), (sx, sy - 1), (sx - 1, sy), (sx, sy)]
}

/// How many cells round the wood lathe's spot (beside the turning wheel)
/// a lathe held in hand is still set there (the castaway walks over to
/// it first).
pub const LATHE_REACH: i32 = 4;

/// Where the castaway stands to set a wood lathe beside the water wheel
/// off the mill site's bank `site`: just west of it, facing east.
pub fn lathe_stand((sx, sy): (i32, i32)) -> (i32, i32) {
    (sx - 2, sy)
}

/// The ground the mill is to stand on, marked out with stakes, off the
/// mill site's bank `site`: `(x0, y0, x1, y1)`, its corner cells. It runs
/// west from the bank and takes in the wood lathe beside the water wheel
/// (on its east edge), for the mill to be built round it. It stops a row
/// short of the water-drawing spot north of the lathe: the frame's north
/// plate rises into that row, which must be clear of the river's water
/// drawn on BG2.
pub fn mill_plot((sx, sy): (i32, i32)) -> (i32, i32, i32, i32) {
    (sx - 6, sy - 2, sx, sy + 2)
}

/// Where the bench for building the mill stands, off the mill site's bank
/// `site`: just west of its ground, a row in from its north edge (out of
/// the way of the building, on the side the castaway comes from, and off
/// the row of the spot they build from).
pub fn site_bench(site: (i32, i32)) -> (i32, i32) {
    let (x0, y0, _, _) = mill_plot(site);
    (x0 - 1, y0 + 1)
}

/// Where the mill's posts will stand on its ground off the mill site's
/// bank `site`, a flat stone bedded under each: its corners and the middle
/// of its long sides.
pub fn foundation_stones(site: (i32, i32)) -> [(i32, i32); 6] {
    let (x0, y0, x1, y1) = mill_plot(site);
    let mid = (x0 + x1) / 2;
    [(x0, y0), (mid, y0), (x1, y0), (x0, y1), (mid, y1), (x1, y1)]
}

/// How many cells round the mill's ground marking stakes held in hand are
/// still driven in there (the castaway walks over first).
pub const STAKES_REACH: i32 = 4;

/// Where the castaway stands to drive the marking stakes in: in the
/// middle of the mill's ground, west of the lathe.
pub fn stakes_stand((sx, sy): (i32, i32)) -> (i32, i32) {
    (sx - 3, sy)
}

/// How many rows up or down the river from the narrow's middle a log
/// bridge can be held out and still go across it (at the middle).
pub const BRIDGE_REACH: i32 = 3;

/// Where a log bridge goes across the river: its row, and the bank cells
/// its ends rest on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BridgeSpan {
    pub row: i32,
    pub west: i32,
    pub east: i32,
}

/// Whether a ladder stood against a cliff would reach the top.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LadderFit {
    /// It reaches: the foot of the face it stands against, and the face's
    /// height in cells.
    Reaches { foot: (i32, i32), rows: i32 },
    /// Nowhere to stand it (a cliff too tall for it included).
    No,
}

impl WorldMap {
    pub fn new(w: i32, h: i32, fill: Terrain) -> Self {
        let n = (w * h) as usize;
        Self {
            w,
            h,
            terrain: vec![fill; n],
            features: vec![Feature::None; n],
            elevation: vec![0.0; n],
            river: None,
            riverside: vec![false; n],
            foot: vec![0; n],
            water_spots: Vec::new(),
            high: vec![false; n],
            soil: vec![Soil::Sand; n],
            rise: vec![0; n],
            spots: Spots::default(),
            fruit: HashMap::new(),
            bark: HashMap::new(),
            spawn: (0, 0),
            crab_spawns: Vec::new(),
            loot: Vec::new(),
        }
    }

    pub fn width(&self) -> i32 {
        self.w
    }

    pub fn height(&self) -> i32 {
        self.h
    }

    pub fn pixel_size(&self) -> (i32, i32) {
        (self.w * TILE_SIZE, self.h * TILE_SIZE)
    }

    /// Chunk columns and rows covering the map.
    pub fn chunk_count(&self) -> (i32, i32) {
        super::chunk::chunks_for(self.w, self.h)
    }

    /// The map's cells in a chunk, row by row (none past the map's edge).
    pub fn chunk_cells(&self, id: super::chunk::ChunkId) -> impl Iterator<Item = (i32, i32)> + use<> {
        let (x0, y0) = id.origin();
        let (x1, y1) = ((x0 + super::chunk::CHUNK_CELLS).min(self.w), (y0 + super::chunk::CHUNK_CELLS).min(self.h));
        (y0.max(0)..y1).flat_map(move |y| (x0.max(0)..x1).map(move |x| (x, y)))
    }

    #[inline]
    pub fn in_bounds(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && x < self.w && y < self.h
    }

    #[inline]
    fn idx(&self, x: i32, y: i32) -> usize {
        (y * self.w + x) as usize
    }

    /// Out-of-bounds cells read as deep water.
    pub fn terrain(&self, x: i32, y: i32) -> Terrain {
        if self.in_bounds(x, y) { self.terrain[self.idx(x, y)] } else { Terrain::DeepWater }
    }

    pub fn feature(&self, x: i32, y: i32) -> Feature {
        if self.in_bounds(x, y) { self.features[self.idx(x, y)] } else { Feature::None }
    }

    pub fn set_terrain(&mut self, x: i32, y: i32, t: Terrain) {
        if self.in_bounds(x, y) {
            let i = self.idx(x, y);
            self.terrain[i] = t;
        }
    }

    pub fn set_feature(&mut self, x: i32, y: i32, f: Feature) {
        if self.in_bounds(x, y) {
            let i = self.idx(x, y);
            self.features[i] = f;
        }
    }

    pub fn set_elevation(&mut self, e: Vec<f32>) {
        assert_eq!(e.len(), self.elevation.len());
        self.elevation = e;
    }

    /// Elevation of a cell; coordinates are clamped to the map.
    pub fn elevation(&self, x: i32, y: i32) -> f32 {
        let (x, y) = (x.clamp(0, self.w - 1), y.clamp(0, self.h - 1));
        self.elevation[self.idx(x, y)]
    }

    /// Bilinearly interpolated elevation at a world pixel.
    pub fn elevation_at_px(&self, px: f32, py: f32) -> f32 {
        let (u, v) = (px / TILE_SIZE as f32 - 0.5, py / TILE_SIZE as f32 - 0.5);
        let (x0, y0) = (u.floor() as i32, v.floor() as i32);
        let (tx, ty) = (u - x0 as f32, v - y0 as f32);
        let a = self.elevation(x0, y0);
        let b = self.elevation(x0 + 1, y0);
        let c = self.elevation(x0, y0 + 1);
        let d = self.elevation(x0 + 1, y0 + 1);
        let top = a + (b - a) * tx;
        let bot = c + (d - c) * tx;
        top + (bot - top) * ty
    }

    /// Whether the river's water (or the bank just beside it) reaches into
    /// the cell.
    /// Whether a cell is up on the high ground (its lake and shores too).
    pub fn is_high(&self, x: i32, y: i32) -> bool {
        self.in_bounds(x, y) && self.high[self.idx(x, y)]
    }

    /// What a cell's ground is made of.
    pub fn soil(&self, x: i32, y: i32) -> Soil {
        let (x, y) = (x.clamp(0, self.w - 1), y.clamp(0, self.h - 1));
        self.soil[self.idx(x, y)]
    }

    pub fn set_soil(&mut self, x: i32, y: i32, soil: Soil) {
        if self.in_bounds(x, y) {
            let i = self.idx(x, y);
            self.soil[i] = soil;
        }
    }

    /// The tier of a crag's top at a cell (0 off the crags, or at their
    /// outer edge).
    pub fn rise(&self, x: i32, y: i32) -> u8 {
        if self.in_bounds(x, y) { self.rise[self.idx(x, y)] } else { 0 }
    }

    pub fn set_rise(&mut self, x: i32, y: i32, tier: u8) {
        if self.in_bounds(x, y) {
            let i = self.idx(x, y);
            self.rise[i] = tier;
        }
    }

    pub fn set_high(&mut self, high: Vec<bool>) {
        assert_eq!(high.len(), self.high.len());
        self.high = high;
    }

    pub fn is_riverside(&self, x: i32, y: i32) -> bool {
        self.in_bounds(x, y) && self.riverside[self.idx(x, y)]
    }

    pub fn set_riverside(&mut self, x: i32, y: i32, on: bool) {
        if self.in_bounds(x, y) {
            let i = self.idx(x, y);
            self.riverside[i] = on;
        }
    }

    /// Which top corners of a cell are filled with the rock of a cliff's
    /// slanting foot: `[left, right]` (the top-left half of the cell from
    /// its top-right to bottom-left corner, or the top-right half).
    pub fn foot_fill(&self, x: i32, y: i32) -> [bool; 2] {
        let bits = if self.in_bounds(x, y) { self.foot[self.idx(x, y)] } else { 0 };
        [bits & 1 != 0, bits & 2 != 0]
    }

    pub fn set_foot_fill(&mut self, x: i32, y: i32, fill: [bool; 2]) {
        if self.in_bounds(x, y) {
            let i = self.idx(x, y);
            self.foot[i] = fill[0] as u8 | (fill[1] as u8) << 1;
        }
    }

    /// Whether any part of `r` that lies in the cell overlaps its foot rock.
    fn hits_foot(&self, x: i32, y: i32, r: Rect) -> bool {
        let [left, right] = self.foot_fill(x, y);
        if !left && !right {
            return false;
        }
        let (x0, y0) = (x * TILE_SIZE, y * TILE_SIZE);
        let (ax, bx) = (r.x.max(x0) - x0, (r.right() - 1).min(x0 + TILE_SIZE - 1) - x0);
        let ay = r.y.max(y0) - y0;
        // The rock is the part of the cell above its slant: the rect
        // reaches it if its corner nearest the rock does.
        (right && bx >= ay) || (left && (TILE_SIZE - 1 - ax) >= ay)
    }

    /// Whether a ladder stood against the rock face at the cell (x, y) would
    /// reach the top: the cell must be the foot of a cliff face, dry and
    /// clear, with the high ground's edge and open ground above it.
    pub fn ladder_fit(&self, x: i32, y: i32) -> LadderFit {
        let face = |y: i32| self.terrain(x, y) == Terrain::Cliff && self.feature(x, y) == Feature::None;
        if !face(y) || self.terrain(x, y + 1) == Terrain::Cliff || self.foot_fill(x, y + 1) != [false, false] || self.is_riverside(x, y) {
            return LadderFit::No;
        }
        let rows = (0..).find(|&k| !face(y - k)).unwrap_or(0);
        let top = y - rows;
        let lands = self.terrain(x, top) == Terrain::CliffTop
            && self.feature(x, top) == Feature::None
            && self.terrain(x, top - 1) == Terrain::Highland
            && !self.props(x, top - 1).solid;
        if lands && rows <= LADDER_ROWS { LadderFit::Reaches { foot: (x, y), rows } } else { LadderFit::No }
    }

    /// Where a ladder held to the rock face at the cell (x, y) would go:
    /// against that face if it reaches the top there (`ladder_fit`), or
    /// else, anywhere along a stretch of rock low enough for a ladder (the
    /// recess in the north cliff), at the one place in that stretch where
    /// it reaches (the middle).
    pub fn ladder_spot(&self, x: i32, y: i32) -> LadderFit {
        if let fit @ LadderFit::Reaches { .. } = self.ladder_fit(x, y) {
            return fit;
        }
        // A cell at the foot of a face no taller than a ladder.
        let low = |x: i32| {
            self.terrain(x, y) == Terrain::Cliff
                && self.terrain(x, y + 1) != Terrain::Cliff
                && (0..=LADDER_ROWS).any(|k| self.terrain(x, y - k) != Terrain::Cliff)
        };
        if !low(x) {
            return LadderFit::No;
        }
        for dir in [-1, 1] {
            let mut cx = x + dir;
            while low(cx) {
                if let fit @ LadderFit::Reaches { .. } = self.ladder_fit(cx, y) {
                    return fit;
                }
                cx += dir;
            }
        }
        LadderFit::No
    }

    /// Stands a ladder against the face whose foot is at `foot`, `rows`
    /// cells tall, up to the edge above. Returns the cells it covers.
    pub fn stand_ladder(&mut self, foot: (i32, i32), rows: i32) -> Vec<(i32, i32)> {
        let (x, y) = foot;
        (0..=rows)
            .map(|k| {
                let cell = (x, y - rows + k);
                self.set_feature(cell.0, cell.1, Feature::Ladder(k as u8));
                cell
            })
            .collect()
    }

    /// Where a log bridge held out over the river cell (x, y) would go:
    /// only across the narrow (`spots.bridge`), and from anywhere along it
    /// (the row above or below too), across its middle row. Both ends must
    /// rest on open ground and the water between be no wider than a log
    /// bridge spans.
    pub fn bridge_spot(&self, x: i32, y: i32) -> Option<BridgeSpan> {
        let (bx, by) = self.spots.bridge?;
        let river = |x: i32| self.terrain(x, by) == Terrain::River && self.feature(x, by) == Feature::None;
        if self.terrain(x, y) != Terrain::River || (y - by).abs() > BRIDGE_REACH || (x - bx).abs() > BRIDGE_CELLS || !river(bx) {
            return None;
        }
        let west = (1..).map(|k| bx - k).find(|&x| !river(x))?;
        let east = (1..).map(|k| bx + k).find(|&x| !river(x))?;
        let open = |x: i32| self.feature(x, by) == Feature::None && !self.props(x, by).solid;
        (east - west - 1 <= BRIDGE_CELLS && open(west) && open(east)).then_some(BridgeSpan { row: by, west, east })
    }

    /// The bank a log bridge would rest on at the narrow's middle row, on
    /// the first area's side (east), while none is laid.
    pub fn bridge_bank(&self) -> Option<(i32, i32)> {
        let (bx, by) = self.spots.bridge?;
        self.bridge_spot(bx, by).map(|span| (span.east, span.row))
    }

    /// Where a water wheel held facing the river cell `(x, y)` would go
    /// in: the mill site's bank (its cell), if the cell is the river just
    /// off that bank within `WHEEL_REACH` rows of it, and no wheel is in yet.
    pub fn wheel_spot(&self, x: i32, y: i32) -> Option<(i32, i32)> {
        let (sx, sy) = self.spots.mill?;
        let off_the_bank = (sx + 1..=sx + 3).contains(&x) && (y - sy).abs() <= WHEEL_REACH;
        let free = wheel_cells((sx, sy)).iter().all(|&(cx, cy)| self.terrain(cx, cy) == Terrain::River && self.feature(cx, cy) == Feature::None);
        (off_the_bank && self.terrain(x, y) == Terrain::River && free).then_some((sx, sy))
    }

    /// Sets a water wheel in the river off the mill site's bank `site`.
    /// Returns the cells it takes.
    pub fn set_wheel(&mut self, site: (i32, i32)) -> Vec<(i32, i32)> {
        let cells = wheel_cells(site);
        for (i, &(x, y)) in cells.iter().enumerate() {
            self.set_feature(x, y, Feature::WaterWheel(i as u8));
        }
        cells.to_vec()
    }

    /// Whether a water wheel stands in the river at the mill site.
    pub fn has_wheel(&self) -> bool {
        self.spots.mill.is_some_and(|site| wheel_cells(site).iter().all(|&(x, y)| matches!(self.feature(x, y), Feature::WaterWheel(_))))
    }

    /// Whether a cart can't be pulled over any cell `r` (world px) touches:
    /// the shallows or a ladder.
    pub fn stops_a_cart(&self, r: crate::common::math::Rect) -> bool {
        let (x0, y0) = (r.x.div_euclid(TILE_SIZE), r.y.div_euclid(TILE_SIZE));
        let (x1, y1) = ((r.right() - 1).div_euclid(TILE_SIZE), (r.bottom() - 1).div_euclid(TILE_SIZE));
        (y0..=y1).any(|y| (x0..=x1).any(|x| self.terrain(x, y) == Terrain::ShallowWater || matches!(self.feature(x, y), Feature::Ladder(_))))
    }

    /// Where a wood lathe held by someone at the cell `(x, y)` would be
    /// set: the mill site's bank (its cell), if the cell is within
    /// `LATHE_REACH` of the cells the lathe would take beside a water wheel
    /// turning in the river there, and the ground there is clear for it.
    pub fn lathe_spot(&self, x: i32, y: i32) -> Option<(i32, i32)> {
        let site = self.spots.mill.filter(|_| self.has_wheel())?;
        let cells = lathe_cells(site);
        let clear = cells.iter().all(|&(cx, cy)| self.terrain(cx, cy).is_land() && self.feature(cx, cy) == Feature::None);
        let near = cells.iter().any(|&(cx, cy)| (cx - x).abs() <= LATHE_REACH && (cy - y).abs() <= LATHE_REACH);
        (clear && near).then_some(site)
    }

    /// Sets a wood lathe on the bank beside the water wheel off the mill
    /// site's bank `site`. Returns the cells it takes.
    pub fn set_lathe(&mut self, site: (i32, i32)) -> Vec<(i32, i32)> {
        let cells = lathe_cells(site);
        for (i, &(x, y)) in cells.iter().enumerate() {
            self.set_feature(x, y, Feature::Lathe(i as u8));
        }
        cells.to_vec()
    }

    /// Where marking stakes held by someone at the cell `(x, y)` would be
    /// driven in: the mill site's bank (its cell), if the wood lathe turns
    /// beside the wheel there, the mill's ground isn't marked out yet, its
    /// corners and the bench's cell are clear, and the cell is within
    /// `STAKES_REACH` of it.
    pub fn stakes_spot(&self, x: i32, y: i32) -> Option<(i32, i32)> {
        let site = self.spots.mill.filter(|_| self.lathe_middle().is_some() && !self.is_staked())?;
        let (x0, y0, x1, y1) = mill_plot(site);
        let clear = [(x0, y0), (x1, y0), (x0, y1), (x1, y1), site_bench(site)]
            .iter()
            .all(|&(cx, cy)| self.terrain(cx, cy).is_land() && self.feature(cx, cy) == Feature::None);
        let near = (x0 - STAKES_REACH..=x1 + STAKES_REACH).contains(&x) && (y0 - STAKES_REACH..=y1 + STAKES_REACH).contains(&y);
        (clear && near).then_some(site)
    }

    /// Marks out the mill's ground off the mill site's bank `site`: a
    /// stake at each corner, rope along the edges between them, and the
    /// bench for building it set up beside it (`site_bench`) (what
    /// stands on an edge already, the lathe, stays, the rope drawn over
    /// it). Returns the cells whose look changed.
    pub fn set_stakes(&mut self, site: (i32, i32)) -> Vec<(i32, i32)> {
        let (x0, y0, x1, y1) = mill_plot(site);
        let mut changed = Vec::new();
        for y in y0..=y1 {
            for x in x0..=x1 {
                let (top, bottom, left, right) = (y == y0, y == y1, x == x0, x == x1);
                if !(top || bottom || left || right) {
                    continue;
                }
                // What already stands on an edge (the lathe) keeps its
                // place; the rope is drawn on over it.
                if self.feature(x, y) != Feature::None {
                    changed.push((x, y));
                    continue;
                }
                let mut ropes = 0;
                if top || bottom {
                    ropes |= if x < x1 { Marking::EAST } else { 0 } | if x > x0 { Marking::WEST } else { 0 };
                }
                if left || right {
                    ropes |= if y < y1 { Marking::SOUTH } else { 0 } | if y > y0 { Marking::NORTH } else { 0 };
                }
                let stake = (top || bottom) && (left || right);
                self.set_feature(x, y, Feature::Marking(Marking { stake, ropes }));
                changed.push((x, y));
            }
        }
        let bench = site_bench(site);
        self.set_feature(bench.0, bench.1, Feature::SiteBench);
        changed.push(bench);
        changed
    }

    /// Lays the mill's foundation on its ground off the mill site's bank
    /// `site`: the stakes and rope come up, the ground is tamped, and flat
    /// stones are bedded where the posts will stand (`foundation_stones`).
    /// What stands there already (the lathe) stays. Returns the cells
    /// changed.
    pub fn lay_foundation(&mut self, site: (i32, i32)) -> Vec<(i32, i32)> {
        let (x0, y0, x1, y1) = mill_plot(site);
        let stones = foundation_stones(site);
        let mut changed = Vec::new();
        for y in y0..=y1 {
            for x in x0..=x1 {
                if matches!(self.feature(x, y), Feature::None | Feature::Marking(_)) {
                    self.set_feature(x, y, Feature::Foundation(stones.contains(&(x, y))));
                }
                changed.push((x, y));
            }
        }
        changed
    }

    /// Raises the mill's frame on its foundation off the mill site's bank
    /// `site`: a post on each stone; the plates and beams over the ground
    /// are drawn from where the posts are (`mill_frame`). Returns the
    /// cells whose look changed: the ground, and the row north of it the
    /// north posts' heads reach up into.
    pub fn raise_frame(&mut self, site: (i32, i32)) -> Vec<(i32, i32)> {
        let (x0, y0, x1, y1) = mill_plot(site);
        let mut changed = Vec::new();
        for y in y0 - 1..=y1 {
            for x in x0..=x1 {
                if let Feature::Foundation(stone) = self.feature(x, y) {
                    self.set_feature(x, y, Feature::Framed(stone));
                }
                changed.push((x, y));
            }
        }
        changed
    }

    /// Whether the mill's frame is up (its posts stand on the stones).
    pub fn has_mill_frame(&self) -> bool {
        self.spots.mill.is_some_and(|site| {
            let (x0, y0, _, _) = mill_plot(site);
            matches!(self.feature(x0, y0), Feature::Framed(_))
        })
    }

    /// Whether the mill's ground is marked out (its corner stakes are in).
    pub fn is_staked(&self) -> bool {
        self.spots.mill.is_some_and(|site| {
            let (x0, y0, _, _) = mill_plot(site);
            matches!(self.feature(x0, y0), Feature::Marking(Marking { stake: true, .. }))
        })
    }

    /// Whether a signal fire could be set in the cell: dry ground (sand
    /// or grass, below or on the high ground) with nothing on it.
    pub fn signal_fire_spot(&self, x: i32, y: i32) -> bool {
        self.in_bounds(x, y) && self.terrain(x, y).is_land() && self.feature(x, y) == Feature::None
    }

    /// Sets a signal fire in the cell, unlit.
    pub fn set_signal_fire(&mut self, (x, y): (i32, i32)) {
        self.set_feature(x, y, Feature::SignalFire(false));
    }

    /// The middle of the wood lathe (world px), if one stands by the wheel.
    pub fn lathe_middle(&self) -> Option<(i32, i32)> {
        let site = self.spots.mill?;
        let cells = lathe_cells(site);
        cells.iter().all(|&(x, y)| matches!(self.feature(x, y), Feature::Lathe(_))).then_some((site.0 * TILE_SIZE, site.1 * TILE_SIZE))
    }

    /// Lays a log bridge across `span`. Returns the cells it covers.
    pub fn lay_bridge(&mut self, span: BridgeSpan) -> Vec<(i32, i32)> {
        (span.west..=span.east)
            .map(|x| {
                let part = if x == span.west {
                    0
                } else if x == span.east {
                    2
                } else {
                    1
                };
                self.set_feature(x, span.row, Feature::Bridge(part));
                (x, span.row)
            })
            .collect()
    }

    /// Whether the cell has a ladder stood in it.
    pub fn is_ladder(&self, x: i32, y: i32) -> bool {
        matches!(self.feature(x, y), Feature::Ladder(_))
    }

    /// How far inside the river's water a world pixel is (px; negative =
    /// outside).
    pub fn river_depth_px(&self, px: f32, py: f32) -> f32 {
        self.river.as_ref().map_or(f32::MIN, |r| r.depth_px(px, py))
    }

    /// Whether the world pixel is river water that can be drawn from the
    /// bank at a water spot.
    pub fn is_drinking_water(&self, px: i32, py: i32) -> bool {
        self.water_spots.iter().any(|&(sx, sy)| {
            let (dx, dy) = ((px - sx) as f32, (py - sy) as f32);
            dx * dx + dy * dy < DRAW_WATER_RANGE * DRAW_WATER_RANGE
        }) && self.river_depth_px(px as f32, py as f32) > -4.0
    }

    /// The water spot nearest a world pixel, if any.
    pub fn nearest_water_spot(&self, px: f32, py: f32) -> Option<(f32, f32)> {
        self.water_spots.iter().map(|&(x, y)| (x as f32, y as f32)).min_by(|a, b| (a.0 - px).hypot(a.1 - py).total_cmp(&(b.0 - px).hypot(b.1 - py)))
    }

    /// Fruit left on the tree whose top-left cell is `anchor`.
    pub fn fruit(&self, anchor: (i32, i32)) -> u8 {
        self.fruit.get(&anchor).copied().unwrap_or(0)
    }

    pub fn set_fruit(&mut self, anchor: (i32, i32), n: u8) {
        if n == 0 {
            self.fruit.remove(&anchor);
        } else {
            self.fruit.insert(anchor, n);
        }
    }

    /// Strips of bark left on the broadleaf tree whose top-left cell is
    /// `anchor`.
    pub fn bark(&self, anchor: (i32, i32)) -> u8 {
        self.bark.get(&anchor).copied().unwrap_or(0)
    }

    pub fn set_bark(&mut self, anchor: (i32, i32), n: u8) {
        self.bark.insert(anchor, n);
    }

    /// The top-left cells of the trees that have had fruit.
    pub fn palm_anchors(&self) -> Vec<(i32, i32)> {
        let mut v: Vec<_> = self.fruit.keys().copied().collect();
        v.sort();
        v
    }

    /// Fruit and bark that differ from `made` (the island as it was made),
    /// by the tree's anchor: what a save has to keep.
    pub fn tree_changes(&self, made: &WorldMap) -> (TreeCounts, TreeCounts) {
        fn changed(now: &HashMap<(i32, i32), u8>, then: &HashMap<(i32, i32), u8>) -> TreeCounts {
            let mut v: Vec<_> = now
                .keys()
                .chain(then.keys())
                .copied()
                .filter(|a| now.get(a).copied().unwrap_or(0) != then.get(a).copied().unwrap_or(0))
                .map(|a| (a, now.get(&a).copied().unwrap_or(0)))
                .collect();
            v.sort();
            v.dedup();
            v
        }
        (changed(&self.fruit, &made.fruit), changed(&self.bark, &made.bark))
    }

    /// Sets the fruit and bark of the trees a save changed (when loading).
    pub fn restore_trees(&mut self, fruit: &[((i32, i32), u8)], bark: &[((i32, i32), u8)]) {
        for &(a, n) in fruit {
            self.set_fruit(a, n);
        }
        for &(a, n) in bark {
            self.set_bark(a, n);
        }
    }

    /// Out-of-bounds cells are solid so nothing can leave the map.
    pub fn props(&self, x: i32, y: i32) -> TileProps {
        if !self.in_bounds(x, y) {
            return TileProps::SOLID;
        }
        tile_props(self.terrain(x, y), self.feature(x, y))
    }

    pub fn props_at_px(&self, px: i32, py: i32) -> TileProps {
        let (x, y) = (px.div_euclid(TILE_SIZE), py.div_euclid(TILE_SIZE));
        if self.hits_foot(x, y, Rect::new(px, py, 1, 1)) {
            return TileProps::SOLID;
        }
        self.props(x, y)
    }

    /// True if any cell overlapped by `r` satisfies `pred`.
    pub fn any_cell(&self, r: Rect, mut pred: impl FnMut(i32, i32) -> bool) -> bool {
        let x0 = r.x.div_euclid(TILE_SIZE);
        let y0 = r.y.div_euclid(TILE_SIZE);
        let x1 = (r.right() - 1).div_euclid(TILE_SIZE);
        let y1 = (r.bottom() - 1).div_euclid(TILE_SIZE);
        for y in y0..=y1 {
            for x in x0..=x1 {
                if pred(x, y) {
                    return true;
                }
            }
        }
        false
    }

    pub fn is_solid_rect(&self, r: Rect) -> bool {
        self.any_cell(r, |x, y| self.props(x, y).solid || self.hits_foot(x, y, r))
    }
}

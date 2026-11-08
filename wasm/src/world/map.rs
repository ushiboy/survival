use std::collections::HashMap;

use super::river::River;
use super::terrain::{Feature, Terrain, TileProps, tile_props};
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
    /// Bank of the upper river where water can be drawn (world px).
    pub water_spot: Option<(i32, i32)>,
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
            water_spot: None,
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

    /// How far inside the river's water a world pixel is (px; negative =
    /// outside).
    pub fn river_depth_px(&self, px: f32, py: f32) -> f32 {
        self.river.as_ref().map_or(f32::MIN, |r| r.depth_px(px, py))
    }

    /// Whether the world pixel is river water that can be drawn from the
    /// bank at `water_spot`.
    pub fn is_drinking_water(&self, px: i32, py: i32) -> bool {
        let Some((sx, sy)) = self.water_spot else {
            return false;
        };
        let (dx, dy) = ((px - sx) as f32, (py - sy) as f32);
        dx * dx + dy * dy < DRAW_WATER_RANGE * DRAW_WATER_RANGE && self.river_depth_px(px as f32, py as f32) > -4.0
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

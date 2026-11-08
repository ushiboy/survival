use std::collections::HashMap;

use super::river::River;
use super::terrain::{Feature, Terrain, TileProps, tile_props};
use crate::common::math::Rect;
use crate::items::ItemKind;

/// Size of a map cell in pixels (one metatile).
pub const TILE_SIZE: i32 = 16;
/// How far (px) from the water-drawing spot the river can be reached.
const DRAW_WATER_RANGE: f32 = 30.0;

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
    /// Bank of the upper river where water can be drawn (world px).
    pub water_spot: Option<(i32, i32)>,
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
            water_spot: None,
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

    /// Fruit on every tree that has any (for saving), by anchor.
    pub fn fruit_entries(&self) -> Vec<((i32, i32), u8)> {
        let mut v: Vec<_> = self.fruit.iter().map(|(&k, &n)| (k, n)).collect();
        v.sort();
        v
    }

    /// Bark left on every broadleaf tree (for saving), by anchor.
    pub fn bark_entries(&self) -> Vec<((i32, i32), u8)> {
        let mut v: Vec<_> = self.bark.iter().map(|(&k, &n)| (k, n)).collect();
        v.sort();
        v
    }

    /// Replaces the fruit and bark on the trees (when loading a save).
    pub fn restore_trees(&mut self, fruit: &[((i32, i32), u8)], bark: &[((i32, i32), u8)]) {
        self.fruit = fruit.iter().copied().filter(|&(_, n)| n > 0).collect();
        self.bark = bark.iter().copied().collect();
    }

    /// Out-of-bounds cells are solid so nothing can leave the map.
    pub fn props(&self, x: i32, y: i32) -> TileProps {
        if !self.in_bounds(x, y) {
            return TileProps::SOLID;
        }
        tile_props(self.terrain(x, y), self.feature(x, y))
    }

    pub fn props_at_px(&self, px: i32, py: i32) -> TileProps {
        self.props(px.div_euclid(TILE_SIZE), py.div_euclid(TILE_SIZE))
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
        self.any_cell(r, |x, y| self.props(x, y).solid)
    }
}

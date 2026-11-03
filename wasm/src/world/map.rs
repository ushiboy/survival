use super::terrain::{Feature, Terrain, TileProps, tile_props};
use crate::common::math::Rect;

/// Size of a map cell in pixels (one metatile).
pub const TILE_SIZE: i32 = 16;

#[derive(Clone, Debug)]
pub struct WorldMap {
    w: i32,
    h: i32,
    terrain: Vec<Terrain>,
    features: Vec<Feature>,
    /// Player start position (pixels, sprite top-left).
    pub spawn: (i32, i32),
    /// Suggested crab start cells.
    pub crab_spawns: Vec<(i32, i32)>,
}

impl WorldMap {
    pub fn new(w: i32, h: i32, fill: Terrain) -> Self {
        let n = (w * h) as usize;
        Self {
            w,
            h,
            terrain: vec![fill; n],
            features: vec![Feature::None; n],
            spawn: (0, 0),
            crab_spawns: Vec::new(),
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

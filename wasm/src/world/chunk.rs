//! The island in chunks: square blocks of cells, the unit in which the far
//! parts of the island are put away and brought back (their ground tiles
//! built, their creatures set going) as the castaway comes near.

use super::map::TILE_SIZE;
use crate::common::math::Rect;

/// Cells along a chunk's side.
pub const CHUNK_CELLS: i32 = 32;
/// Pixels along a chunk's side.
pub const CHUNK_PX: i32 = CHUNK_CELLS * TILE_SIZE;

/// A chunk, by its column and row.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ChunkId {
    pub cx: i32,
    pub cy: i32,
}

impl ChunkId {
    pub const fn new(cx: i32, cy: i32) -> Self {
        Self { cx, cy }
    }

    /// The chunk holding a cell.
    pub fn of_cell(x: i32, y: i32) -> Self {
        Self::new(x.div_euclid(CHUNK_CELLS), y.div_euclid(CHUNK_CELLS))
    }

    /// The chunk holding a world pixel.
    pub fn of_px(px: i32, py: i32) -> Self {
        Self::of_cell(px.div_euclid(TILE_SIZE), py.div_euclid(TILE_SIZE))
    }

    /// Its top-left cell.
    pub fn origin(self) -> (i32, i32) {
        (self.cx * CHUNK_CELLS, self.cy * CHUNK_CELLS)
    }

    /// Its area in world pixels.
    pub fn rect(self) -> Rect {
        Rect::new(self.cx * CHUNK_PX, self.cy * CHUNK_PX, CHUNK_PX, CHUNK_PX)
    }
}

/// How many chunks (columns, rows) cover a map of `w`×`h` cells.
pub fn chunks_for(w: i32, h: i32) -> (i32, i32) {
    ((w + CHUNK_CELLS - 1) / CHUNK_CELLS, (h + CHUNK_CELLS - 1) / CHUNK_CELLS)
}

/// The chunks (of a `count` grid) that a world-pixel rectangle touches.
pub fn chunks_touching(r: Rect, count: (i32, i32)) -> impl Iterator<Item = ChunkId> {
    let a = ChunkId::of_px(r.x, r.y);
    let b = ChunkId::of_px(r.right() - 1, r.bottom() - 1);
    let (x0, y0) = (a.cx.max(0), a.cy.max(0));
    let (x1, y1) = (b.cx.min(count.0 - 1), b.cy.min(count.1 - 1));
    (y0..=y1).flat_map(move |cy| (x0..=x1).map(move |cx| ChunkId::new(cx, cy)))
}

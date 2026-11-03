//! The island's river: it flows in from beyond the north edge of the map,
//! winds down through the jungle and grassland, crosses the beach and runs
//! into the cove. Too deep and fast to wade, it is a barrier.
//!
//! The river is a shape of its own (a widening band around a meandering
//! centreline), separate from the elevation field, so the land keeps its
//! grass right up to the water. Path coordinates are in cells (cell centres
//! sit on whole numbers).

use std::f32::consts::PI;

use super::map::TILE_SIZE;
use crate::common::rng::hash_unit;

/// Half-width of the channel where it enters the map and at the mouth
/// (cells).
const HALF_WIDTH: (f32, f32) = (1.1, 1.6);
/// How far the course swings from a straight line (cells).
const MEANDER: f32 = 3.0;
const PATH_STEPS: usize = 64;

/// Where a point lies relative to the river.
#[derive(Clone, Copy, Debug)]
pub struct RiverPoint {
    /// Distance from the centreline (cells).
    pub dist: f32,
    /// Signed distance across the flow (cells; positive on the side the
    /// normal from `point` points to).
    pub across: f32,
    /// Distance downstream from the source (cells).
    pub along: f32,
    /// How far downstream (0 = source, 1 = mouth).
    pub t: f32,
}

#[derive(Clone, Debug)]
pub struct River {
    /// Centreline from the source (first) to the mouth (last).
    path: Vec<(f32, f32)>,
    /// Distance along the centreline at each path point (cells).
    lengths: Vec<f32>,
}

impl River {
    pub fn new(source: (f32, f32), mouth: (f32, f32), seed: u32) -> Self {
        let phase = hash_unit(1, 2, seed ^ 0x51DE) * PI;
        let path: Vec<(f32, f32)> = (0..=PATH_STEPS)
            .map(|i| {
                let t = i as f32 / PATH_STEPS as f32;
                // Bends fade out at the mouth so the river meets the cove
                // head-on.
                let swing = MEANDER * (t * PI * 2.1 + phase).sin() * (1.0 - t * t);
                (source.0 + (mouth.0 - source.0) * t + swing, source.1 + (mouth.1 - source.1) * t)
            })
            .collect();
        let mut lengths = vec![0.0];
        for w in path.windows(2) {
            let d = ((w[1].0 - w[0].0).powi(2) + (w[1].1 - w[0].1).powi(2)).sqrt();
            lengths.push(lengths.last().unwrap() + d);
        }
        Self { path, lengths }
    }

    pub fn nearest(&self, x: f32, y: f32) -> RiverPoint {
        let mut best = RiverPoint { dist: f32::MAX, across: 0.0, along: 0.0, t: 0.0 };
        for (i, w) in self.path.windows(2).enumerate() {
            let ((ax, ay), (bx, by)) = (w[0], w[1]);
            let (dx, dy) = (bx - ax, by - ay);
            let len2 = dx * dx + dy * dy;
            let u = (((x - ax) * dx + (y - ay) * dy) / len2).clamp(0.0, 1.0);
            let d = ((ax + dx * u - x).powi(2) + (ay + dy * u - y).powi(2)).sqrt();
            if d < best.dist {
                let cross = dx * (y - ay) - dy * (x - ax);
                best = RiverPoint {
                    dist: d,
                    across: d * cross.signum(),
                    along: self.lengths[i] + u * len2.sqrt(),
                    t: (i as f32 + u) / (self.path.len() - 1) as f32,
                };
            }
        }
        best
    }

    pub fn half_width(t: f32) -> f32 {
        HALF_WIDTH.0 + (HALF_WIDTH.1 - HALF_WIDTH.0) * t
    }

    /// Where a world pixel lies relative to the river.
    pub fn at_px(&self, wx: f32, wy: f32) -> RiverPoint {
        let ts = TILE_SIZE as f32;
        self.nearest(wx / ts - 0.5, wy / ts - 0.5)
    }

    /// How far inside the water a world pixel is (px; negative = on land).
    pub fn depth_px(&self, wx: f32, wy: f32) -> f32 {
        let p = self.at_px(wx, wy);
        (Self::half_width(p.t) - p.dist) * TILE_SIZE as f32
    }

    /// Centreline point `t` of the way downstream, and the unit vector
    /// pointing across to the right bank.
    pub fn point(&self, t: f32) -> ((f32, f32), (f32, f32)) {
        let f = t.clamp(0.0, 1.0) * (self.path.len() - 1) as f32;
        let i = (f as usize).min(self.path.len() - 2);
        let ((ax, ay), (bx, by)) = (self.path[i], self.path[i + 1]);
        let u = f - i as f32;
        let (dx, dy) = (bx - ax, by - ay);
        let l = (dx * dx + dy * dy).sqrt();
        ((ax + dx * u, ay + dy * u), (-dy / l, dx / l))
    }
}

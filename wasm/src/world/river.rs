//! The island's river: it rises on the high ground beyond the north edge of
//! the map, falls over the cliff into a pool, winds down through the forest
//! and grassland, crosses the beach and runs out into the cove, where its
//! current carries on out to the deep sea. Too deep and fast to wade, it is
//! a barrier.
//!
//! The river is a shape of its own (a band of varying width around a
//! centreline, plus the round pool under the falls), separate from the
//! elevation field, so the land keeps its grass right up to the water. Path
//! coordinates are in cells (cell centres sit on whole numbers).

use std::f32::consts::PI;

use super::map::TILE_SIZE;
use crate::common::rng::hash_unit;

/// Half-width of the winding river where it leaves the forest and at the
/// mouth (cells).
const HALF_WIDTH: (f32, f32) = (1.1, 1.6);
/// Half-width of the stream above the pool, over the falls.
const HEAD_HALF_WIDTH: f32 = 1.0;
/// How far the course swings from a straight line (cells); its sign picks
/// which way the bends swing. Above the narrow the course is laid straight
/// instead (see `River::narrowed`), so this shapes the river below it.
const MEANDER: f32 = -3.0;
const PATH_STEPS: usize = 64;
/// How far either side of the narrow the course runs straight (cells
/// along it), and how far beyond that it eases back into its bends.
const STRAIGHT_REACH: (f32, f32) = (4.0, 14.0);
/// How far above the narrow (cells along the course) the river, straight
/// down from the pool until then, starts its bend over to the narrow.
const BEND_REACH: f32 = 9.0;
/// Spacing of the points along straight stretches (cells).
const STRAIGHT_STEP: f32 = 0.5;

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
    /// Half-width of the channel there (cells).
    pub half: f32,
}

impl RiverPoint {
    /// How far inside the channel (cells; negative = outside).
    pub fn depth(&self) -> f32 {
        self.half - self.dist
    }
}

#[derive(Clone, Debug)]
pub struct River {
    /// Centreline from the source (first) to the end of the current out at
    /// sea (last).
    path: Vec<(f32, f32)>,
    /// Half-width at each path point (cells).
    widths: Vec<f32>,
    /// Distance along the centreline at each path point (cells).
    lengths: Vec<f32>,
    /// The pool under the falls: centre and radius (cells).
    pool: Option<((f32, f32), f32)>,
}

impl River {
    /// A river winding from `source` to `mouth`.
    pub fn new(source: (f32, f32), mouth: (f32, f32), seed: u32) -> Self {
        let phase = hash_unit(1, 2, seed ^ 0x51DE) * PI;
        let mut path = Vec::new();
        let mut widths = Vec::new();
        for i in 0..=PATH_STEPS {
            let t = i as f32 / PATH_STEPS as f32;
            // Bends fade out at the mouth so the river meets the cove
            // head-on.
            let swing = MEANDER * (t * PI * 2.1 + phase).sin() * (1.0 - t * t);
            path.push((source.0 + (mouth.0 - source.0) * t + swing, source.1 + (mouth.1 - source.1) * t));
            widths.push(HALF_WIDTH.0 + (HALF_WIDTH.1 - HALF_WIDTH.0) * t);
        }
        let mut r = Self { path, widths, lengths: Vec::new(), pool: None };
        r.measure();
        r
    }

    fn measure(&mut self) {
        self.lengths = vec![0.0];
        for w in self.path.windows(2) {
            let d = ((w[1].0 - w[0].0).powi(2) + (w[1].1 - w[0].1).powi(2)).sqrt();
            self.lengths.push(self.lengths.last().unwrap() + d);
        }
    }

    /// Straight points from `a` toward `b` (`a` included, `b` not).
    fn straight(a: (f32, f32), b: (f32, f32)) -> Vec<(f32, f32)> {
        let len = ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt();
        let n = (len / STRAIGHT_STEP).ceil().max(1.0) as usize;
        (0..n)
            .map(|i| {
                let t = i as f32 / n as f32;
                (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t)
            })
            .collect()
    }

    /// Puts the stream's upper course in front of the river: from
    /// `source` straight down over the falls into a pool of `radius` at
    /// `pool`, and on out of the pool to where the river began.
    pub fn with_head(mut self, source: (f32, f32), pool: (f32, f32), radius: f32) -> Self {
        let mut head = Self::straight(source, pool);
        head.extend(Self::straight(pool, self.path[0]));
        let mut widths: Vec<f32> = head.iter().map(|_| HEAD_HALF_WIDTH).collect();
        // Out of the pool the stream widens into the river.
        let n = widths.len();
        let from = self.widths[0];
        for (i, w) in widths.iter_mut().enumerate().skip(n / 2) {
            let t = (i - n / 2) as f32 / (n - n / 2) as f32;
            *w = HEAD_HALF_WIDTH + (from - HEAD_HALF_WIDTH) * t;
        }
        head.append(&mut self.path);
        widths.append(&mut self.widths);
        self.path = head;
        self.widths = widths;
        self.pool = Some((pool, radius));
        self.measure();
        self
    }

    /// Widens the head of the stream into a mouth `extra` cells wider each
    /// side where it starts, narrowing back over `reach` cells: where it
    /// runs out of a lake, the lake's water gathers into it.
    pub fn flared(mut self, extra: f32, reach: f32) -> Self {
        for (w, l) in self.widths.iter_mut().zip(&self.lengths) {
            let t = (1.0 - l / reach).max(0.0);
            *w += extra * t * t;
        }
        self
    }

    /// Narrows the channel to `half` around the point of the course
    /// nearest `at`, over `reach` cells up and down stream, and lines the
    /// course up north to south through it: straight down from the pool,
    /// one bend over to the narrow, straight through it, then back into
    /// the bends (see `STRAIGHT_REACH` and `BEND_REACH`).
    pub fn narrowed(mut self, at: (f32, f32), half: f32, reach: f32) -> Self {
        let along = self.nearest(at.0, at.1).along;
        for (w, l) in self.widths.iter_mut().zip(&self.lengths) {
            let d = (l - along).abs() / reach;
            if d < 1.0 {
                // A smooth dip, narrowest in the middle.
                let k = 0.5 + 0.5 * (d * PI).cos();
                *w += (half - *w) * k;
            }
        }
        // The course runs straight north to south through the narrow, so
        // its few cells across line up on a row (a log bridge spans them).
        // Above it, from the pool down, the river runs straight too, then
        // swings over to the narrow in one bend just before it; below it,
        // the bends ease back in on the way to the sea.
        let mid = self.lengths.iter().enumerate().min_by(|a, b| (a.1 - along).abs().total_cmp(&(b.1 - along).abs())).map_or(0, |(i, _)| i);
        let x = self.path[mid].0;
        let below_pool = self.pool.map_or(f32::MIN, |((_, py), r)| py + r);
        let start = self.path.iter().position(|p| p.1 >= below_pool).unwrap_or(0);
        let top = self.path[start].0;
        let ease = |t: f32| 0.5 - 0.5 * (t.clamp(0.0, 1.0) * PI).cos();
        for (i, (p, l)) in self.path.iter_mut().zip(&self.lengths).enumerate() {
            let d = l - along;
            if d < -STRAIGHT_REACH.0 {
                if i >= start {
                    // Straight down from the pool, then the bend over.
                    let t = (d + BEND_REACH) / (BEND_REACH - STRAIGHT_REACH.0);
                    p.0 = top + (x - top) * ease(t);
                }
            } else if d < STRAIGHT_REACH.0 {
                p.0 = x;
            } else if d < STRAIGHT_REACH.1 {
                let k = 1.0 - ease((d - STRAIGHT_REACH.0) / (STRAIGHT_REACH.1 - STRAIGHT_REACH.0));
                p.0 += (x - p.0) * k;
            }
        }
        self.measure();
        self
    }

    /// Carries the current on past the mouth, straight ahead, until
    /// `is_deep` says the sea is deep enough (at most `max` cells). From
    /// where the river meets the sea (`is_sea`) it fans out, slowly at
    /// first and then faster, as its water spreads into the sea: `spread`
    /// is how much the half-width has grown a cell out (it grows with the
    /// distance to the power 1.5).
    pub fn with_outflow(mut self, max: f32, spread: f32, is_sea: impl Fn(f32, f32) -> bool, is_deep: impl Fn(f32, f32) -> bool) -> Self {
        let n = self.path.len();
        let (a, b) = (self.path[n - 2], self.path[n - 1]);
        let l = ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt();
        let dir = ((b.0 - a.0) / l, (b.1 - a.1) / l);
        let width = self.widths[n - 1];
        // The fan is measured from where the river meets the sea, counted
        // back from the end.
        let shore = self.path.iter().rposition(|&(x, y)| !is_sea(x, y)).map_or(0, |i| i + 1);
        self.measure();
        let from = self.lengths.get(shore).copied().unwrap_or(0.0);
        let fan = |l: f32| spread * (l - from).max(0.0).powf(1.5);
        for (w, &l) in self.widths.iter_mut().zip(&self.lengths).skip(shore) {
            *w += fan(l);
        }
        let mut s = STRAIGHT_STEP;
        while s <= max {
            let p = (b.0 + dir.0 * s, b.1 + dir.1 * s);
            let half = width + fan(self.lengths[n - 1] + s);
            self.path.push(p);
            self.widths.push(half);
            // Run on until the whole width is out over the deep (and a
            // little past), so its end melts into the deep sea.
            let back = (p.0 - dir.0 * 1.5, p.1 - dir.1 * 1.5);
            let side = |q: (f32, f32), k: f32| is_deep(q.0 - dir.1 * half * k, q.1 + dir.0 * half * k);
            if [-1.0, 0.0, 1.0].iter().all(|&k| side(p, k) && side(back, k)) {
                break;
            }
            s += STRAIGHT_STEP;
        }
        self.measure();
        self
    }

    pub fn nearest(&self, x: f32, y: f32) -> RiverPoint {
        self.nearest_of(0..self.path.len() - 1, x, y)
    }

    /// The nearest point on the segments `segments` (indices of their
    /// first points, in order).
    fn nearest_of(&self, segments: impl IntoIterator<Item = usize>, x: f32, y: f32) -> RiverPoint {
        let mut best = RiverPoint { dist: f32::MAX, across: 0.0, along: 0.0, half: 0.0 };
        for i in segments {
            let ((ax, ay), (bx, by)) = (self.path[i], self.path[i + 1]);
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
                    half: self.widths[i] + (self.widths[i + 1] - self.widths[i]) * u,
                };
            }
        }
        best
    }

    /// The river as seen from within the box `(x0, y0)`-`(x1, y1)` (cells):
    /// only the segments that could be nearest to some point in it, so
    /// many lookups there (every pixel of a cell) are quick. They give the
    /// same answers as `nearest`.
    pub fn around(&self, (x0, y0): (f32, f32), (x1, y1): (f32, f32)) -> RiverNear<'_> {
        let seg = |i: usize, x: f32, y: f32| self.nearest_of([i], x, y).dist;
        let corners = [(x0, y0), (x1, y0), (x0, y1), (x1, y1)];
        // A segment's distance from a point is convex, so over the box it
        // is greatest at a corner; the nearest segment to any point in the
        // box is no further than the least of those greatest distances.
        let reach = (0..self.path.len() - 1).map(|i| corners.iter().map(|&(x, y)| seg(i, x, y)).fold(0.0, f32::max)).fold(f32::MAX, f32::min);
        // Keep each segment whose bounding box comes within that of the box.
        let segments = (0..self.path.len() - 1)
            .filter(|&i| {
                let ((ax, ay), (bx, by)) = (self.path[i], self.path[i + 1]);
                let gx = (ax.min(bx) - x1).max(x0 - ax.max(bx)).max(0.0);
                let gy = (ay.min(by) - y1).max(y0 - ay.max(by)).max(0.0);
                (gx * gx + gy * gy).sqrt() <= reach + 1e-3
            })
            .collect();
        RiverNear { river: self, segments }
    }

    /// How far inside the pool a point is (cells; negative = outside).
    pub fn pool_depth(&self, x: f32, y: f32) -> f32 {
        self.pool.map_or(f32::MIN, |((px, py), r)| r - ((x - px).powi(2) + (y - py).powi(2)).sqrt())
    }

    /// The pool under the falls: centre and radius (cells).
    pub fn pool(&self) -> Option<((f32, f32), f32)> {
        self.pool
    }

    /// Where a world pixel lies relative to the river's channel.
    pub fn at_px(&self, wx: f32, wy: f32) -> RiverPoint {
        let ts = TILE_SIZE as f32;
        self.nearest(wx / ts - 0.5, wy / ts - 0.5)
    }

    /// How far inside the water (channel or pool) a world pixel is (px;
    /// negative = on land).
    pub fn depth_px(&self, wx: f32, wy: f32) -> f32 {
        let ts = TILE_SIZE as f32;
        let (x, y) = (wx / ts - 0.5, wy / ts - 0.5);
        self.nearest(x, y).depth().max(self.pool_depth(x, y)) * ts
    }
}

/// The river's segments near a box (see `River::around`).
pub struct RiverNear<'a> {
    river: &'a River,
    segments: Vec<usize>,
}

impl RiverNear<'_> {
    /// `River::at_px`, for a world pixel inside the box.
    pub fn at_px(&self, wx: f32, wy: f32) -> RiverPoint {
        let ts = TILE_SIZE as f32;
        self.river.nearest_of(self.segments.iter().copied(), wx / ts - 0.5, wy / ts - 0.5)
    }
}

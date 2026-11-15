//! Tiny procedural painting toolkit used to author 4bpp art in code:
//! ordered dithering onto colour ramps, tileable noise and simple shapes.

use crate::common::rng::{hash_unit, hash2};
use crate::ppu::pattern::PixelArt;

/// 4x4 Bayer threshold in (0, 1).
#[inline]
pub fn bayer(x: i32, y: i32) -> f32 {
    const M: [[u8; 4]; 4] = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];
    (M[(y & 3) as usize][(x & 3) as usize] as f32 + 0.5) / 16.0
}

/// Maps `t` in [0, 1] onto `ramp` (dark → light). Only a band of width
/// `soft` (in ramp steps) around each boundary is dithered, which keeps the
/// clean banding typical of 16-bit era shading.
pub fn ramp(ramp: &[u8], t: f32, x: i32, y: i32, soft: f32) -> u8 {
    let v = t.clamp(0.0, 1.0) * (ramp.len() as f32 - 0.001) - 0.5;
    let base = v.floor();
    let frac = v - base;
    let threshold = 0.5 + (bayer(x, y) - 0.5) * soft;
    let i = if frac > threshold { base + 1.0 } else { base };
    ramp[(i.max(0.0) as usize).min(ramp.len() - 1)]
}

/// Value noise that tiles every `period` pixels.
#[derive(Clone, Copy)]
pub struct TileNoise {
    pub seed: u32,
    pub period: i32,
}

impl TileNoise {
    pub const fn new(seed: u32, period: i32) -> Self {
        Self { seed, period }
    }

    fn lattice(&self, x: i32, y: i32, cells: i32) -> f32 {
        hash_unit(x.rem_euclid(cells), y.rem_euclid(cells), self.seed ^ cells as u32) * 2.0 - 1.0
    }

    /// One octave with lattice spacing `cell` px (must divide the period).
    pub fn octave(&self, x: f32, y: f32, cell: i32) -> f32 {
        let cells = self.period / cell;
        let (fx, fy) = (x / cell as f32, y / cell as f32);
        let (x0, y0) = (fx.floor() as i32, fy.floor() as i32);
        let s = |t: f32| t * t * (3.0 - 2.0 * t);
        let (tx, ty) = (s(fx - x0 as f32), s(fy - y0 as f32));
        let a = self.lattice(x0, y0, cells);
        let b = self.lattice(x0 + 1, y0, cells);
        let c = self.lattice(x0, y0 + 1, cells);
        let d = self.lattice(x0 + 1, y0 + 1, cells);
        let top = a + (b - a) * tx;
        let bot = c + (d - c) * tx;
        top + (bot - top) * ty
    }

    /// Fractal noise in roughly [-1, 1], starting at lattice spacing `cell`.
    pub fn fbm(&self, x: f32, y: f32, cell: i32, octaves: u32) -> f32 {
        let (mut sum, mut amp, mut norm, mut cell) = (0.0, 1.0, 0.0, cell);
        for _ in 0..octaves {
            sum += self.octave(x, y, cell.max(1)) * amp;
            norm += amp;
            amp *= 0.5;
            cell /= 2;
        }
        sum / norm
    }
}

/// Deterministic per-pixel random in [0, 1).
#[inline]
pub fn noise_px(x: i32, y: i32, seed: u32) -> f32 {
    hash_unit(x, y, seed)
}

#[inline]
pub fn rand_int(x: i32, y: i32, seed: u32, n: u32) -> i32 {
    (hash2(x, y, seed) % n) as i32
}

/// Fills an ellipse; `shade(nx, ny)` gets the normalised position inside
/// (-1..1) and returns the colour, or None to skip the pixel.
pub fn fill_ellipse(art: &mut PixelArt, cx: f32, cy: f32, rx: f32, ry: f32, mut shade: impl FnMut(i32, i32, f32, f32) -> Option<u8>) {
    let (x0, x1) = ((cx - rx).floor() as i32, (cx + rx).ceil() as i32);
    let (y0, y1) = ((cy - ry).floor() as i32, (cy + ry).ceil() as i32);
    for y in y0..=y1 {
        for x in x0..=x1 {
            let nx = (x as f32 + 0.5 - cx) / rx;
            let ny = (y as f32 + 0.5 - cy) / ry;
            if nx * nx + ny * ny <= 1.0
                && let Some(v) = shade(x, y, nx, ny)
            {
                art.put(x, y, v);
            }
        }
    }
}

/// Lambert-ish brightness for a point on a sphere lit from the top-left.
pub fn sphere_light(nx: f32, ny: f32) -> f32 {
    let nz = (1.0 - nx * nx - ny * ny).max(0.0).sqrt();
    let (lx, ly, lz) = (-0.55f32, -0.65f32, 0.52f32);
    let d = (nx * lx + ny * ly + nz * lz).max(0.0);
    (0.15 + 0.85 * d).clamp(0.0, 1.0)
}

/// Draws a 1px line.
pub fn line(art: &mut PixelArt, x0: f32, y0: f32, x1: f32, y1: f32, color: u8) {
    let steps = (x1 - x0).abs().max((y1 - y0).abs()).ceil().max(1.0) as i32;
    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        art.put((x0 + (x1 - x0) * t).round() as i32, (y0 + (y1 - y0) * t).round() as i32, color);
    }
}

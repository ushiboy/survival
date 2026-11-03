//! Value noise for organic coastlines.

use crate::common::rng::hash_unit;

fn lattice(x: i32, y: i32, seed: u32) -> f32 {
    hash_unit(x, y, seed) * 2.0 - 1.0
}

fn smooth(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

/// Smooth noise in roughly `[-1, 1]`.
pub fn value_noise(x: f32, y: f32, seed: u32) -> f32 {
    let (x0, y0) = (x.floor() as i32, y.floor() as i32);
    let (tx, ty) = (smooth(x - x0 as f32), smooth(y - y0 as f32));
    let a = lattice(x0, y0, seed);
    let b = lattice(x0 + 1, y0, seed);
    let c = lattice(x0, y0 + 1, seed);
    let d = lattice(x0 + 1, y0 + 1, seed);
    let top = a + (b - a) * tx;
    let bottom = c + (d - c) * tx;
    top + (bottom - top) * ty
}

/// Fractal sum of octaves, normalised to roughly `[-1, 1]`.
pub fn fbm(x: f32, y: f32, seed: u32, octaves: u32) -> f32 {
    let (mut sum, mut amp, mut freq, mut norm) = (0.0, 1.0, 1.0, 0.0);
    for o in 0..octaves {
        sum += value_noise(x * freq, y * freq, seed.wrapping_add(o * 7919)) * amp;
        norm += amp;
        amp *= 0.5;
        freq *= 2.0;
    }
    sum / norm
}

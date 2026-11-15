//! Procedural ground textures. Each is a seamless 64x64 canvas (4x4 metatiles)
//! so repetition is far less visible than with single 16x16 tiles; a map cell
//! uses the slice matching its position modulo 4.

use super::paint::{TileNoise, bayer, noise_px, ramp, rand_int};
use super::palettes::{deep, grass, rock, sand, shore};
use crate::ppu::pattern::PixelArt;
use std::f32::consts::TAU;

pub const TEX: i32 = 64;
pub const VARIANTS: i32 = TEX / 16;

#[derive(Clone, Debug)]
pub struct TerrainTextures {
    pub sand: PixelArt,
    /// Wet soil of the lake's shore (rock palette's earth browns).
    pub mud: PixelArt,
    pub grass: PixelArt,
    pub shallow: PixelArt,
    pub deep: PixelArt,
}

impl TerrainTextures {
    pub fn generate() -> Self {
        Self { sand: sand_canvas(), mud: mud_canvas(), grass: grass_canvas(), shallow: shallow_canvas(), deep: deep_canvas() }
    }
}

/// The 16x16 slice of a texture for map cell (cx, cy).
pub fn slice(tex: &PixelArt, cx: i32, cy: i32) -> PixelArt {
    tex.sub((cx.rem_euclid(VARIANTS) * 16) as usize, (cy.rem_euclid(VARIANTS) * 16) as usize, 16, 16)
}

fn put_wrap(a: &mut PixelArt, x: i32, y: i32, v: u8) {
    a.set(x.rem_euclid(TEX) as usize, y.rem_euclid(TEX) as usize, v);
}

fn canvas(mut f: impl FnMut(i32, i32) -> u8) -> PixelArt {
    let mut a = PixelArt::filled(TEX as usize, TEX as usize, 0);
    for y in 0..TEX {
        for x in 0..TEX {
            a.set(x as usize, y as usize, f(x, y));
        }
    }
    a
}

fn sand_canvas() -> PixelArt {
    let n = TileNoise::new(11, TEX);
    let mut a = canvas(|x, y| {
        let (fx, fy) = (x as f32, y as f32);
        let base = n.fbm(fx, fy, 32, 3);
        // Wind ripples running diagonally across the beach.
        let dune = (TAU * (fy / TEX as f32 * 5.0 + fx / TEX as f32) + base * 3.0).sin();
        let t = 0.55 + base * 0.2 + dune * 0.14;
        let r = noise_px(x, y, 0x5A11);
        if r < 0.012 {
            sand::GLINT
        } else if r < 0.035 {
            sand::RAMP[0]
        } else if r < 0.040 {
            sand::DEBRIS
        } else {
            ramp(&sand::RAMP, t, x, y, 0.5)
        }
    });
    // Scattered pebbles.
    for k in 0..6 {
        let (x, y) = (rand_int(k, 1, 0x9EB, TEX as u32), rand_int(k, 2, 0x9EB, TEX as u32));
        put_wrap(&mut a, x, y, sand::PEBBLE[1]);
        put_wrap(&mut a, x + 1, y, sand::PEBBLE[2]);
        put_wrap(&mut a, x, y + 1, sand::PEBBLE[0]);
        put_wrap(&mut a, x + 1, y + 1, sand::RAMP[0]);
    }
    a
}

/// Damp, dark earth (rock palette): mottled browns, darker where it's
/// wetter, with moss and a few small stones.
fn mud_canvas() -> PixelArt {
    let n = TileNoise::new(41, TEX);
    let earth = [rock::WOOD[0], rock::WOOD[1], rock::WOOD[2]];
    let mut a = canvas(|x, y| {
        let t = 0.45 + n.fbm(x as f32, y as f32, 32, 3) * 0.35;
        let r = noise_px(x, y, 0x3D1D);
        if r < 0.03 {
            rock::MOSS
        } else if r < 0.05 {
            rock::WOOD[0]
        } else {
            ramp(&earth[..2], t, x, y, 0.6)
        }
    });
    // Little stones bedded in it, lit from the top-left.
    for k in 0..7 {
        let (x, y) = (rand_int(k, 1, 0x3D2E, TEX as u32), rand_int(k, 2, 0x3D2E, TEX as u32));
        put_wrap(&mut a, x, y, rock::STONE[4]);
        put_wrap(&mut a, x + 1, y, rock::STONE[3]);
        put_wrap(&mut a, x, y + 1, rock::STONE[2]);
        put_wrap(&mut a, x + 1, y + 1, rock::STONE[1]);
    }
    // Tufts of moss.
    for k in 0..10 {
        let (x, y) = (rand_int(k, 3, 0x3D3F, TEX as u32), rand_int(k, 4, 0x3D3F, TEX as u32));
        put_wrap(&mut a, x, y, rock::MOSS);
        put_wrap(&mut a, x + 1, y, rock::MOSS);
        put_wrap(&mut a, x, y + 1, rock::WOOD[0]);
    }
    a
}

fn grass_canvas() -> PixelArt {
    let n = TileNoise::new(23, TEX);
    let r = grass::RAMP;
    let mut a = canvas(|x, y| {
        let t = 0.5 + n.fbm(x as f32, y as f32, 32, 3) * 0.32;
        ramp(&[r[2], r[3], r[4]], t, x, y, 0.7)
    });
    // Grass blades on a jittered grid: dark root, mid stem, light tip.
    for gy in 0..TEX / 4 {
        for gx in 0..TEX / 3 + 1 {
            let seed = (gx * 97 + gy * 13) as u32;
            if noise_px(gx, gy, 0x61AD) < 0.25 {
                continue;
            }
            let bx = gx * 3 + rand_int(gx, gy, 0xB1 ^ seed, 3);
            let by = gy * 4 + rand_int(gx, gy, 0xB2, 4);
            let h = 2 + rand_int(gx, gy, 0xB3, 3);
            let lean = rand_int(gx, gy, 0xB4, 3) - 1;
            for k in 0..h {
                let c = if k == 0 {
                    r[1]
                } else if k == h - 1 {
                    if noise_px(gx, gy, 0xB5) < 0.4 { r[6] } else { r[5] }
                } else {
                    r[4]
                };
                put_wrap(&mut a, bx + lean * k / 2, by - k, c);
            }
            put_wrap(&mut a, bx + 1, by, r[2]);
        }
    }
    a
}

fn shallow_canvas() -> PixelArt {
    let depth = TileNoise::new(31, TEX);
    let caustic = TileNoise::new(37, TEX);
    let phase = TileNoise::new(41, TEX);
    let w = shore::WATER;
    canvas(|x, y| {
        let (fx, fy) = (x as f32, y as f32);
        let c = caustic.fbm(fx, fy, 16, 3);
        if c.abs() < 0.028 {
            // Thin caustic network; colours are rotated in CGRAM to make it ripple.
            let p = ((phase.fbm(fx, fy, 16, 2) + 1.0) * 4.5) as i32;
            return shore::CYCLE[p.rem_euclid(3) as usize];
        }
        if noise_px(x, y, 0x5EA) < 0.010 {
            return shore::SEABED;
        }
        let t = 0.5 + depth.fbm(fx, fy, 32, 3) * 0.3 + if c.abs() < 0.07 { 0.18 } else { 0.0 };
        ramp(&[w[1], w[2], w[3]], t, x, y, 0.6)
    })
}

fn deep_canvas() -> PixelArt {
    let n = TileNoise::new(53, TEX);
    let w = deep::WATER;
    let mut a = canvas(|x, y| {
        let t = 0.5 + n.fbm(x as f32, y as f32, 32, 3) * 0.35;
        ramp(&[w[0], w[1], w[2]], t, x, y, 0.6)
    });
    // Swell crests: short arcs whose colours cycle so they appear to roll.
    for gy in 0..TEX / 8 {
        for gx in 0..TEX / 16 {
            let len = 5 + rand_int(gx, gy, 0xD1, 4);
            let bx = gx * 16 + rand_int(gx, gy, 0xD2, (16 - len.min(10)) as u32);
            let by = gy * 8 + 2 + rand_int(gx, gy, 0xD3, 4);
            for i in 0..len {
                let lift = ((i as f32 / (len - 1) as f32) * std::f32::consts::PI).sin() * 1.4;
                let y = by - lift.round() as i32;
                let c = if i == len / 2 && bayer(gx, gy) > 0.5 { deep::CREST } else { deep::CYCLE[((i + gx) % 3) as usize] };
                put_wrap(&mut a, bx + i, y, c);
                put_wrap(&mut a, bx + i, y + 1, w[0]);
            }
        }
    }
    a
}

/// Re-indexes sand pixels for palettes that carry a reduced sand ramp.
pub fn sand_to_shore(v: u8) -> u8 {
    match v {
        _ if v == sand::RAMP[0] || v == sand::DEBRIS || v == sand::DEBRIS_DARK => shore::SAND_DARK,
        _ if v == sand::RAMP[2] || v == sand::GLINT => shore::SAND_LIGHT,
        _ if sand::PEBBLE.contains(&v) => shore::SAND_DARK,
        _ => shore::SAND,
    }
}

pub fn sand_to(v: u8, dark: u8, mid: u8, light: u8) -> u8 {
    match v {
        _ if v == sand::RAMP[0] || v == sand::DEBRIS || v == sand::DEBRIS_DARK || sand::PEBBLE.contains(&v) => dark,
        _ if v == sand::RAMP[2] || v == sand::GLINT => light,
        _ => mid,
    }
}

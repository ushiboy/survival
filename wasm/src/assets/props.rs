//! Map objects painted in code with light coming from the top-left.
//!
//! Background props are overlays: 0 = show the ground underneath,
//! `SHADOW_MASK` = darken the ground, anything else = a palette index.

use super::paint::{fill_ellipse, line, noise_px, ramp, sphere_light};
use super::palettes::{PAL_FOLIAGE, PAL_GRASS, PAL_ROCK, PAL_SAND, foliage, grass, rock, sand};
use crate::ppu::pattern::PixelArt;
use std::f32::consts::PI;

pub const SHADOW_MASK: u8 = 255;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Ground {
    Sand,
    Grass,
}

/// A 16x16 overlay composited onto the cell's ground texture.
#[derive(Clone, Debug)]
pub struct PropTile {
    pub palette: u8,
    pub ground: Ground,
    pub overlay: PixelArt,
}

/// Composites a prop tile onto a ground slice taken from the terrain textures
/// (sand slices use PAL_SAND indices, grass slices PAL_GRASS indices).
pub fn composite(prop: &PropTile, ground_slice: &PixelArt) -> PixelArt {
    let ground = ground_slice.map(|v| reindex_ground(prop.palette, v));
    let mut out = ground.clone();
    for i in 0..out.px.len() {
        out.px[i] = match prop.overlay.px[i] {
            0 => ground.px[i],
            SHADOW_MASK => shade_ground(prop.palette, ground.px[i]),
            v => v,
        };
    }
    out
}

fn reindex_ground(palette: u8, v: u8) -> u8 {
    use super::terrain::sand_to;
    match palette {
        PAL_ROCK => sand_to(v, rock::SAND[0], rock::SAND[1], rock::SAND[2]),
        PAL_FOLIAGE => sand_to(v, foliage::SAND[0], foliage::SAND[0], foliage::SAND[1]),
        _ => v,
    }
}

fn shade_ground(palette: u8, v: u8) -> u8 {
    match palette {
        PAL_SAND => {
            if v == sand::RAMP[0] || v == sand::DEBRIS || v == sand::DEBRIS_DARK { sand::SHADOW[0] } else { sand::SHADOW[1] }
        }
        PAL_ROCK => rock::SAND_SHADOW,
        PAL_FOLIAGE => foliage::SAND_SHADOW,
        PAL_GRASS => {
            let r = grass::RAMP;
            let i = r.iter().position(|&c| c == v).unwrap_or(3);
            r[i.saturating_sub(2)]
        }
        _ => v,
    }
}

fn shadow_ellipse(art: &mut PixelArt, cx: f32, cy: f32, rx: f32, ry: f32) {
    fill_ellipse(art, cx, cy, rx, ry, |x, y, nx, ny| {
        let edge = nx * nx + ny * ny;
        // Soft, dithered rim.
        if edge > 0.75 && super::paint::bayer(x, y) < (edge - 0.75) * 4.0 { None } else { Some(SHADOW_MASK) }
    });
}

// ---------------------------------------------------------------- rocks & wood (PAL_ROCK)

pub fn boulder() -> PropTile {
    let mut a = PixelArt::filled(16, 16, 0);
    shadow_ellipse(&mut a, 9.5, 12.0, 7.0, 3.0);
    let mut stone = PixelArt::filled(16, 16, 0);
    fill_ellipse(&mut stone, 8.0, 8.5, 6.5, 5.5, |x, y, nx, ny| {
        if ny < -0.45 && noise_px(x, y, 0x3055) < 0.55 && nx < 0.4 {
            return Some(rock::MOSS);
        }
        let t = sphere_light(nx, ny) * 0.92 + noise_px(x, y, 0xB01) * 0.12 - 0.02;
        Some(ramp(&rock::STONE[1..], t, x, y, 0.6))
    });
    // A crack.
    line(&mut stone, 9.0, 6.0, 11.0, 10.0, rock::STONE[1]);
    a.overlay(&stone.outlined(rock::STONE[0]), 0, 0);
    PropTile { palette: PAL_ROCK, ground: Ground::Sand, overlay: a }
}

pub fn pebbles() -> PropTile {
    let mut a = PixelArt::filled(16, 16, 0);
    shadow_ellipse(&mut a, 6.0, 11.0, 4.5, 2.0);
    shadow_ellipse(&mut a, 12.0, 8.0, 3.5, 1.5);
    let mut stones = PixelArt::filled(16, 16, 0);
    for (cx, cy, rx, ry) in [(5.5, 9.0, 4.0, 3.0), (11.5, 6.5, 3.0, 2.3)] {
        fill_ellipse(&mut stones, cx, cy, rx, ry, |x, y, nx, ny| {
            Some(ramp(&rock::STONE[1..], sphere_light(nx, ny), x, y, 0.6))
        });
    }
    a.overlay(&stones.outlined(rock::STONE[0]), 0, 0);
    PropTile { palette: PAL_ROCK, ground: Ground::Sand, overlay: a }
}

pub fn driftwood() -> PropTile {
    let mut a = PixelArt::filled(16, 16, 0);
    shadow_ellipse(&mut a, 8.5, 11.0, 7.5, 2.0);
    let mut log = PixelArt::filled(16, 16, 0);
    let (x0, y0, x1, y1) = (1.5f32, 9.5f32, 14.5f32, 6.5f32);
    let (dx, dy) = (x1 - x0, y1 - y0);
    let len = (dx * dx + dy * dy).sqrt();
    let (ux, uy) = (dx / len, dy / len);
    for y in 0..16 {
        for x in 0..16 {
            let (px, py) = (x as f32 + 0.5 - x0, y as f32 + 0.5 - y0);
            let along = px * ux + py * uy;
            let across = -px * uy + py * ux;
            let radius = 2.1 - (along / len - 0.5).abs() * 0.8;
            if !(0.0..=len).contains(&along) || across.abs() > radius {
                continue;
            }
            let v = across / radius;
            let mut t = 0.75 - v * 0.5;
            if (along * 1.3 + v * 2.0).sin() > 0.85 {
                t -= 0.3; // grain
            }
            let c = if along > len - 1.2 { rock::WOOD[2] } else { ramp(&rock::WOOD, t, x, y, 0.5) };
            log.put(x, y, c);
        }
    }
    a.overlay(&log.outlined(rock::WOOD[0]), 0, 0);
    PropTile { palette: PAL_ROCK, ground: Ground::Sand, overlay: a }
}

// ---------------------------------------------------------------- beach debris (PAL_SAND)

pub fn shell() -> PropTile {
    let mut a = PixelArt::filled(16, 16, 0);
    shadow_ellipse(&mut a, 8.5, 11.5, 4.5, 1.5);
    let mut s = PixelArt::filled(16, 16, 0);
    let (cx, cy) = (8.0f32, 11.0f32);
    for y in 0..16 {
        for x in 0..16 {
            let (dx, dy) = (x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);
            let r = (dx * dx + dy * dy).sqrt();
            let ang = dy.atan2(dx);
            if dy > 0.3 || r > 5.0 || !(-PI..=0.0).contains(&ang) {
                continue;
            }
            let rib = ((ang + PI) * 7.0 / PI) as i32 % 2 == 0;
            let lit = r < 2.0 || dx < -1.5;
            s.put(x, y, if rib { if lit { sand::SHELL[2] } else { sand::SHELL[1] } } else { sand::SHELL[1] });
        }
    }
    a.overlay(&s.outlined(sand::SHELL[0]), 0, 0);
    // A little starfish-pink pebble nearby.
    a.put(3, 4, sand::SHELL[1]);
    a.put(4, 4, sand::SHELL[2]);
    PropTile { palette: PAL_SAND, ground: Ground::Sand, overlay: a }
}

// ---------------------------------------------------------------- vegetation (PAL_GRASS)

pub fn bush() -> PropTile {
    let mut a = PixelArt::filled(16, 16, 0);
    shadow_ellipse(&mut a, 8.5, 13.0, 7.5, 2.5);
    let mut b = PixelArt::filled(16, 16, 0);
    let blobs = [(4.0, 10.0, 3.6), (12.0, 10.0, 3.6), (8.0, 11.0, 4.0), (5.5, 6.5, 3.6), (10.5, 6.0, 3.8), (8.0, 4.0, 3.2)];
    let r = grass::RAMP;
    for &(cx, cy, rad) in &blobs {
        fill_ellipse(&mut b, cx, cy, rad, rad * 0.9, |x, y, nx, ny| {
            // Each clump lit on its own, plus overall dome lighting.
            let dome = 1.0 - (y as f32 / 16.0);
            let t = sphere_light(nx, ny) * 0.65 + dome * 0.35;
            Some(ramp(&r[1..7], t, x, y, 0.8))
        });
    }
    for &(x, y) in &[(5, 9), (11, 7), (8, 12), (6, 4)] {
        b.put(x, y, grass::RED);
        b.put(x, y - 1, grass::PINK);
    }
    a.overlay(&b.outlined(r[0]), 0, 0);
    PropTile { palette: PAL_GRASS, ground: Ground::Grass, overlay: a }
}

pub fn flowers() -> PropTile {
    let mut a = PixelArt::filled(16, 16, 0);
    let r = grass::RAMP;
    for &(x, y, petal, centre) in &[
        (4, 4, grass::WHITE, grass::YELLOW),
        (11, 7, grass::YELLOW, grass::RED),
        (6, 12, grass::PINK, grass::YELLOW),
    ] {
        a.put(x, y + 2, r[2]);
        a.put(x, y + 3, r[1]);
        for (dx, dy) in [(0, -1), (-1, 0), (1, 0), (0, 1)] {
            a.put(x + dx, y + dy, petal);
        }
        a.put(x, y, centre);
        a.put(x + 1, y + 1, r[1]);
    }
    PropTile { palette: PAL_GRASS, ground: Ground::Grass, overlay: a }
}

// ---------------------------------------------------------------- palm tree (32x48)

/// Stump left after felling a palm (PAL_FOLIAGE over sand, one cell).
pub fn stump() -> PropTile {
    let mut a = PixelArt::filled(16, 16, 0);
    shadow_ellipse(&mut a, 9.5, 12.5, 7.0, 2.4);
    let mut s = PixelArt::filled(16, 16, 0);
    let (cx, top, bottom) = (8.0f32, 7.0f32, 13.0f32);
    for y in top as i32..=bottom as i32 {
        let hw = 3.8 + (y as f32 - top) * 0.25 + if y as f32 > bottom - 2.0 { 1.0 } else { 0.0 };
        for x in (cx - hw).floor() as i32..=(cx + hw).ceil() as i32 {
            let u = (x as f32 + 0.5 - (cx - hw)) / (2.0 * hw);
            if (0.0..=1.0).contains(&u) {
                let i = if u < 0.3 { 3 } else if u < 0.65 { 2 } else { 1 };
                s.put(x, y, foliage::BARK[i]);
            }
        }
    }
    // Cut face with growth rings.
    fill_ellipse(&mut s, cx, top, 4.0, 1.9, |_, _, nx, ny| {
        let r = (nx * nx + ny * ny).sqrt();
        Some(if r > 0.75 { foliage::BARK[2] } else if r > 0.4 { foliage::SAND[1] } else if r > 0.25 { foliage::BARK[3] } else { foliage::SAND[1] })
    });
    a.overlay(&s.outlined(foliage::BARK[0]), 0, 0);
    // Wood chips scattered around.
    for &(x, y) in &[(1, 13), (14, 12), (2, 9), (13, 15)] {
        a.put(x, y, foliage::BARK[3]);
    }
    PropTile { palette: PAL_FOLIAGE, ground: Ground::Sand, overlay: a }
}

/// Palm tree size in pixels (3x4 cells).
pub const PALM_W: i32 = 48;
pub const PALM_H: i32 = 64;

/// Palm tree art. `upper` (48x48: canopy and upper trunk) goes on BG2 above
/// sprites; `lower` (3 cells, overlay) is the trunk base on BG1; `shade`
/// (3 cells, mask) is the canopy's shadow on the BG1 row under the canopy.
pub struct PalmArt {
    pub upper: PixelArt,
    pub lower: [PropTile; 3],
    pub shade: [PropTile; 3],
}

fn trunk_x(t: f32) -> f32 {
    // Base at x=24, crown at x=22, bowing to the right on the way up.
    24.0 - 2.0 * t + 4.5 * (PI * t).sin()
}

pub fn palm() -> PalmArt {
    let (crown_x, crown_y) = (trunk_x(1.0), 17.0);
    let (base_y, top_y) = (61.0f32, 17.0f32);

    // Trunk with cylindrical shading and bark rings.
    let mut trunk = PixelArt::filled(PALM_W as usize, PALM_H as usize, 0);
    for y in top_y as i32..=base_y as i32 {
        let t = (base_y - y as f32) / (base_y - top_y);
        let cx = trunk_x(t);
        let mut hw = 3.3 - 1.2 * t;
        if y > 56 {
            hw += (y - 56) as f32 * 0.7; // root flare
        }
        let ring = (y % 3) == 0;
        for x in (cx - hw).floor() as i32..=(cx + hw).ceil() as i32 {
            let u = (x as f32 + 0.5 - (cx - hw)) / (2.0 * hw);
            if !(0.0..=1.0).contains(&u) {
                continue;
            }
            let mut i = if u < 0.25 { 3 } else if u < 0.55 { 2 } else { 1 };
            if ring {
                i -= 1;
                // A lighter notch on the lit side of each ring.
                if u < 0.2 {
                    i = 3;
                }
            }
            trunk.put(x, y, foliage::BARK[i.max(0) as usize]);
        }
    }
    let trunk = trunk.outlined(foliage::BARK[0]);

    // Fronds: (angle°, length, droop, back layer). Each frond is a curved
    // midrib with thin leaflets, giving a feathery edge.
    let mut canopy = PixelArt::filled(PALM_W as usize, PALM_H as usize, 0);
    let leaf = foliage::LEAF;
    let fronds: [(f32, f32, f32, bool); 12] = [
        (-130.0, 15.0, 4.0, true), (-90.0, 12.0, 2.0, true), (-50.0, 15.0, 4.0, true),
        (-170.0, 22.0, 16.0, false), (-10.0, 22.0, 16.0, false),
        (163.0, 19.0, 18.0, false), (17.0, 19.0, 18.0, false),
        (-150.0, 18.0, 9.0, false), (-30.0, 18.0, 9.0, false),
        (130.0, 14.0, 12.0, false), (50.0, 14.0, 12.0, false), (92.0, 9.0, 7.0, false),
    ];
    for (fi, &(deg, len, droop, back)) in fronds.iter().enumerate() {
        let a = deg.to_radians();
        let (dx, dy) = (a.cos(), a.sin());
        let steps = (len * 2.0) as i32;
        let point = |t: f32| (crown_x + dx * len * t, crown_y + dy * len * t + droop * t * t);
        for s in (0..=steps).rev() {
            let t = s as f32 / steps as f32;
            let (px, py) = point(t);
            let (qx, qy) = point((t + 0.02).min(1.0));
            let (tx, ty) = {
                let (vx, vy) = (qx - px, qy - py);
                let l = (vx * vx + vy * vy).sqrt().max(1e-3);
                (vx / l, vy / l)
            };
            // Leaflets sweep forward along the frond and droop under gravity.
            let size = 5.6 * (PI * (t * 1.1).min(1.0)).sin().powf(0.7) * (1.0 - 0.3 * t) + 0.8;
            if s % 3 == 0 {
                for side in [-1.0f32, 1.0] {
                    let (mut lx, mut ly) = (tx * 0.8 - ty * side * 0.6, ty * 0.8 + tx * side * 0.6);
                    ly += 0.5;
                    let l = (lx * lx + ly * ly).sqrt();
                    lx /= l;
                    ly /= l;
                    let upper = ly < 0.2;
                    let mut k = 1.0;
                    while k <= size {
                        let (x, y) = (px + lx * k, py + ly * k);
                        let mut light = 0.85 - 0.35 * t - k * 0.025 + if upper { 0.1 } else { -0.2 };
                        if back {
                            light -= 0.3;
                        }
                        light += if fi % 2 == 0 { 0.06 } else { -0.06 };
                        canopy.put(x.round() as i32, y.round() as i32, ramp(&leaf[1..7], light, x as i32, y as i32, 0.4));
                        k += 0.7;
                    }
                }
            }
            canopy.put(px.round() as i32, py.round() as i32, if back { leaf[2] } else if t < 0.7 { leaf[5] } else { leaf[4] });
        }
    }
    // Coconuts.
    for &(cx, cy) in &[(crown_x - 2.4, crown_y + 3.0), (crown_x + 2.2, crown_y + 3.2), (crown_x, crown_y + 5.2)] {
        fill_ellipse(&mut canopy, cx, cy, 2.3, 2.1, |_, _, nx, ny| {
            Some(if nx + ny < -0.6 { foliage::BARK[3] } else if nx + ny > 0.7 { foliage::BARK[0] } else { foliage::COCONUT })
        });
    }
    // Dark edge only below/beside leaves (light comes from above).
    let mut edged = canopy.clone();
    for y in 0..PALM_H {
        for x in 0..PALM_W {
            if canopy.at(x, y) == 0 && (canopy.at(x, y - 1) != 0 || (canopy.at(x - 1, y) != 0 && canopy.at(x + 1, y) != 0)) {
                edged.put(x, y, leaf[0]);
            }
        }
    }
    let canopy = edged;

    let mut full = trunk.clone();
    full.overlay(&canopy, 0, 0);

    // Bottom row: trunk base over sand with a contact shadow.
    let mut lower = PixelArt::filled(PALM_W as usize, 16, 0);
    shadow_ellipse(&mut lower, 26.0, 13.5, 11.0, 2.8);
    lower.overlay(&full.sub(0, 48, PALM_W as usize, 16), 0, 0);

    // Canopy shadow on the row above the base (cast down-right).
    let mut shade = PixelArt::filled(PALM_W as usize, 16, 0);
    shadow_ellipse(&mut shade, 28.0, 9.0, 19.0, 6.5);

    let split = |art: &PixelArt, palette: u8| -> [PropTile; 3] {
        std::array::from_fn(|i| PropTile { palette, ground: Ground::Sand, overlay: art.sub(i * 16, 0, 16, 16) })
    };
    PalmArt { upper: full.sub(0, 0, PALM_W as usize, 48), lower: split(&lower, PAL_FOLIAGE), shade: split(&shade, PAL_SAND) }
}

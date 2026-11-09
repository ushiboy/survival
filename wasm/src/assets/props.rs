//! Map objects painted in code with light coming from the top-left.
//!
//! Background props are overlays: 0 = show the ground underneath,
//! `SHADOW_MASK` = darken the ground, anything else = a palette index.

use super::paint::{bayer, fill_ellipse, line, noise_px, ramp, sphere_light};
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
    let ground = ground_slice.map(|v| reindex_ground(prop.palette, prop.ground, v));
    let mut out = ground.clone();
    for i in 0..out.px.len() {
        out.px[i] = match prop.overlay.px[i] {
            0 => ground.px[i],
            SHADOW_MASK => shade_ground(prop.palette, prop.ground, ground.px[i]),
            v => v,
        };
    }
    out
}

fn reindex_ground(palette: u8, ground: Ground, v: u8) -> u8 {
    use super::terrain::sand_to;
    match (palette, ground) {
        (PAL_ROCK, _) => sand_to(v, rock::SAND[0], rock::SAND[1], rock::SAND[2]),
        // Grass is drawn with the greens alone, which foliage shares.
        (PAL_FOLIAGE, Ground::Grass) => v,
        (PAL_FOLIAGE, _) => sand_to(v, foliage::SAND[0], foliage::SAND[0], foliage::SAND[1]),
        _ => v,
    }
}

fn shade_ground(palette: u8, ground: Ground, v: u8) -> u8 {
    if (palette, ground) == (PAL_FOLIAGE, Ground::Grass) {
        return shade_ground(PAL_GRASS, ground, v);
    }
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

/// A boulder sunk in the sand: a rounded crown whose flanks spread out
/// into a broad foot, the sand banked up round it so it reads as bedded in
/// rather than lying loose.
pub fn boulder() -> PropTile {
    const TOP: f32 = 2.5;
    const FOOT: f32 = 14.5;
    let mut a = PixelArt::filled(16, 16, 0);
    // Shadow thrown down and to the right, hugging the foot.
    shadow_ellipse(&mut a, 9.5, 13.5, 7.5, 2.3);
    let mut stone = PixelArt::filled(16, 16, 0);
    let cx = 8.0;
    for y in 0..16 {
        let fy = y as f32 + 0.5;
        if !(TOP..FOOT).contains(&fy) {
            continue;
        }
        // 0 at the crown, 1 at the foot.
        let t = (fy - TOP) / (FOOT - TOP);
        // A dome that flares out near the ground.
        let half = 4.4 * (1.0 - (1.0 - t).powi(3)).sqrt() + 3.6 * t.powi(3);
        for x in 0..16 {
            let dx = x as f32 + 0.5 - cx;
            if dx.abs() > half {
                continue;
            }
            // The crown faces up, the flanks face out, the foot faces the
            // viewer and slopes away into the sand.
            let ny = -0.85 * (1.0 - t).powf(1.4) + 0.35 * t * t;
            let nx = dx / half * (1.0 - ny * ny).sqrt();
            let mut l = sphere_light(nx, ny) * 0.92 + noise_px(x, y, 0xB01) * 0.12 - 0.02;
            // The flared foot sits lower and darker.
            l -= 0.18 * t.powi(4);
            let c = if t < 0.4 && ny < -0.3 && nx < 0.3 && noise_px(x, y, 0x3055) < 0.45 {
                rock::MOSS
            } else {
                ramp(&rock::STONE[1..], l, x, y, 0.6)
            };
            stone.put(x, y, c);
        }
    }
    // Cracks and a ledge where the stone breaks.
    line(&mut stone, 9.0, 4.0, 10.0, 7.0, rock::STONE[1]);
    line(&mut stone, 10.0, 7.0, 12.0, 8.0, rock::STONE[1]);
    line(&mut stone, 4.0, 10.0, 6.0, 10.0, rock::STONE[2]);
    let mut outlined = stone.outlined(rock::STONE[0]);
    // The foot has no outline: sand banks up over it, ragged and dithered.
    for y in 12..16 {
        for x in 0..16 {
            let fy = y as f32 + 0.5;
            let depth = (fy - 13.0) / 1.5 + (noise_px(x, y, 0x5A4D) - 0.5) * 0.5;
            if outlined.at(x, y) != 0 && depth > bayer(x, y) {
                outlined.put(x, y, if depth > 0.9 { rock::SAND[1] } else { rock::SAND[0] });
            }
        }
    }
    a.overlay(&outlined, 0, 0);
    // A little sand heaped against the flanks.
    for (x, y) in [(1, 13), (2, 12), (14, 13), (13, 12), (0, 14), (15, 14)] {
        if a.at(x, y) == 0 || a.at(x, y) == SHADOW_MASK {
            a.put(x, y, rock::SAND[2]);
        }
    }
    PropTile { palette: PAL_ROCK, ground: Ground::Sand, overlay: a }
}

/// A log of driftwood washed up on the beach, two cells long (32x16, cut
/// into the left and right cell): weathered brown greying in the sun, its
/// root ball splayed at one end and the other broken off jagged, a snapped
/// branch sticking up, grain running along it, half sunk in the sand.
pub fn driftwood() -> [PropTile; 2] {
    let (w, h) = (32usize, 16usize);
    let mut a = PixelArt::filled(w, h, 0);
    shadow_ellipse(&mut a, 17.0, 12.0, 15.0, 2.6);
    let mut log = PixelArt::filled(w, h, 0);
    // Weathered wood, dark to light: brown, greying where the sun and salt
    // have bleached it most; the grain and cracks are the darker browns, a
    // broken end shows the paler wood inside.
    let bleached = [rock::WOOD[0], rock::WOOD[1], rock::WOOD[2], rock::STONE[3], rock::WOOD[3]];
    // A round limb from `a` to `b`, `r0` thick at `a` and `r1` at `b`.
    let limb = |log: &mut PixelArt, (ax, ay): (f32, f32), (bx, by): (f32, f32), r0: f32, r1: f32, grain: bool| {
        let (dx, dy) = (bx - ax, by - ay);
        let len = (dx * dx + dy * dy).sqrt();
        let (ux, uy) = (dx / len, dy / len);
        for y in 0..h as i32 {
            for x in 0..w as i32 {
                let (px, py) = (x as f32 + 0.5 - ax, y as f32 + 0.5 - ay);
                let along = px * ux + py * uy;
                if !(0.0..=len).contains(&along) {
                    continue;
                }
                let r = r0 + (r1 - r0) * along / len;
                let across = -px * uy + py * ux;
                if across.abs() > r {
                    continue;
                }
                // Lit from above: the top of the round limb is brightest.
                let v = across / r;
                let mut t = 0.64 - v * 0.5 + (noise_px(x, y, 0xD21F) - 0.5) * 0.12;
                if grain && ((along * 0.9 + v * 3.1).sin() > 0.88 || (along * 0.37 - v * 5.0).sin() > 0.95) {
                    t -= 0.35;
                }
                log.put(x, y, ramp(&bleached, t, x, y, 0.5));
            }
        }
    };
    // The trunk, thick at the root end and tapering a little.
    limb(&mut log, (7.0, 9.0), (29.0, 8.0), 3.4, 2.6, true);
    // Roots splayed out from the left end.
    for (to, r) in [((1.5, 4.5), 1.1), ((1.0, 9.5), 1.2), ((2.5, 13.5), 1.0), ((5.0, 3.0), 0.9)] {
        limb(&mut log, (7.5, 9.0), to, 1.8, r, false);
    }
    // A snapped-off branch sticking up.
    limb(&mut log, (19.0, 7.0), (22.5, 2.0), 1.2, 0.9, false);
    // Knots.
    for (kx, ky) in [(13, 9), (24, 8)] {
        log.put(kx, ky, rock::WOOD[0]);
        log.put(kx + 1, ky, rock::WOOD[1]);
    }
    let mut log = log.outlined(rock::STONE[0]);
    // The broken right end: jagged, the warmer wood inside showing.
    for (y, cut) in [(5, 29), (6, 30), (7, 28), (8, 30), (9, 29), (10, 30), (11, 28)] {
        log.put(cut, y, rock::WOOD[3]);
        log.put(cut - 1, y, rock::WOOD[2]);
    }
    // The broken branch tip.
    log.put(22, 2, rock::WOOD[3]);
    log.put(23, 2, rock::WOOD[2]);
    a.overlay(&log, 0, 0);
    // Sand drifted up against the underside.
    for x in 4..30 {
        if noise_px(x, 0, 0x5A9D) < 0.45 {
            let y = 12 + (noise_px(x, 1, 0x5A9D) * 1.5) as usize;
            if a.get(x as usize, y) != 0 && a.get(x as usize, y) != SHADOW_MASK {
                a.set(x as usize, y, rock::SAND[1]);
            }
        }
    }
    std::array::from_fn(|half| {
        let mut cell = PixelArt::filled(16, 16, 0);
        for y in 0..16 {
            for x in 0..16 {
                cell.set(x, y, a.get(half * 16 + x, y));
            }
        }
        PropTile { palette: PAL_ROCK, ground: Ground::Sand, overlay: cell }
    })
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

/// A climbing plant on an old stake: twisting stems wound round it and
/// heart-shaped leaves. With `leaves` false, the stake after the vine was
/// pulled off: torn stem ends at its foot.
pub fn vine(leaves: bool) -> PropTile {
    let r = grass::RAMP;
    let mut a = PixelArt::filled(16, 16, 0);
    shadow_ellipse(&mut a, 9.0, 13.5, 5.5, 2.0);
    let mut p = PixelArt::filled(16, 16, 0);
    // The stake: a weathered stick, lit on the left.
    for y in 2..14 {
        p.set(7, y, grass::SAND[1]);
        p.set(8, y, grass::SOIL);
    }
    p.set(7, 2, grass::SAND[2]);
    if leaves {
        // Woody stems twisting up round the stake: brown against the
        // grass, lit on their front turns.
        for (phase, top) in [(0.0f32, 1.0f32), (2.6, 3.0)] {
            let mut y = 14.5;
            while y >= top {
                let turn = y * 0.8 + phase;
                let x = 7.5 + 3.0 * turn.sin() * (0.5 + 0.5 * (14.0 - y) / 13.0);
                p.put(x.round() as i32, y as i32, if turn.cos() > 0.2 { grass::SAND[1] } else { grass::SOIL });
                y -= 0.5;
            }
        }
        // A few broad, bright heart-shaped leaves.
        for &(x, y) in &[(4.0, 3.5), (11.5, 5.0), (4.5, 9.0), (11.0, 10.5), (8.0, 1.5)] {
            fill_ellipse(&mut p, x, y, 2.4, 1.9, |px, py, nx, ny| {
                // Notch at the top makes the heart.
                if ny < -0.5 && nx.abs() < 0.25 {
                    return None;
                }
                Some(ramp(&r[3..7], sphere_light(nx, ny) * 0.9 + 0.15, px, py, 0.5))
            });
        }
    } else {
        // Torn stem ends and a dead leaf or two at the foot.
        for &(x, y) in &[(5, 13), (6, 12), (10, 13), (10, 12), (9, 11)] {
            p.set(x, y, grass::SOIL);
        }
        p.set(4, 14, grass::SAND[0]);
        p.set(11, 14, grass::SAND[0]);
    }
    a.overlay(&p.outlined(r[0]), 0, 0);
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

/// Most coconuts a palm carries.
pub const PALM_COCONUTS: usize = 3;

/// Palm tree art. `upper` (48x48: canopy and upper trunk) goes on BG2 above
/// sprites; `lower` (3 cells, overlay) is the trunk base on BG1; `shade`
/// (3 cells, mask) is the canopy's shadow on the BG1 row under the canopy.
/// `uppers[n]` is `upper` on a palm with `n` coconuts left (only the centre
/// cell, column 1 row 1, differs).
pub struct PalmArt {
    pub uppers: [PixelArt; PALM_COCONUTS + 1],
    pub lower: [PropTile; 3],
    pub shade: [PropTile; 3],
}

fn trunk_x(t: f32) -> f32 {
    // Base at x=24, crown at x=22, bowing to the right on the way up.
    24.0 - 2.0 * t + 4.5 * (PI * t).sin()
}

const CROWN: (f32, f32) = (22.0, 17.0);

/// Centres of the coconuts hanging under the crown (unflipped palm px), in
/// the order they are taken: the lowest one drops first.
pub fn coconut_spots() -> [(f32, f32); PALM_COCONUTS] {
    let (x, y) = CROWN;
    [(x - 2.4, y + 3.0), (x + 2.2, y + 3.2), (x, y + 5.2)]
}

pub fn palm() -> PalmArt {
    let uppers = std::array::from_fn(|n| palm_with(n).0);
    let (_, lower, shade) = palm_with(0);
    PalmArt { uppers, lower, shade }
}

impl PalmArt {
    /// The upper art with `n` coconuts, shifted `dx` px sideways (sway).
    pub fn swayed(&self, n: usize, dx: i32) -> PixelArt {
        self.uppers[n].shift_rows(0..self.uppers[n].h, dx)
    }
}

/// Paints the palm carrying the first `coconuts` of `coconut_spots`.
fn palm_with(coconuts: usize) -> (PixelArt, [PropTile; 3], [PropTile; 3]) {
    let (crown_x, crown_y) = CROWN;
    debug_assert_eq!(crown_x, trunk_x(1.0));
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
    for &(cx, cy) in &coconut_spots()[..coconuts] {
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
    (full.sub(0, 0, PALM_W as usize, 48), split(&lower, PAL_FOLIAGE), split(&shade, PAL_SAND))
}

// ---------------------------------------------------------------- broadleaf tree (48x64)

/// Broadleaf tree (a lauan) art, 3x4 cells. `upper` (48x48: the crown and
/// the top of the trunk) goes on BG2 above sprites; `base` (3 cells on BG1)
/// is the trunk's foot and its shadow on the grass, whole and with bark
/// stripped; `shade` is the crown's shadow on the grass row above.
pub struct BroadleafArt {
    pub upper: PixelArt,
    pub base: [[PropTile; 3]; 2],
    pub shade: [PropTile; 3],
    /// The stump left after felling.
    pub stump: PropTile,
}

pub fn broadleaf() -> BroadleafArt {
    let (cx, base_y, top_y) = (24.0f32, 61.0f32, 26.0f32);
    let bark = foliage::BARK;
    let trunk_px = |art: &mut PixelArt, stripped: bool| {
        for y in top_y as i32..=base_y as i32 {
            let mut hw = 3.4;
            if y > 55 {
                hw += (y - 55) as f32 * 0.8; // buttress roots
            }
            for x in (cx - hw).floor() as i32..=(cx + hw).ceil() as i32 {
                let u = (x as f32 + 0.5 - (cx - hw)) / (2.0 * hw);
                if !(0.0..=1.0).contains(&u) {
                    continue;
                }
                // Rounded shading with long vertical cracks in the bark.
                let mut i = if u < 0.28 { 3 } else if u < 0.6 { 2 } else { 1 };
                if noise_px(x, y / 4, 0xBA4C) < 0.18 {
                    i -= 1;
                }
                let mut c = bark[i];
                // Where bark has been peeled off, pale wood shows.
                if stripped && (49..58).contains(&y) && (u - 0.45).abs() < 0.3 {
                    c = if (u - 0.45).abs() > 0.22 || y == 49 { bark[3] } else { foliage::SAND[1] };
                }
                art.put(x, y, c);
            }
        }
        // Two boughs forking up into the crown.
        line(art, cx - 1.0, 32.0, cx - 9.0, 20.0, bark[1]);
        line(art, cx + 1.0, 31.0, cx + 8.0, 19.0, bark[2]);
    };

    // The crown: overlapping clumps of leaves, each lit on its own and the
    // whole dome lit from above.
    let mut crown = PixelArt::filled(48, 48, 0);
    let clumps = [
        (13.0, 22.0, 9.0), (35.0, 22.0, 9.0), (24.0, 25.0, 9.5), (9.0, 14.0, 7.5), (39.0, 14.0, 7.5),
        (17.0, 11.0, 9.0), (31.0, 10.0, 9.0), (24.0, 6.0, 7.0), (24.0, 16.0, 9.0),
    ];
    let leaf = foliage::LEAF;
    for &(x0, y0, r) in &clumps {
        fill_ellipse(&mut crown, x0, y0, r, r * 0.82, |x, y, nx, ny| {
            let dome = 1.0 - y as f32 / 34.0;
            let t = sphere_light(nx, ny) * 0.6 + dome * 0.4 + (noise_px(x, y, 0x1EAF) - 0.5) * 0.18;
            Some(ramp(&leaf[1..7], t, x, y, 0.7))
        });
    }
    let crown = crown.outlined(leaf[0]);

    let make = |stripped: bool| {
        let mut full = PixelArt::filled(48, 64, 0);
        trunk_px(&mut full, stripped);
        let full = full.outlined(bark[0]);
        let mut with_crown = full.clone();
        with_crown.overlay(&crown, 0, 0);
        with_crown
    };
    let (whole, stripped) = (make(false), make(true));
    let split = |art: &PixelArt, palette: u8| -> [PropTile; 3] {
        std::array::from_fn(|i| PropTile { palette, ground: Ground::Grass, overlay: art.sub(i * 16, 0, 16, 16) })
    };
    // The trunk's foot over its contact shadow.
    let base = |art: &PixelArt| {
        let mut row = PixelArt::filled(48, 16, 0);
        shadow_ellipse(&mut row, 26.0, 13.5, 10.0, 2.6);
        row.overlay(&art.sub(0, 48, 48, 16), 0, 0);
        split(&row, PAL_FOLIAGE)
    };
    // The crown's shadow, cast down-right over the row above the foot.
    let mut shade = PixelArt::filled(48, 16, 0);
    shadow_ellipse(&mut shade, 27.0, 8.0, 21.0, 7.0);

    // Stump: a broad cut face with rings on a short buttressed trunk.
    let mut stump = PixelArt::filled(16, 16, 0);
    for y in 7..=14 {
        let hw = 4.4 + if y > 11 { (y - 11) as f32 * 0.8 } else { 0.0 };
        for x in (8.0 - hw).floor() as i32..=(8.0 + hw).ceil() as i32 {
            let u = (x as f32 + 0.5 - (8.0 - hw)) / (2.0 * hw);
            if (0.0..=1.0).contains(&u) {
                stump.put(x, y, bark[if u < 0.3 { 3 } else if u < 0.65 { 2 } else { 1 }]);
            }
        }
    }
    fill_ellipse(&mut stump, 8.0, 7.0, 4.6, 2.1, |_, _, nx, ny| {
        let r = (nx * nx + ny * ny).sqrt();
        Some(if r > 0.78 { bark[2] } else if (0.35..0.55).contains(&r) { bark[3] } else { foliage::SAND[1] })
    });
    let mut stump_tile = PixelArt::filled(16, 16, 0);
    shadow_ellipse(&mut stump_tile, 9.5, 13.5, 7.5, 2.4);
    stump_tile.overlay(&stump.outlined(bark[0]), 0, 0);

    BroadleafArt {
        upper: whole.sub(0, 0, 48, 48),
        base: [base(&whole), base(&stripped)],
        shade: split(&shade, PAL_GRASS),
        stump: PropTile { palette: PAL_FOLIAGE, ground: Ground::Grass, overlay: stump_tile },
    }
}

// ---------------------------------------------------------------- pine (48x64)

/// Pine (a conifer of the high ground) art, 3x4 cells, laid out like the
/// broadleaf's: `upper` (48x48: the tiers of needles and the trunk among
/// them) goes on BG2 above sprites; `base` is the trunk's foot over its
/// shadow, `shade` the tiers' shadow on the row above it.
pub struct PineArt {
    pub upper: PixelArt,
    pub base: [PropTile; 3],
    pub shade: [PropTile; 3],
}

pub fn pine() -> PineArt {
    let (cx, base_y) = (24.0f32, 61.0f32);
    let bark = foliage::BARK;
    let leaf = foliage::LEAF;
    let mut full = PixelArt::filled(48, 64, 0);
    // A straight trunk, flaring a little at the foot.
    for y in 20..=base_y as i32 {
        let hw = 2.3 + if y > 57 { (y - 57) as f32 * 0.6 } else { 0.0 };
        for x in (cx - hw).floor() as i32..=(cx + hw).ceil() as i32 {
            let u = (x as f32 + 0.5 - (cx - hw)) / (2.0 * hw);
            if (0.0..=1.0).contains(&u) {
                let i = if u < 0.3 { 3 } else if u < 0.65 { 2 } else { 1 };
                let i = if noise_px(x, y / 3, 0x914E) < 0.2 { i - 1 } else { i };
                full.put(x, y, bark[i]);
            }
        }
    }
    let full = full.outlined(bark[0]);
    // Tiers of needles, each a drooping skirt wider than the one above,
    // lit from the left, darker toward its hem and under the tier above.
    let mut crown = PixelArt::filled(48, 48, 0);
    let tiers = [(0.0f32, 15.0f32, 7.0f32), (7.0, 24.0, 11.5), (15.0, 34.0, 16.0), (23.0, 45.0, 20.5)];
    for (k, &(top, hem, w)) in tiers.iter().enumerate() {
        for y in top as i32..=hem as i32 {
            let f = (y as f32 + 0.5 - top) / (hem - top);
            // Ragged sprays of needles along the sides and the hem.
            let rag = (noise_px(y, k as i32, 0x9117) - 0.5) * 2.5;
            let hw = w * f.powf(0.85) + rag * f;
            for x in (cx - hw).floor() as i32..=(cx + hw).ceil() as i32 {
                let nx = (x as f32 + 0.5 - cx) / hw.max(1.0);
                if nx.abs() > 1.0 {
                    continue;
                }
                // The hem hangs lower at the tips than in the middle.
                if f > 0.82 && noise_px(x, y + k as i32 * 7, 0x9118) < (f - 0.82) * 4.0 * (1.0 - nx.abs() * 0.5) {
                    continue;
                }
                let t = 0.62 - nx * 0.32 - f * 0.28 + (noise_px(x, y, 0x9119) - 0.5) * 0.2;
                crown.put(x, y, ramp(&leaf[1..5], t, x, y, 0.7));
            }
        }
    }
    let crown = crown.outlined(leaf[0]);
    let mut whole = full.clone();
    whole.overlay(&crown, 0, 0);

    let split = |art: &PixelArt, palette: u8| -> [PropTile; 3] {
        std::array::from_fn(|i| PropTile { palette, ground: Ground::Grass, overlay: art.sub(i * 16, 0, 16, 16) })
    };
    let mut base = PixelArt::filled(48, 16, 0);
    shadow_ellipse(&mut base, 26.0, 13.5, 8.0, 2.4);
    base.overlay(&whole.sub(0, 48, 48, 16), 0, 0);
    // The tiers' shadow, cast down-right over the row above the foot.
    let mut shade = PixelArt::filled(48, 16, 0);
    shadow_ellipse(&mut shade, 27.0, 7.0, 19.0, 6.0);
    PineArt { upper: whole.sub(0, 0, 48, 48), base: split(&base, PAL_FOLIAGE), shade: split(&shade, PAL_GRASS) }
}

/// A pine cone fallen in the grass: a short brown egg of scales, lit from
/// the top-left, over a little shadow.
pub fn pine_cone() -> PropTile {
    let bark = foliage::BARK;
    let mut a = PixelArt::filled(16, 16, 0);
    shadow_ellipse(&mut a, 8.5, 12.5, 4.0, 1.4);
    let mut c = PixelArt::filled(16, 16, 0);
    fill_ellipse(&mut c, 8.0, 9.5, 2.6, 3.4, |x, y, nx, ny| {
        // Rows of scales: a dark seam under each.
        let seam = (y + (x % 2)) % 2 == 0 && ny > -0.6;
        let lit = sphere_light(nx, ny);
        Some(if seam { bark[1] } else if lit > 0.6 { bark[3] } else { bark[2] })
    });
    a.overlay(&c.outlined(bark[0]), 0, 0);
    PropTile { palette: PAL_FOLIAGE, ground: Ground::Grass, overlay: a }
}

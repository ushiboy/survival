//! Bag-screen graphics: item icons sized to their footprint in the bag (one
//! 16px cell per grid cell), and the cursor bracket. Coconut and timber use
//! the item colours of the crab palette; water and the axe the effects
//! palette.

use super::paint::{bayer, fill_ellipse, ramp, sphere_light};
use super::palettes::{FX_LEGEND, ITEM_LEGEND};
use crate::ppu::pattern::PixelArt;

fn ix(legend: &[(char, u8)], c: char) -> u8 {
    legend.iter().find(|(k, _)| *k == c).map(|(_, v)| *v).unwrap_or(0)
}

/// A whole coconut, 16x16, with its three "eyes" at the stem end.
pub fn icon_coconut() -> PixelArt {
    let i = |c| ix(ITEM_LEGEND, c);
    let mut a = PixelArt::filled(16, 16, 0);
    fill_ellipse(&mut a, 8.0, 8.5, 5.8, 5.4, |x, y, nx, ny| {
        Some(ramp(&[i('d'), i('w'), i('W'), i('h')], sphere_light(nx, ny) * 0.95, x, y, 0.5))
    });
    for (x, y) in [(6, 5), (8, 4), (7, 7)] {
        a.set(x, y, i('d'));
    }
    a.outlined(i('o'))
}

/// A squared timber lying corner to corner, 32x32, cut end at the top right.
pub fn icon_timber() -> PixelArt {
    let i = |c| ix(ITEM_LEGEND, c);
    let (x0, y0, x1, y1) = (4.5f32, 27.5f32, 25.5f32, 6.5f32);
    let (dx, dy) = (x1 - x0, y1 - y0);
    let len = (dx * dx + dy * dy).sqrt();
    let (ux, uy) = (dx / len, dy / len);
    let mut a = PixelArt::filled(32, 32, 0);
    for y in 0..32 {
        for x in 0..32 {
            let (px, py) = (x as f32 + 0.5 - x0, y as f32 + 0.5 - y0);
            let along = px * ux + py * uy;
            let across = -px * uy + py * ux;
            if !(-1.0..=len + 2.5).contains(&along) || across.abs() > 5.0 {
                continue;
            }
            let c = if along > len {
                // End grain with a ring.
                let r = ((along - len - 1.2).powi(2) + across.powi(2)).sqrt();
                if (1.6..2.6).contains(&r) { i('W') } else { i('h') }
            } else if across < 0.0 {
                // Lit top face with grain streaks.
                if ((along * 0.9 + across * 2.0).sin() > 0.8) || (across < -3.8 && bayer(x, y) < 0.5) { i('W') } else { i('h') }
            } else if across < 2.5 {
                i('w')
            } else {
                i('d')
            };
            a.set(x as usize, y as usize, c);
        }
    }
    a.outlined(i('o'))
}

/// A cupped leaf holding water, 16x16.
pub fn icon_raw_water() -> PixelArt {
    let i = |c| ix(FX_LEGEND, c);
    let rows = [
        "................",
        "................",
        "................",
        "................",
        "..oooooooooooo..",
        ".oillllllllllio.",
        ".olllilllllblio.",
        "obbblllllbbbbbbo",
        "ogbbbbbbbbbbbbGo",
        "ogGbbbbbbbbbbGGo",
        ".ogGGbbbbbbGGGo.",
        ".oggGGGGGGGGGgo.",
        "..oggGGGGGGggo..",
        "...oogggggoo....",
        ".....ooooo......",
        "................",
    ];
    let legend = [('o', i('o')), ('g', i('g')), ('G', i('G')), ('b', i('b')), ('l', i('l')), ('i', i('i'))];
    PixelArt::parse(&rows, &legend)
}

/// The axe standing upright, 16x48 (head at the top, blade to the right).
pub fn icon_axe() -> PixelArt {
    let i = |c| ix(FX_LEGEND, c);
    let mut a = PixelArt::filled(16, 48, 0);
    // Handle, lit on the left.
    for y in 8..46 {
        a.set(6, y, i('W'));
        a.set(7, y, i('w'));
        if y % 7 == 3 {
            a.set(7, y, i('W'));
        }
    }
    // Head: a socket round the handle, a stubby poll behind it and a blade
    // flaring out to a bright cutting edge.
    let centre = 6.0f32;
    for x in 4..14i32 {
        let half = if x <= 5 {
            2.0
        } else if x <= 8 {
            2.6
        } else {
            2.6 + (x - 8) as f32 * 0.6
        };
        let edge = x >= 12;
        for y in 0..14i32 {
            if (y as f32 + 0.5 - centre - 0.5).abs() > half {
                continue;
            }
            let c = if edge {
                i('L')
            } else if (y as f32) < centre {
                i('M')
            } else {
                i('m')
            };
            a.set(x as usize, y as usize, c);
        }
    }
    a.outlined(i('o'))
}

/// One corner of the cursor bracket, 8x8 (top-left; flip for the others).
pub fn cursor_corner() -> PixelArt {
    PixelArt::parse(&["ooooo...", "oYYYo...", "oYooo...", "oYo.....", "ooo.....", "........", "........", "........"], FX_LEGEND)
}

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

/// A strip of bark curling over at its ends, 16x16: rough outer bark on
/// top, the pale inner face showing where it curls.
pub fn icon_bark() -> PixelArt {
    let i = |c| ix(ITEM_LEGEND, c);
    let mut a = PixelArt::filled(16, 16, 0);
    let (x0, y0, x1, y1) = (3.0f32, 12.0f32, 13.0f32, 4.0f32);
    let (dx, dy) = (x1 - x0, y1 - y0);
    let len = (dx * dx + dy * dy).sqrt();
    let (ux, uy) = (dx / len, dy / len);
    for y in 0..16 {
        for x in 0..16 {
            let (px, py) = (x as f32 + 0.5 - x0, y as f32 + 0.5 - y0);
            let along = px * ux + py * uy;
            // Bowed: the middle sags, the ends curl up.
            let across = -px * uy + py * ux - ((along / len - 0.5) * 2.0).powi(2) * 1.6;
            if !(-1.0..=len + 1.0).contains(&along) || across.abs() > 2.6 {
                continue;
            }
            let curl = along < 1.5 || along > len - 1.5;
            let c = if curl && across > 0.0 {
                i('h')
            } else if across < -1.2 {
                if (along * 1.7).sin() > 0.6 { i('d') } else { i('W') }
            } else if (along * 1.7 + 1.0).sin() > 0.7 {
                i('d')
            } else {
                i('w')
            };
            a.set(x as usize, y as usize, c);
        }
    }
    a.outlined(i('o'))
}

/// A pot of bark bent into a ring and stitched with vine, 16x16, seen a
/// little from above so its dark inside shows.
pub fn icon_bark_container() -> PixelArt {
    let i = |c| ix(FX_LEGEND, c);
    let mut a = PixelArt::filled(16, 16, 0);
    for y in 4..15 {
        for x in 2..14 {
            let u = (x as f32 + 0.5 - 2.0) / 12.0;
            // Slightly narrower at the foot.
            let foot = (y as f32 - 4.0) / 10.0 * 0.06;
            if !(foot..=1.0 - foot).contains(&u) {
                continue;
            }
            let streak = (x * 5 + y / 3) % 7 == 0;
            let c = if u < 0.3 { i('W') } else if u > 0.78 || streak { i('o') } else { i('w') };
            a.set(x, y, c);
        }
    }
    // Rim and the shadowed inside.
    fill_ellipse(&mut a, 8.0, 4.5, 6.0, 2.2, |_, _, nx, ny| Some(if (nx * nx + ny * ny).sqrt() > 0.7 { i('c') } else { i('o') }));
    // A band of vine round the middle, and the stitched seam.
    for x in 2..14 {
        a.set(x, 9, if x % 2 == 0 { i('G') } else { i('g') });
    }
    for y in [6, 11, 13] {
        a.set(10, y, i('g'));
    }
    a.outlined(i('o'))
}

/// The pack frame standing upright, 16x32: two branch rails with cross
/// bars, a bark board between them, lashed with vine at every joint.
pub fn icon_pack_frame() -> PixelArt {
    let i = |c| ix(FX_LEGEND, c);
    let mut a = PixelArt::filled(16, 32, 0);
    // Bark board between the rails, streaked.
    for y in 8..21 {
        for x in 5..11 {
            a.set(x, y, if (x + y / 2) % 4 == 0 { i('w') } else { i('c') });
        }
    }
    // Rails, splaying a little toward the feet; lit on the left.
    for y in 1..31 {
        let splay = if y > 24 { (y - 24) / 3 } else { 0 };
        a.set(3 - splay, y, i('W'));
        a.set(4 - splay, y, i('w'));
        a.set(11 + splay, y, i('W'));
        a.set(12 + splay, y, i('w'));
    }
    // Cross bars, with vine lashings where they meet the rails.
    for y in [5, 22] {
        for x in 3..13 {
            a.set(x, y, i('W'));
            a.set(x, y + 1, i('w'));
        }
    }
    for y in [5, 8, 20, 22] {
        for x in [3, 4, 11, 12] {
            a.set(x, y, if (x + y) % 2 == 0 { i('G') } else { i('g') });
        }
    }
    a.outlined(i('o'))
}

/// The bark pot full of boiled water, a wisp of steam rising off it.
pub fn icon_boiled_water() -> PixelArt {
    let i = |c| ix(FX_LEGEND, c);
    let mut a = icon_bark_container();
    fill_ellipse(&mut a, 8.0, 4.6, 4.3, 1.4, |x, _, nx, _| Some(if nx < -0.3 || x % 4 == 1 { i('l') } else { i('b') }));
    for (x, y) in [(6, 2), (7, 1), (9, 2), (10, 1), (8, 0)] {
        a.set(x, y, i('i'));
    }
    a
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

/// The stone axe standing upright, 16x32: a knapped stone set against the
/// top of the handle and bound on with crossed turns of vine.
pub fn icon_stone_axe() -> PixelArt {
    let i = |c| ix(FX_LEGEND, c);
    let mut a = PixelArt::filled(16, 32, 0);
    // Stone head, lit from the top left, with a pale chipped edge.
    fill_ellipse(&mut a, 10.5, 8.0, 4.2, 5.6, |x, y, nx, ny| {
        let edge = nx > 0.62 && bayer(x, y) < (nx - 0.5) * 3.0;
        Some(if edge { i('L') } else { ramp(&[i('m'), i('M')], sphere_light(nx, ny), x, y, 0.5) })
    });
    // Handle, lit on the left, with a knot or two.
    for y in 1..30 {
        a.set(6, y, if y % 9 == 5 { i('w') } else { i('W') });
        a.set(7, y, i('w'));
    }
    // Turns of vine binding the stone to the handle, lit on top.
    for y in [4, 7, 10] {
        for x in 5..10 {
            a.set(x, y, i('G'));
            a.set(x, y + 1, i('g'));
        }
    }
    a.outlined(i('o'))
}

/// The wooden spear standing upright, 16x48, point at the top.
pub fn icon_wood_spear() -> PixelArt {
    let i = |c| ix(FX_LEGEND, c);
    let mut a = PixelArt::filled(16, 48, 0);
    for y in 9..46 {
        a.set(7, y, if y % 11 == 3 { i('w') } else { i('W') });
        a.set(8, y, i('w'));
    }
    // Whittled point, tapering to one pixel.
    for y in 2..9 {
        a.set(7, y, i('c'));
        if y >= 5 {
            a.set(8, y, i('c'));
        }
    }
    a.outlined(i('o'))
}

/// Two broad leaves laid one on another along the diagonal, 16x16, with
/// pale midribs and a stalk; the bottom right stays clear for the count.
pub fn icon_leaf() -> PixelArt {
    let i = |c| ix(FX_LEGEND, c);
    let mut a = PixelArt::filled(16, 16, 0);
    // (centre, half-length, half-width, angle): back leaf first.
    for &(cx, cy, len, wid, ang) in &[(8.6f32, 6.4f32, 5.6f32, 2.4f32, -1.25f32), (7.2, 7.8, 6.6, 3.2, -0.72)] {
        let (c, s) = (ang.cos(), ang.sin());
        for y in 0..16 {
            for x in 0..16 {
                let (dx, dy) = (x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);
                let (u, v) = (dx * c + dy * s, -dx * s + dy * c);
                // Pointed at both ends: width tapers along the length.
                let half = wid * (1.0 - (u / len).powi(2)).max(0.0).sqrt();
                if u.abs() > len || v.abs() > half {
                    continue;
                }
                let col = if v.abs() < 0.5 { i('h') } else if v < 0.0 { i('G') } else { i('g') };
                a.set(x, y, col);
            }
        }
    }
    for k in 0..3 {
        a.set(2 + k, 14 - k, i('g'));
    }
    a.outlined(i('o'))
}

/// A HUD font glyph as an 8x8 sprite: outlined, filled with `fill` (dark,
/// light) from the effects palette.
fn glyph_sprite(c: char, fill: (char, char)) -> PixelArt {
    let i = |c| ix(FX_LEGEND, c);
    let glyphs = super::hud_art::glyphs();
    let g = glyphs.iter().find(|(k, _)| *k == c).map(|(_, a)| a.clone()).unwrap_or_else(|| PixelArt::filled(8, 8, 0));
    g.map(|v| match v {
        1 => i('o'),
        2 => i(fill.0),
        3 => i(fill.1),
        _ => 0,
    })
}

/// Digits for stack counts, 8x8 each (0-9): the HUD font in the effects
/// palette's outline and pale metal greys, so they read over any icon.
pub fn count_digits() -> [PixelArt; 10] {
    std::array::from_fn(|d| glyph_sprite(char::from_digit(d as u32, 10).unwrap_or('0'), ('M', 'L')))
}

/// The "E" marking the tool held in hand, 8x8, in spark yellow.
pub fn equipped_mark() -> PixelArt {
    glyph_sprite('E', ('y', 'Y'))
}

/// A bow drill, `size` px square (16 in the bag and on the ground):
/// a bent branch strung with vine, the string looped once round a
/// pale spindle.
pub fn fire_drill(size: usize) -> PixelArt {
    let i = |c| ix(FX_LEGEND, c);
    let k = size as f32 / 32.0;
    let mut a = PixelArt::filled(size, size, 0);
    let dot = |a: &mut PixelArt, cx: f32, cy: f32, r: f32, c: u8| {
        for y in (cy - r).floor() as i32..=(cy + r).ceil() as i32 {
            for x in (cx - r).floor() as i32..=(cx + r).ceil() as i32 {
                if (x as f32 + 0.5 - cx).powi(2) + (y as f32 + 0.5 - cy).powi(2) <= r * r {
                    a.put(x, y, c);
                }
            }
        }
    };
    // String, end to end of the bow.
    let (ax, ay, bx, by) = (5.0 * k, 6.0 * k, 27.0 * k, 25.0 * k);
    for s in 0..=64 {
        let t = s as f32 / 64.0;
        dot(&mut a, ax + (bx - ax) * t, ay + (by - ay) * t, 0.45, i('g'));
    }
    // Bow: a curve bulging toward the bottom left, lit along its outside.
    let (qx, qy) = (4.0 * k, 28.0 * k);
    let r = (1.4 * k).max(0.7);
    for s in 0..=96 {
        let t = s as f32 / 96.0;
        let u = 1.0 - t;
        let (x, y) = (u * u * ax + 2.0 * u * t * qx + t * t * bx, u * u * ay + 2.0 * u * t * qy + t * t * by);
        dot(&mut a, x, y, r, i('w'));
        dot(&mut a, x - 0.5 * k, y + 0.5 * k, r * 0.5, i('W'));
    }
    // Spindle crossing the string, with the loop of vine round it.
    let (sx0, sy0, sx1, sy1) = (20.0 * k, 3.0 * k, 13.0 * k, 29.0 * k);
    for s in 0..=64 {
        let t = s as f32 / 64.0;
        dot(&mut a, sx0 + (sx1 - sx0) * t, sy0 + (sy1 - sy0) * t, (1.0 * k).max(0.5), i('c'));
    }
    let (mx, my) = ((sx0 + sx1) / 2.0, (sy0 + sy1) / 2.0);
    for d in [-1.0f32, 1.0] {
        dot(&mut a, mx + d * 0.8 * k, my - d * 0.6 * k, (0.9 * k).max(0.5), i('G'));
    }
    a.outlined(i('o'))
}

/// The iron axe standing upright, 16x48 (head at the top, blade to the
/// right). Kept for when iron tools can be made.
#[allow(dead_code)]
pub fn icon_iron_axe() -> PixelArt {
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

/// A dead branch standing upright, 16x32, forking near the top.
pub fn icon_branch() -> PixelArt {
    let i = |c| ix(ITEM_LEGEND, c);
    let mut a = PixelArt::filled(16, 32, 0);
    let mut stroke = |x0: f32, y0: f32, x1: f32, y1: f32, r: f32| {
        let steps = ((x1 - x0).abs().max((y1 - y0).abs()) * 2.0) as i32;
        for k in 0..=steps {
            let t = k as f32 / steps as f32;
            let (cx, cy) = (x0 + (x1 - x0) * t, y0 + (y1 - y0) * t);
            for y in (cy - r).floor() as i32..=(cy + r).ceil() as i32 {
                for x in (cx - r).floor() as i32..=(cx + r).ceil() as i32 {
                    let dx = x as f32 + 0.5 - cx;
                    if dx * dx + (y as f32 + 0.5 - cy).powi(2) <= r * r && (0..16).contains(&x) && (0..32).contains(&y) {
                        // Lit from the left; knots darken the bark.
                        let c = if dx < -0.4 { i('W') } else if dx > 0.6 || (y % 9 == 4) { i('d') } else { i('w') };
                        a.set(x as usize, y as usize, c);
                    }
                }
            }
        }
    };
    stroke(7.0, 29.0, 8.0, 12.0, 1.6);
    stroke(8.0, 13.0, 4.0, 3.0, 1.0);
    stroke(8.0, 14.0, 12.0, 6.0, 1.1);
    stroke(7.5, 22.0, 10.5, 19.0, 0.7);
    // Pale broken ends.
    a.set(4, 2, i('h'));
    a.set(12, 5, i('h'));
    a.outlined(i('o'))
}

/// A rounded stone, 16x16.
pub fn icon_stone() -> PixelArt {
    let i = |c| ix(ITEM_LEGEND, c);
    let mut a = PixelArt::filled(16, 16, 0);
    fill_ellipse(&mut a, 8.0, 9.0, 6.0, 4.6, |x, y, nx, ny| Some(ramp(&[i('s'), i('F'), i('f')], sphere_light(nx, ny), x, y, 0.5)));
    a.outlined(i('o'))
}

/// A coil of vine with a few leaves still on it, 16x16.
pub fn icon_vine() -> PixelArt {
    let i = |c| ix(FX_LEGEND, c);
    let mut a = PixelArt::filled(16, 16, 0);
    // Loops of a spiral.
    for k in 0..220 {
        let t = k as f32 / 220.0 * std::f32::consts::TAU * 2.6;
        let r = 2.2 + t * 0.32;
        let (x, y) = ((8.0 + r * t.cos() * 1.1) as i32, (8.5 + r * t.sin() * 0.85) as i32);
        if (0..16).contains(&x) && (0..16).contains(&y) {
            a.set(x as usize, y as usize, if t.sin() < -0.3 { i('G') } else { i('g') });
        }
    }
    for (x, y) in [(3, 4), (12, 11), (11, 3)] {
        fill_ellipse(&mut a, x as f32, y as f32, 1.6, 1.2, |_, _, nx, ny| Some(if nx + ny < 0.0 { i('h') } else { i('G') }));
    }
    a.outlined(i('o'))
}

/// One corner of the cursor bracket, 8x8 (top-left; flip for the others).
pub fn cursor_corner() -> PixelArt {
    PixelArt::parse(&["ooooo...", "oYYYo...", "oYooo...", "oYo.....", "ooo.....", "........", "........", "........"], FX_LEGEND)
}

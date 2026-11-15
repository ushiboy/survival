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
/// bars, a bark board between them, lashed with rope at every joint.
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
    // Cross bars, with rope lashings where they meet the rails.
    for y in [5, 22] {
        for x in 3..13 {
            a.set(x, y, i('W'));
            a.set(x, y + 1, i('w'));
        }
    }
    for y in [5, 8, 20, 22] {
        for x in [3, 4, 11, 12] {
            a.set(x, y, if (x + y) % 2 == 0 { i('c') } else { i('W') });
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
    // Turns of rope binding the stone to the handle, lit on top.
    for y in [4, 7, 10] {
        for x in 5..10 {
            a.set(x, y, i('c'));
            a.set(x, y + 1, i('W'));
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

/// Half a coconut husk, 16x16: a cup of shaggy brown fibre, the dark shell
/// showing inside, loose strands sticking out of the torn rim.
pub fn icon_husk() -> PixelArt {
    let i = |c| ix(ITEM_LEGEND, c);
    let mut a = PixelArt::filled(16, 16, 0);
    // The husk, rounder at the bottom, streaked with fibre.
    fill_ellipse(&mut a, 8.0, 9.0, 6.2, 5.0, |x, y, nx, ny| {
        if ny < -0.25 {
            return None;
        }
        let streak = (x * 3 + y) % 4 == 0;
        Some(if streak { i('d') } else { ramp(&[i('d'), i('w')], sphere_light(nx, ny), x, y, 0.5) })
    });
    // The open top: the fibre's pale cut edge round the dark shell inside.
    fill_ellipse(&mut a, 8.0, 7.6, 6.2, 2.4, |_, _, nx, ny| Some(if (nx * nx + ny * ny).sqrt() > 0.6 { i('h') } else { i('d') }));
    // Strands sticking out of the rim.
    for (x, y) in [(2, 5), (4, 4), (12, 4), (14, 5), (13, 3)] {
        a.set(x, y, i('W'));
    }
    a.outlined(i('o'))
}

/// A coil of coir rope, 16x16: loops of tan cord with the twist showing as
/// dark diagonal marks, and a loose end hanging off.
pub fn icon_rope() -> PixelArt {
    let i = |c| ix(ITEM_LEGEND, c);
    let mut a = PixelArt::filled(16, 16, 0);
    let (cx, cy) = (8.0f32, 7.5f32);
    for y in 0..16 {
        for x in 0..16 {
            let (dx, dy) = ((x as f32 + 0.5 - cx) / 6.4, (y as f32 + 0.5 - cy) / 5.4);
            let r = (dx * dx + dy * dy).sqrt();
            // Two loops of cord with a dark gap between them, round a hole.
            if !(0.42..=1.0).contains(&r) {
                continue;
            }
            let loop_at = (r - 0.42) / 0.58 * 2.0;
            let inside = loop_at.fract();
            let twist = (x + y + if loop_at < 1.0 { 0 } else { 2 }) % 4 == 0;
            let c = if (0.95..1.12).contains(&loop_at) {
                i('d')
            } else if twist {
                i('w')
            } else if dy < 0.0 && inside < 0.6 {
                i('h')
            } else {
                i('W')
            };
            a.set(x, y, c);
        }
    }
    // The loose end trailing down to the right.
    for (x, y) in [(13, 11), (14, 12), (14, 13), (15, 14)] {
        a.set(x, y, if (x + y) % 3 == 0 { i('w') } else { i('W') });
    }
    a.outlined(i('o'))
}

/// A palm frond standing upright, 16x32: a pale midrib curving a little,
/// leaflets sweeping down from it on both sides, shorter toward the tip.
pub fn icon_palm_frond() -> PixelArt {
    let i = |c| ix(FX_LEGEND, c);
    let mut a = PixelArt::filled(16, 32, 0);
    let rib = |y: i32| 8 + (if y < 10 { (10 - y) / 4 } else { 0 });
    // Leaflets: every other row a pair, slanting down and out.
    for y0 in (2..27).step_by(2) {
        let len = 2 + (y0 - 2) * 5 / 24;
        for side in [-1i32, 1] {
            for k in 1..=len {
                let (x, y) = (rib(y0) + side * k, y0 + k / 2);
                if (0..16).contains(&x) && y < 30 {
                    a.set(x as usize, y as usize, if side < 0 { i('h') } else { i('G') });
                    a.set(x as usize, (y + 1) as usize, i('g'));
                }
            }
        }
    }
    for y in 1..31 {
        a.set(rib(y) as usize, y as usize, if y > 26 { i('w') } else { i('W') });
    }
    a.outlined(i('o'))
}

/// A thatch panel, 32x16: rows of frond leaflets laid overlapping, held by
/// two cords of vine.
pub fn icon_thatch() -> PixelArt {
    let i = |c| ix(FX_LEGEND, c);
    let mut a = PixelArt::filled(32, 16, 0);
    for y in 2..14 {
        for x in 1..31 {
            let row = (y - 2) / 4;
            let stroke = (x + (y - 2) % 4 + row * 2) % 4;
            let c = if (y - 2) % 4 == 3 { i('g') } else if stroke == 0 { i('h') } else { i('G') };
            a.set(x, y, c);
        }
    }
    for x in [8, 23] {
        for y in 1..15 {
            a.set(x, y, if y % 2 == 0 { i('g') } else { i('G') });
            a.set(x + 1, y, i('g'));
        }
    }
    a.outlined(i('o'))
}

/// The shelter rolled up to carry, 32x32: a bundle of thatch tied with
/// rope round two poles that stick out past its ends.
pub fn icon_shelter() -> PixelArt {
    let i = |c| ix(FX_LEGEND, c);
    let mut a = PixelArt::filled(32, 32, 0);
    // Poles behind the roll.
    for x in 1..31 {
        for (y, c) in [(12, 'W'), (13, 'w'), (19, 'W'), (20, 'w')] {
            a.set(x, y, i(c));
        }
    }
    // The roll of thatch, lit on top.
    for y in 8..25 {
        for x in 5..27 {
            let ny = (y as f32 - 16.0) / 8.5;
            let c = if (x + y) % 5 == 0 { i('g') } else if ny < -0.4 { i('h') } else if ny < 0.4 { i('G') } else { i('g') };
            a.set(x, y, c);
        }
    }
    // Its ends: the coil of fronds.
    for cx in [5.0f32, 26.0] {
        fill_ellipse(&mut a, cx, 16.5, 2.4, 8.4, |_, _, nx, ny| Some(if ((nx * nx + ny * ny).sqrt() * 3.0) as i32 % 2 == 0 { i('h') } else { i('g') }));
    }
    // Rope ties.
    for x in [11, 20] {
        for y in 7..26 {
            a.set(x, y, if y % 2 == 0 { i('c') } else { i('W') });
            a.set(x + 1, y, i('W'));
        }
    }
    a.outlined(i('o'))
}

/// A stone knife, 16x16: a long flake knapped to a point, lying on the
/// diagonal, its chipped cutting edge pale and the back left rough.
pub fn icon_stone_knife() -> PixelArt {
    let i = |c| ix(FX_LEGEND, c);
    let mut a = PixelArt::filled(16, 16, 0);
    for y in 0..16 {
        for x in 0..16 {
            // Along the blade (butt bottom-left to point top-right) and
            // across it (edge on the lower right).
            let (u, v) = ((x as f32 + 0.5 + (15.5 - y as f32)) / 2.0, (x as f32 - (15.5 - y as f32)) / 2.0);
            let half = if u < 5.0 { 2.4 } else { 2.4 * (14.0 - u).max(0.0) / 9.0 };
            if !(3.0..14.0).contains(&u) || v.abs() > half {
                continue;
            }
            let c = if v > half - 0.9 {
                if (x + y) % 2 == 0 { 'L' } else { 'M' }
            } else if v < -half + 0.9 || (x * 3 + y) % 5 == 0 {
                'm'
            } else {
                'M'
            };
            a.set(x, y, i(c));
        }
    }
    a.outlined(i('o'))
}

/// A coconut cut open, 16x16: half its shell seen from above, a ring of
/// white flesh inside the dark shell and husk, milk shining in the hollow.
pub fn icon_opened_coconut() -> PixelArt {
    let i = |c| ix(ITEM_LEGEND, c);
    let mut a = PixelArt::filled(16, 16, 0);
    // The outside of the half, below the rim.
    fill_ellipse(&mut a, 8.0, 9.5, 6.2, 5.2, |x, y, nx, ny| {
        if ny < 0.0 {
            return None;
        }
        Some(ramp(&[i('d'), i('w'), i('W')], sphere_light(nx, ny) * 0.9, x, y, 0.5))
    });
    // The cut face: husk, then shell, then flesh round the milky hollow.
    fill_ellipse(&mut a, 8.0, 8.5, 6.2, 3.6, |_, _, nx, ny| {
        let r = (nx * nx + ny * ny).sqrt();
        Some(if r > 0.86 {
            i('W')
        } else if r > 0.72 {
            i('d')
        } else if r > 0.45 {
            if nx + ny < 0.0 { i('f') } else { i('F') }
        } else if nx < -0.1 && ny < -0.1 {
            i('f')
        } else {
            i('F')
        })
    });
    a.outlined(i('o'))
}

/// A scallop shell, 16x16: a fan of pale coral ribs from a hinge with two
/// little ears, the crab palette's pinks.
pub fn icon_sea_shell() -> PixelArt {
    let i = |c| ix(crate::assets::palettes::CRAB_LEGEND, c);
    let mut a = PixelArt::filled(16, 16, 0);
    let (cx, cy) = (8.0f32, 13.0f32);
    for y in 0..16 {
        for x in 0..16 {
            let (dx, dy) = (x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);
            let r = (dx * dx + dy * dy).sqrt();
            let ang = dy.atan2(dx);
            // A fan opening upward, rounded at the rim.
            if dy > 0.0 || r > 10.5 || !(-2.6..=-0.54).contains(&ang) {
                continue;
            }
            let rib = ((ang + 2.6) * 9.0 / 2.06) as i32 % 2 == 0;
            let lit = dx < 0.0;
            a.set(x, y, if rib { if lit { i('w') } else { i('l') } } else if lit { i('l') } else { i('q') });
        }
    }
    // The ears at the hinge.
    for (x, y) in [(5, 12), (6, 12), (10, 12), (11, 12), (6, 13), (7, 13), (8, 13), (9, 13), (10, 13)] {
        a.set(x, y, i('q'));
    }
    a.outlined(i('o'))
}

/// A fish hook ground from shell, 16x16: a pale curved J with a barb, its
/// shank bound with a twist of cord.
pub fn icon_shell_hook() -> PixelArt {
    let i = |c| ITEM_LEGEND.iter().chain(crate::assets::palettes::CRAB_LEGEND).find(|(k, _)| *k == c).map(|(_, v)| *v).unwrap_or(0);
    PixelArt::parse(
        &[
            "................",
            "......oooo......",
            "......oWdo......",
            "......odWo......",
            "......oWdo......",
            "......olwo......",
            "......olwo......",
            "......olwo......",
            "..oo..olwo......",
            ".olwo.olwo......",
            ".olqo.olwo......",
            ".olwoolwqo......",
            "..oqlllqo.......",
            "...oqqqo........",
            "....ooo.........",
            "................",
        ],
        &[('o', i('o')), ('W', i('W')), ('d', i('d')), ('l', i('l')), ('w', i('w')), ('q', i('q'))],
    )
}

/// A fishing rod, 16x48: a slim branch, bending at the tip, with the rope
/// line running down from it to a shell hook.
pub fn icon_fishing_rod() -> PixelArt {
    let i = |c| ix(FX_LEGEND, c);
    let mut a = PixelArt::filled(16, 48, 0);
    // The rod, thick at the butt and leaning right toward the tip.
    for y in 4..46 {
        let x = 5 + (46 - y) / 14;
        a.set(x as usize, y as usize, if y % 9 == 2 { i('w') } else { i('W') });
        if y > 24 {
            a.set(x as usize + 1, y as usize, i('w'));
        }
    }
    // The line from the tip down to the hook.
    for y in 5..30 {
        a.set(11, y, i('L'));
    }
    for (x, y) in [(10, 3), (9, 4), (11, 4)] {
        a.set(x, y, i('L'));
    }
    // The hook.
    for (x, y) in [(11, 30), (11, 31), (12, 32), (13, 31), (13, 30)] {
        a.set(x, y, i('c'));
    }
    a.outlined(i('o'))
}

/// A silver fish, 16x16, swimming left: dark blue back, pale belly, a
/// forked tail and an eye.
pub fn icon_fish() -> PixelArt {
    let i = |c| ix(FX_LEGEND, c);
    let mut a = PixelArt::filled(16, 16, 0);
    fill_ellipse(&mut a, 7.0, 8.0, 5.6, 3.0, |x, y, nx, ny| {
        let _ = (x, y, nx);
        Some(if ny < -0.45 { i('b') } else if ny < 0.0 { i('M') } else if ny < 0.5 { i('L') } else { i('i') })
    });
    // Tail.
    for (x, y, c) in [(12, 6, 'b'), (13, 5, 'b'), (14, 4, 'm'), (12, 8, 'M'), (13, 8, 'M'), (12, 10, 'L'), (13, 11, 'L'), (14, 12, 'm'), (12, 7, 'M'), (12, 9, 'L')] {
        a.set(x, y, i(c));
    }
    let mut a = a.outlined(i('o'));
    a.set(3, 7, i('o'));
    a
}

/// A grilled fish, 16x16: the same shape browned, with dark grill marks.
pub fn icon_grilled_fish() -> PixelArt {
    let fx = icon_fish();
    let i = |c| ITEM_LEGEND.iter().find(|(k, _)| *k == c).map(|(_, v)| *v).unwrap_or(0);
    let f = |c| ix(FX_LEGEND, c);
    let mut a = PixelArt::filled(16, 16, 0);
    for y in 0..16 {
        for x in 0..16 {
            let v = fx.get(x, y);
            let out = if v == 0 {
                0
            } else if v == f('o') {
                i('o')
            } else if (x + y) % 4 == 0 && (3..12).contains(&x) {
                i('d')
            } else if v == f('b') || v == f('m') {
                i('w')
            } else if v == f('i') {
                i('h')
            } else {
                i('W')
            };
            a.set(x, y, out);
        }
    }
    a
}

/// A ladder standing upright, 16x64: two poles lit on the left, branch
/// rungs across them, and rope lashed round where they cross.
pub fn icon_ladder() -> PixelArt {
    ladder_art(16, 64)
}

/// The ladder lying on the ground, 32x16, rungs running up and down.
pub fn ladder_lying() -> PixelArt {
    ladder_art(16, 32).rotated_cw()
}

/// A ladder `w` wide and `h` tall (upright) in the effects palette.
fn ladder_art(w: usize, h: usize) -> PixelArt {
    let i = |c| ix(FX_LEGEND, c);
    let mut a = PixelArt::filled(w, h, 0);
    let (l, r) = (3, w - 5);
    // Rungs every 7 px, between the poles.
    let rungs: Vec<usize> = (5..h - 2).step_by(7).collect();
    for &y in &rungs {
        for x in l + 2..r {
            a.set(x, y, i('W'));
            a.set(x, y + 1, i('w'));
        }
    }
    // Poles over the rungs' ends.
    for y in 1..h - 1 {
        a.set(l, y, i('W'));
        a.set(l + 1, y, i('w'));
        a.set(r, y, i('W'));
        a.set(r + 1, y, i('w'));
    }
    // Rope lashings where rungs cross the poles.
    for &y in &rungs {
        for x in [l, l + 1, r, r + 1] {
            a.set(x, y, i('c'));
            a.set(x, y + 1, if x % 2 == 0 { i('c') } else { i('W') });
        }
    }
    a.outlined(i('o'))
}

/// A log bridge standing upright, 16x64: two pine logs side by side, lit
/// on the left, cut ends at the top, lashed with rope near both ends.
pub fn icon_log_bridge() -> PixelArt {
    log_bridge_art(64)
}

/// The log bridge lying on the ground, 32x16, logs running across.
pub fn log_bridge_lying() -> PixelArt {
    log_bridge_art(32).rotated_cw()
}

/// Two pine logs 16 wide and `h` tall (upright), in pine's item colours
/// (see `as_pine`: `w` dark, `W` mid, `h` light, `f` end grain).
fn log_bridge_art(h: usize) -> PixelArt {
    let i = |c| ix(ITEM_LEGEND, c);
    let mut a = PixelArt::filled(16, h, 0);
    let end = 3;
    let lashings = [h / 6, h - 1 - h / 6];
    for left in [1usize, 8] {
        for y in 1..h - 1 {
            for dx in 0..6 {
                let c = if y < 1 + end {
                    // The cut end: pale wood with a darker ring.
                    if (dx == 1 || dx == 4) && y == 2 || (dx == 2 || dx == 3) && (y == 1 || y == end) { 'h' } else { 'f' }
                } else if lashings.iter().any(|&l| (l..l + 2).contains(&y)) {
                    // Rope wound round both logs.
                    if (y + dx) % 2 == 0 { 'w' } else { 'd' }
                } else {
                    match dx {
                        0 => 'h',
                        1..=3 => if (y * 7 + left * 3 + dx * 5) % 11 == 0 { 'w' } else { 'W' },
                        _ => 'w',
                    }
                };
                a.set(left + dx, y, i(c));
            }
        }
    }
    a.outlined(i('o'))
}

/// A pine cone, 16x16: an egg of brown woody scales in rows, overlapping
/// downward, lit from the top-left, with its stalk at the top.
pub fn icon_pine_cone() -> PixelArt {
    let i = |c| ix(ITEM_LEGEND, c);
    let mut a = PixelArt::filled(16, 16, 0);
    let (cx, cy, rx, ry) = (8.0f32, 8.5f32, 4.6f32, 6.0f32);
    for y in 0..16 {
        for x in 0..16 {
            let (nx, ny) = ((x as f32 + 0.5 - cx) / rx, (y as f32 + 0.5 - cy) / ry);
            // Narrower toward the stalk than at the base.
            let nx = nx * (1.0 + (-ny).max(0.0) * 0.35);
            if nx * nx + ny * ny > 1.0 {
                continue;
            }
            // Scales in staggered rows: each a lit tip over a dark seam.
            let row = (y + 1) / 3;
            let col = (x + row % 2 * 2) / 4;
            let in_scale = (y + 1) % 3;
            let lit = nx < 0.1 && ny < 0.4;
            let v = if in_scale == 0 || (x + row % 2 * 2) % 4 == 0 {
                'd'
            } else if in_scale == 1 && (lit || col % 2 == 0) {
                'W'
            } else if lit {
                'w'
            } else {
                'd'
            };
            a.set(x, y, i(v));
        }
    }
    // The stalk.
    a.set(8, 1, i('d'));
    a.set(8, 2, i('d'));
    a.outlined(i('o'))
}

/// A length of bamboo standing upright, 16x48: a green culm with pale
/// joints, cut square at both ends (the hollow showing at the top).
pub fn icon_bamboo() -> PixelArt {
    let i = |c| ix(FX_LEGEND, c);
    let mut a = PixelArt::filled(16, 48, 0);
    for y in 3..45 {
        let joint = (y + 4) % 13;
        for (k, x) in (6..10).enumerate() {
            let c = match (joint, k) {
                (0, _) => i('g'),
                (1, 0..=2) => i('h'),
                (_, 0) => i('h'),
                (_, 3) => i('g'),
                _ => i('G'),
            };
            a.set(x, y, c);
        }
    }
    // The cut end: a pale rim round the dark hollow.
    for x in 6..10 {
        a.set(x, 2, i('c'));
    }
    a.set(7, 2, i('w'));
    a.set(8, 2, i('w'));
    a.outlined(i('o'))
}

/// A digging stick, 16x32: a straight branch, its foot whittled to a pale
/// point.
pub fn icon_digging_stick() -> PixelArt {
    let i = |c| ix(ITEM_LEGEND, c);
    let mut a = PixelArt::filled(16, 32, 0);
    for y in 2..30 {
        // Leaning a little, the point at the bottom.
        let cx = 9.5 - (y as f32 - 2.0) * 0.09;
        let point = y >= 23;
        let r = if point { 1.6 * (30.0 - y as f32) / 7.0 } else { 1.6 };
        for x in 0..16 {
            let dx = x as f32 + 0.5 - cx;
            if dx.abs() <= r.max(0.5) {
                let c = if point {
                    // Fresh-cut wood where the knife took the bark off.
                    if dx < 0.0 { i('h') } else { i('F') }
                } else if dx < -0.5 {
                    i('W')
                } else if dx > 0.6 || y % 8 == 5 {
                    i('d')
                } else {
                    i('w')
                };
                a.set(x, y, c);
            }
        }
    }
    a.outlined(i('o'))
}

/// A bamboo spade, 16x48: a round culm for a handle, bound with rope where
/// the split begins, then the split half opening into a curved blade (its
/// pale inside showing) that ends at the node.
pub fn icon_bamboo_spade() -> PixelArt {
    let i = |c| ix(FX_LEGEND, c);
    let mut a = PixelArt::filled(16, 48, 0);
    // The handle: a culm with a joint.
    for y in 2..26 {
        for (k, x) in (6..10).enumerate() {
            let c = match (y % 12 == 7, k) {
                (true, _) => i('g'),
                (_, 0) => i('h'),
                (_, 3) => i('g'),
                _ => i('G'),
            };
            a.set(x, y, c);
        }
    }
    for x in 6..10 {
        a.set(x, 1, i('c'));
    }
    // The blade: the split half widening, its pale inside up.
    for y in 26..45 {
        let t = ((y - 26) as f32 / 6.0).min(1.0);
        let half = 2.0 + t * 3.0;
        let end = if y >= 42 { (44 - y) as f32 * 0.9 + 2.5 } else { half };
        let half = half.min(end);
        for x in 0..16 {
            let dx = x as f32 + 0.5 - 8.0;
            if dx.abs() <= half {
                let edge = dx.abs() > half - 1.0;
                let c = if edge { if dx < 0.0 { i('h') } else { i('g') } } else if dx > half - 2.0 { i('W') } else { i('c') };
                a.set(x, y, c);
            }
        }
    }
    // The rope binding where the split stops.
    for y in 25..29 {
        for x in 5..11 {
            a.set(x, y, if (x + y) % 2 == 0 { i('W') } else { i('w') });
        }
    }
    a.outlined(i('o'))
}

/// Lumps of clay heaped up, 16x16: a pile of soft blue-grey lumps, each
/// with a little wet sheen, so it doesn't read as one stone.
pub fn icon_clay() -> PixelArt {
    let i = |c| ix(FX_LEGEND, c);
    let mut a = PixelArt::filled(16, 16, 0);
    // Back to front, bottom row last so it overlaps the ones above.
    for (cx, cy, rx, ry) in [(8.0, 5.5, 2.6, 2.0), (5.5, 8.5, 2.8, 2.2), (10.5, 8.5, 2.8, 2.2), (3.5, 12.0, 2.8, 2.0), (8.0, 11.8, 3.0, 2.3), (12.5, 12.0, 2.6, 2.0)] {
        fill_ellipse(&mut a, cx, cy, rx, ry, |x, y, nx, ny| {
            // A dark crease where each lump rests on its neighbours.
            if nx * nx + ny * ny > 0.75 {
                return Some(i('m'));
            }
            let light = sphere_light(nx, ny);
            Some(if light > 0.93 { i('L') } else if light > 0.55 || (light > 0.35 && (x + y) % 2 == 0) { i('M') } else { i('m') })
        });
    }
    a.outlined(i('o'))
}

/// Soil heaped up, 16x16: a mound of crumbly brown earth with a lumpy top,
/// clods on its slopes and crumbs rolled off at its foot.
pub fn icon_soil() -> PixelArt {
    let i = |c| ix(ITEM_LEGEND, c);
    let mut a = PixelArt::filled(16, 16, 0);
    for (cx, cy, rx, ry) in [(8.0, 6.0, 2.4, 2.0), (6.0, 8.5, 3.2, 2.5), (10.5, 8.8, 3.0, 2.4), (8.0, 11.5, 6.5, 2.6)] {
        fill_ellipse(&mut a, cx, cy, rx, ry, |x, y, nx, ny| {
            if nx * nx + ny * ny > 0.8 {
                return Some(i('d'));
            }
            let light = sphere_light(nx, ny) * 0.8 + 0.1;
            let grit = (x * 13 + y * 7) % 7 == 0;
            Some(if grit { i('d') } else { ramp(&[i('d'), i('w'), i('W')], light, x, y, 0.5) })
        });
    }
    // Clods and a pebble in it.
    for (x, y) in [(5, 8), (10, 7), (12, 11), (4, 11)] {
        a.set(x, y, i('W'));
    }
    a.set(8, 10, i('s'));
    // Crumbs rolled off at its foot.
    a.set(1, 13, i('w'));
    a.set(14, 13, i('w'));
    a.outlined(i('o'))
}

/// The water wheel, 48x48, seen face on: a rim, arms and hub of lauan,
/// with paddles of split bamboo standing out round it.
pub fn icon_water_wheel() -> PixelArt {
    let i = |c| ix(FX_LEGEND, c);
    let mut a = PixelArt::filled(48, 48, 0);
    water_wheel_disc(&mut a, 24.0, 24.0, 22.5, 22.5, 0.0);
    a.outlined(i('o'))
}

/// The water wheel lying on the ground, 32x16: the wheel seen flat from
/// above at a slant.
pub fn water_wheel_lying() -> PixelArt {
    let i = |c| ix(FX_LEGEND, c);
    let mut a = PixelArt::filled(32, 16, 0);
    water_wheel_disc(&mut a, 16.0, 8.0, 14.5, 6.5, 0.0);
    a.outlined(i('o'))
}

/// Paints a water wheel's face round `(cx, cy)`, `rx` across and `ry` up
/// and down, turned `turn` radians, in the effects palette: wood for the
/// lauan, green for the bamboo.
pub fn water_wheel_disc(a: &mut PixelArt, cx: f32, cy: f32, rx: f32, ry: f32, turn: f32) {
    let i = |c| ix(FX_LEGEND, c);
    for y in 0..a.h {
        for x in 0..a.w {
            if let Some(part) = water_wheel_pixel(x as f32 + 0.5 - cx, y as f32 + 0.5 - cy, rx, ry, turn) {
                a.set(
                    x,
                    y,
                    i(match part {
                        // Lauan for the rim, arms and hub; split bamboo for
                        // the paddles.
                        WheelPart::RimLit | WheelPart::Spoke => 'W',
                        WheelPart::Rim => 'w',
                        WheelPart::Hub => 'c',
                        WheelPart::Paddle => 'G',
                        WheelPart::PaddleLit => 'h',
                    }),
                );
            }
        }
    }
}

/// What part of a water wheel's face is at a point.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WheelPart {
    /// The rim of split bamboo, lit on the upper left, and in shade.
    RimLit,
    Rim,
    /// A paddle set across the rim, in shade and lit.
    Paddle,
    PaddleLit,
    Spoke,
    /// The lashed hub on the axle.
    Hub,
}

/// The part of a water wheel's face at `(dx, dy)` from its middle, if any
/// of it is there: a broad rim with eight paddles set across it (inside
/// it, as on a real wheel), eight spokes between them and a hub, `rx`
/// across and `ry` up and down, turned `turn` radians.
pub fn water_wheel_pixel(dx: f32, dy: f32, rx: f32, ry: f32, turn: f32) -> Option<WheelPart> {
    use std::f32::consts::{FRAC_PI_4, PI};
    let (nx, ny) = (dx / rx, dy / ry);
    let r = (nx * nx + ny * ny).sqrt();
    if r > 1.0 {
        return None;
    }
    // How far round from the nearest of eight evenly spaced lines (px at
    // that radius): the paddles on the eighths, the spokes between.
    let angle = ny.atan2(nx) - turn;
    let off = |phase: f32| {
        let a = (angle - phase).rem_euclid(FRAC_PI_4);
        a.min(FRAC_PI_4 - a) * r * rx.max(ry)
    };
    let lit = nx + ny < -0.2;
    // The broad rim, the paddles set across it between its two sides (so
    // none stand out past it), seen face on as boards dividing it.
    if r > 0.7 {
        if off(0.0) < 1.3 && r < 0.97 {
            return Some(if lit { WheelPart::PaddleLit } else { WheelPart::Paddle });
        }
        return Some(if lit { WheelPart::RimLit } else { WheelPart::Rim });
    }
    if r < 0.2 {
        return Some(WheelPart::Hub);
    }
    if off(PI / 8.0) < 0.7 {
        return Some(WheelPart::Spoke);
    }
    None
}

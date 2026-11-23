//! Sprite artwork. Hand-drawn sprites use text with a per-palette legend
//! (see `palettes::*_LEGEND`); `.` is transparent.

use super::paint::{bayer, fill_ellipse, sphere_light};
use super::palettes::{CRAB_LEGEND, FX_LEGEND, GULL_LEGEND, ITEM_LEGEND, LAUAN_LEGEND};
use crate::ppu::pattern::PixelArt;

// Crab, 16x16. o outline · r/R/q/l shell dark→light · w eye · e pupil.

const CRAB_TOP: [&str; 13] = [
    "................",
    "................",
    "................",
    ".ooo........ooo.",
    "oqlqo......oqlqo",
    "oRqRo.w..w.oRqRo",
    ".oRRo.e..e.oRRo.",
    "..oRooqqqqooRo..",
    "...ooqllllqoo...",
    "..oRqqlqqlqqRo..",
    ".oRRRqqqqqqRRRo.",
    ".orRRRRRRRRRRro.",
    "..orrRRRRRRrro..",
];

pub fn crab_frames() -> [PixelArt; 2] {
    let make = |legs: [&'static str; 3]| {
        let mut rows = CRAB_TOP.to_vec();
        rows.extend_from_slice(&legs);
        PixelArt::parse(&rows, CRAB_LEGEND)
    };
    [make([".o.oorrrrrroo.o.", "o.o.o.oooo.o.o.o", "................"]), make(["o..oorrrrrroo..o", ".o.o..oooo..o.o.", "................"])]
}

// Seagull seen from above, 16x16. o outline · d/g/w/W greys→white ·
// y/Y beak · k wing tips.

pub fn gull_frames() -> [PixelArt; 2] {
    let wings_out = [
        "................",
        "................",
        "................",
        ".......oo.......",
        "......oYyo......",
        "......oWWo......",
        "kkoooowWWwooookk",
        "kddgggwWWwgggddk",
        ".kkddgwWWwgddkk.",
        "....oo.wgo.oo...",
        "......owgo......",
        "......oddo......",
        ".......oo.......",
        "................",
        "................",
        "................",
    ];
    let wings_in = [
        "................",
        "................",
        "................",
        ".......oo.......",
        "......oYyo......",
        "......oWWo......",
        "..ooo.wWWw.ooo..",
        ".kdgggwWWwgggdk.",
        "..kkdgwWWwgdkk..",
        "....oo.wgo.oo...",
        "......owgo......",
        "......oddo......",
        ".......oo.......",
        "................",
        "................",
        "................",
    ];
    [PixelArt::parse(&wings_out, GULL_LEGEND), PixelArt::parse(&wings_in, GULL_LEGEND)]
}

/// Soft elliptical drop shadow (palette index 1 of the shadow palette).
pub fn shadow(w: usize, h: usize, rx: f32, ry: f32) -> PixelArt {
    let mut a = PixelArt::filled(w, h, 0);
    fill_ellipse(&mut a, w as f32 / 2.0, h as f32 / 2.0, rx, ry, |x, y, nx, ny| {
        let e = nx * nx + ny * ny;
        if e > 0.7 && bayer(x, y) < (e - 0.7) * 3.0 { None } else { Some(1) }
    });
    a
}

/// Expanding ring of foam around wading legs (water-fx palette).
pub fn wade_ripples() -> [PixelArt; 3] {
    let ring = |rx: f32, ry: f32| {
        let mut a = PixelArt::filled(24, 8, 0);
        fill_ellipse(&mut a, 12.0, 4.0, rx, ry, |_, _, nx, ny| {
            let e = (nx * nx + ny * ny).sqrt();
            if e > 0.72 {
                Some(if ny < 0.0 { 2 } else { 1 })
            } else if e > 0.5 {
                Some(3)
            } else {
                None
            }
        });
        a
    };
    [ring(7.5, 2.6), ring(9.0, 3.0), ring(10.5, 3.4)]
}

// Tools & effects (OBJ_FX: o outline · w/W wood · m/M/L metal · c cut wood ·
// g/G/h leaf · y/Y spark).

/// Iron axe lying horizontally, head on the right with the edge facing
/// right. Kept for when iron tools can be made.
#[allow(dead_code)]
pub fn iron_axe_side() -> PixelArt {
    PixelArt::parse(
        &[
            "................",
            "................",
            "................",
            "................",
            "..........ooo...",
            ".........omMLo..",
            ".........omMMLo.",
            "oooooooooomMMLo.",
            "oWWWWWWWWwmMMLo.",
            "owwwwwwwwwmMMLo.",
            "oooooooooomMMLo.",
            ".........omMLo..",
            "..........ooo...",
            "................",
            "................",
            "................",
        ],
        FX_LEGEND,
    )
}

/// Stone axe lying horizontally: a knapped stone head lashed with rope to
/// the end of a branch, edge facing right. The handle lies where the iron
/// axe's does, so both share the grip offsets.
pub fn stone_axe_side() -> PixelArt {
    PixelArt::parse(
        &[
            "................",
            "................",
            "................",
            "............oo..",
            "...........oMLo.",
            "..........oMMLo.",
            "..........oMmMLo",
            "ooooooooocWMmMLo",
            "oWWWWWWWWcWmMmLo",
            "owwwwwwwwWcmmmLo",
            "ooooooooocWmmMLo",
            "..........ommLo.",
            "...........omLo.",
            "............oo..",
            "................",
            "................",
        ],
        FX_LEGEND,
    )
}

/// Wooden spear lying horizontally, 32x8: a straight branch with its end
/// whittled to a pale point (right). The hands hold it 8px from the butt.
pub fn spear_side() -> PixelArt {
    PixelArt::parse(
        &[
            "................................",
            "................................",
            "oooooooooooooooooooooooooooooo..",
            "oWWWWWWWWWWWWWWWWWWWWWWWWWcccco.",
            "owwwwwwwwwwwwwwwwwwwwwwwwwccoo..",
            "oooooooooooooooooooooooooooo....",
            "................................",
            "................................",
        ],
        FX_LEGEND,
    )
}

// Campfire (crab palette: its reds for flames, the item browns and stone
// greys for the ring).

/// A ring of stones round two crossed sticks with glowing embers, 16x8.
pub fn fire_ring() -> PixelArt {
    let i = |c| ITEM_LEGEND.iter().chain(CRAB_LEGEND).find(|(k, _)| *k == c).map(|(_, v)| *v).unwrap_or(0);
    let mut a = PixelArt::filled(16, 8, 0);
    // Crossed sticks, charred at the middle.
    for x in 3..13 {
        let t = (x - 3) as f32 / 9.0;
        let (y0, y1) = ((2.0 + t * 3.0).round() as usize, (5.0 - t * 3.0).round() as usize);
        a.set(x, y0, if (6..10).contains(&x) { i('d') } else { i('W') });
        a.set(x, y1, if (6..10).contains(&x) { i('d') } else { i('w') });
    }
    for (x, y) in [(7, 3), (8, 4), (7, 4), (9, 3)] {
        a.set(x, y, crab_ix('q'));
    }
    a.set(8, 3, crab_ix('l'));
    // Stones round the rim, lit on top; the far ones sit higher.
    for k in 0..8 {
        let ang = k as f32 / 8.0 * std::f32::consts::TAU;
        let (cx, cy) = (8.0 + ang.cos() * 6.3, 4.0 + ang.sin() * 2.6);
        fill_ellipse(&mut a, cx, cy, 1.6, 1.2, |_, _, _, ny| Some(if ny < -0.3 { i('F') } else { i('s') }));
    }
    a.outlined(i('o'))
}

fn crab_ix(c: char) -> u8 {
    CRAB_LEGEND.iter().find(|(k, _)| *k == c).map(|(_, v)| *v).unwrap_or(0)
}

/// A lit torch, 16x32 per frame: a branch handle, a head of husk fibre
/// bound round its top, and a flame licking up from it (three frames),
/// lively or (`low`) burning down. Held at `TORCH_GRIP`.
pub fn torch_frames(low: bool) -> [PixelArt; 3] {
    let i = |c| ITEM_LEGEND.iter().find(|(k, _)| *k == c).map(|(_, v)| *v).unwrap_or(0);
    let mut body = PixelArt::filled(16, 32, 0);
    // The handle, leaning a touch, lit from the left.
    for y in 15..31 {
        let cx = 8.0 - (y as f32 - 15.0) * 0.06;
        for x in 0..16 {
            let dx = x as f32 + 0.5 - cx;
            if dx.abs() <= 0.9 {
                let c = if dx < 0.0 {
                    i('W')
                } else if y % 7 == 3 {
                    i('d')
                } else {
                    i('w')
                };
                body.set(x, y, c);
            }
        }
    }
    // The fibre head: a fat bundle, streaked, with a binding round its foot.
    fill_ellipse(&mut body, 8.0, 14.5, 2.4, 3.2, |x, y, nx, _| {
        Some(if y == 17 || (x * 3 + y) % 4 == 0 || nx > 0.5 {
            i('d')
        } else if nx < -0.4 {
            i('h')
        } else {
            i('w')
        })
    });
    let body = body.outlined(i('o'));
    std::array::from_fn(|f| {
        let mut a = PixelArt::filled(16, 32, 0);
        // A slim flame standing on the head, its foot hidden in the fibre.
        let (height, width) = if low { (4.5, 1.8) } else { (9.0, 2.4) };
        a.overlay(&flames(height, width)[f], 0, -3);
        let mut out = body.clone();
        // Charred, glowing top of the fibre under the flame.
        for (x, y) in [(7, 11), (8, 11), (9, 12), (7, 12)] {
            out.set(x, y, crab_ix(if (x + f) % 2 == 0 { 'q' } else { 'R' }));
        }
        out.overlay(&a, 0, 0);
        out
    })
}

/// Where the hand holds the torch in its image.
pub const TORCH_GRIP: (f32, f32) = (7.5, 23.0);

/// Flickering flames, 16x16 per frame, standing on the bottom edge: three
/// frames of a lively fire (`height` ~12) or a dying one (~6).
pub fn flames(height: f32, width: f32) -> [PixelArt; 3] {
    std::array::from_fn(|f| {
        let mut a = PixelArt::filled(16, 16, 0);
        let phase = f as f32 * 2.1;
        for y in 0..16 {
            for x in 0..16 {
                let up = 15.5 - y as f32;
                // Three tongues of different heights, swaying with the frame.
                let tongue = |off: f32, h: f32, k: f32| {
                    let h = h * (1.0 + 0.12 * (phase + k).sin());
                    if up > h {
                        return None;
                    }
                    let along = up / h;
                    let sway = (along * 2.6 + phase + k).sin() * 1.3 * along;
                    let half = width * (1.0 - along).powf(0.7) * (0.6 + 0.4 * (1.0 - along));
                    let d = (x as f32 + 0.5 - 8.0 - off - sway).abs() / half.max(0.01);
                    (d <= 1.0).then_some((d, along))
                };
                let best = [tongue(0.0, height, 0.0), tongue(-2.6, height * 0.62, 1.7), tongue(2.4, height * 0.7, 3.1)]
                    .into_iter()
                    .flatten()
                    .min_by(|p, q| p.0.total_cmp(&q.0));
                let Some((d, along)) = best else { continue };
                let heat = (1.0 - d) * (1.0 - along * 0.8);
                let c = if heat > 0.55 {
                    'w'
                } else if heat > 0.36 {
                    'l'
                } else if heat > 0.18 {
                    'q'
                } else if d < 0.9 || bayer(x as i32, y as i32) < 0.5 {
                    'R'
                } else {
                    'r'
                };
                a.set(x, y, crab_ix(c));
            }
        }
        a
    })
}

/// A crab sprite (crab palette) cooked: the shell turns a brighter orange
/// with dark grill marks across it.
pub fn grilled(crab: &PixelArt) -> PixelArt {
    let (r, big_r, q, l) = (crab_ix('r'), crab_ix('R'), crab_ix('q'), crab_ix('l'));
    let mut a = crab.map(|v| match v {
        v if v == r => big_r,
        v if v == big_r => q,
        v if v == q => l,
        v => v,
    });
    for y in 0..a.h {
        for x in 0..a.w {
            let v = a.at(x as i32, y as i32);
            if (v == q || v == l) && (x + 2 * y) % 6 == 0 {
                a.set(x, y, r);
            }
        }
    }
    a
}

/// A puff of dust (effects palette: pale with a sandy rim, so it shows
/// on sand too): round, then breaking up.
pub fn dust_frames() -> [PixelArt; 2] {
    [
        PixelArt::parse(&["........", "...cc...", "..cYYc..", ".cYYYYc.", ".cYYYcc.", "..cccc..", "........", "........"], FX_LEGEND),
        PixelArt::parse(&["..Y..c..", "........", "c.....Y.", "........", ".Y....c.", "........", "...c.Y..", "........"], FX_LEGEND),
    ]
}

/// A pack frame lying on its side on the ground, 16x8.
pub fn pack_frame() -> PixelArt {
    let rows = [
        "................",
        ".oooooooooooooo.",
        "oWWcWWWWWWWcWWWo",
        "ow.occcccco.w.wo",
        "oW.ocwcccwo.W.Wo",
        "owwcwwwwwwwcwwwo",
        ".oooooooooooooo.",
        "................",
    ];
    PixelArt::parse(&rows, FX_LEGEND)
}

/// A tool belt lying on the ground, 16x8: the plaited bark band in a
/// loose curve, the toggle at its left end and a rope loop on top.
pub fn tool_belt() -> PixelArt {
    let rows = [
        "................",
        "..........ooo...",
        ".oo......ocococ.",
        "oWWoooooooooowWo",
        "owWWwWWwWWwWWwwo",
        ".owwWwwWwwWwwwo.",
        "..oooooooooooco.",
        "............oco.",
    ];
    PixelArt::parse(&rows, FX_LEGEND)
}

/// A small Z drifting up from a sleeper, 8x8 (effects palette).
pub fn snore() -> PixelArt {
    let rows = ["........", ".ooooo..", ".oLLLo..", "..ooLo..", "..oLo...", ".oLooo..", ".oLLLo..", ".ooooo.."];
    PixelArt::parse(&rows, FX_LEGEND)
}

/// A few leaves lying on the ground, 8x8 (effects palette greens).
pub fn leaves() -> PixelArt {
    let rows = ["........", "....oo..", "..ooGho.", ".oGhGgo.", "ohGgGo..", "oGggoo..", ".ooo....", "........"];
    PixelArt::parse(&rows, FX_LEGEND)
}

/// A crab lying on its back, legs curled up, 16x8.
pub fn crab_caught() -> PixelArt {
    let rows = [
        "................",
        ".o.o.oooooo.o.o.",
        "o.oooqllllqooo.o",
        "..oRqllqqllqRo..",
        ".oRRqqllllqqRRo.",
        ".orRRRRRRRRRRro.",
        "..oorrrrrrrroo..",
        "................",
    ];
    PixelArt::parse(&rows, CRAB_LEGEND)
}

/// An axe (given lying side-on) held upright, head at the top with the
/// edge facing up.
pub fn axe_up(side: PixelArt) -> PixelArt {
    side.transposed().flipped_v()
}

/// An axe (given lying side-on) at 45°, head at the top right. Rotated
/// without its outline, then re-outlined so the edges stay clean.
pub fn axe_diag(side: PixelArt) -> PixelArt {
    let src = side.map(|v| if v == 1 { 0 } else { v });
    let mut out = PixelArt::filled(16, 16, 0);
    let (c, k) = (7.5f32, std::f32::consts::FRAC_1_SQRT_2);
    for y in 0..16 {
        for x in 0..16 {
            // Inverse-rotate the destination pixel by -45° into the source.
            let (dx, dy) = (x as f32 - c, y as f32 - c);
            let (sx, sy) = (dx * k - dy * k + c, dx * k + dy * k + c);
            out.put(x, y, src.at(sx.round() as i32, sy.round() as i32));
        }
    }
    out.outlined(1)
}

/// Translucent swoosh (water-fx palette): an arc of the ellipse centred at
/// (cx, cy) with radii (rx, ry), swept from `from`° to `to`° (screen angles,
/// 0 = right, 90 = down). It thickens toward the leading end.
pub fn swoosh_arc(w: usize, h: usize, (cx, cy): (f32, f32), (rx, ry): (f32, f32), from: f32, to: f32) -> PixelArt {
    let mut a = PixelArt::filled(w, h, 0);
    for y in 0..h {
        for x in 0..w {
            let (nx, ny) = ((x as f32 + 0.5 - cx) / rx, (y as f32 + 0.5 - cy) / ry);
            let r = (nx * nx + ny * ny).sqrt();
            let mut ang = ny.atan2(nx).to_degrees();
            // Bring the angle into the sweep's range.
            while ang < from.min(to) {
                ang += 360.0;
            }
            while ang > from.max(to) {
                ang -= 360.0;
            }
            let p = (ang - from) / (to - from);
            if !(0.0..=1.0).contains(&p) {
                continue;
            }
            let inner = 1.0 - (0.06 + 0.34 * p);
            if r <= 1.0 && r >= inner && !(p < 0.25 && super::paint::bayer(x as i32, y as i32) > p * 4.0) {
                a.set(x, y, if r > 1.0 - 0.12 { 2 } else { 1 });
            }
        }
    }
    a
}

/// Swooshes for a horizontal swing, indexed up, down, left, right.
pub fn swooshes() -> [PixelArt; 4] {
    // Side view: a flat arc passing in front of the body, back → front.
    let right = swoosh_arc(32, 16, (16.0, 5.0), (14.5, 7.0), 180.0, 0.0);
    [
        // Facing away: from the right side, sweeping up behind.
        swoosh_arc(32, 16, (16.0, 14.0), (13.0, 11.0), 0.0, -90.0),
        // Facing the camera: from the left side, sweeping down in front.
        swoosh_arc(32, 16, (16.0, 2.0), (13.0, 11.0), 180.0, 90.0),
        right.flipped_h(),
        right,
    ]
}

fn fx8(rows: [&str; 8]) -> PixelArt {
    PixelArt::parse(&rows, FX_LEGEND)
}

pub fn chip_frames() -> [PixelArt; 2] {
    [
        fx8(["........", "...oo...", "..oWco..", "..owWo..", "...oo...", "........", "........", "........"]),
        fx8(["........", "..oo....", ".ocWo...", "..owwo..", "...ooo..", "........", "........", "........"]),
    ]
}

pub fn leaf_frames() -> [PixelArt; 2] {
    [
        fx8(["........", "....oo..", "...oGho.", "..oGGo..", ".ogGo...", ".oo.....", "........", "........"]),
        fx8(["........", ".oo.....", ".ohGo...", "..oGGo..", "...oGgo.", "....oo..", "........", "........"]),
    ]
}

pub fn spark_frames() -> [PixelArt; 2] {
    [
        fx8(["...y....", "...y....", ".yyYyy..", "...y....", "...y....", "........", "........", "........"]),
        fx8(["y.....y.", ".y...y..", "..yYy...", ".y...y..", "y.....y.", "........", "........", "........"]),
    ]
}

/// Coconut, 8x8 (items share the crab palette, so they darken at night
/// like the crabs).
/// o outline · d/w/W husk dark→light · h highlight. The second frame is
/// turned a quarter for tumbling.
pub fn coconut_frames() -> [PixelArt; 2] {
    let rest = ["........", "..oooo..", ".oWhWwo.", "oWhWwwdo", "oWWwwwdo", "owwwwddo", ".odddddo", "..ooooo."];
    let turned = ["........", "..oooo..", ".owWWWo.", "odwWhWWo", "odwwWWWo", "oddwwwwo", ".oddddo.", "..oooo.."];
    [PixelArt::parse(&rest, ITEM_LEGEND), PixelArt::parse(&turned, ITEM_LEGEND)]
}

/// Timber (a squared length of palm wood) lying on its side, 16x8, cut end
/// to the right. Same legend as the coconut.
pub fn timber() -> PixelArt {
    let rows = [
        "................",
        ".oooooooooooooo.",
        "oWWWWWWhWWWWohho",
        "owwwwwwwwwwwohWo",
        "owwdwwwwwdwwoWho",
        "odddddddddddohho",
        ".oooooooooooooo.",
        "................",
    ];
    PixelArt::parse(&rows, ITEM_LEGEND)
}

/// A post of the mill's frame, rising `rise` px above the middle of its
/// stone's cell and standing `foot` px into it (item colours): a squared
/// lauan post, lit on its left, shaded on its right, its sawn top pale.
/// Eight pixels wide; tall enough for the post and its outline, the post
/// against the bottom (its foot one pixel above the bottom edge).
pub fn mill_post(rise: i32, foot: i32) -> PixelArt {
    let tall = (rise + foot) as usize;
    let h = (tall + 2).div_ceil(8) * 8;
    let top = h - 1 - tall;
    let ix = |c: char| ITEM_LEGEND.iter().find(|&&(k, _)| k == c).map_or(0, |&(_, i)| i);
    let mut a = PixelArt::filled(8, h, 0);
    for y in top..h - 1 {
        for (x, c) in [(2, 'W'), (3, 'w'), (4, 'w'), (5, 'd')] {
            a.set(x, y, ix(if y == top { 'h' } else { c }));
        }
    }
    a.outlined(ix('o'))
}

/// A beam of the mill's frame, `len` px from the north plate to the south
/// one (item colours): a squared pine beam, pale, lit on its left. Eight
/// pixels wide, the beam against the top (a pixel below the top edge).
pub fn mill_beam(len: i32) -> PixelArt {
    let h = (len as usize + 2).div_ceil(8) * 8;
    let ix = |c: char| ITEM_LEGEND.iter().find(|&&(k, _)| k == c).map_or(0, |&(_, i)| i);
    let mut a = PixelArt::filled(8, h, 0);
    for y in 1..=len as usize {
        for (x, c) in [(2, 'f'), (3, 'h'), (4, 'h'), (5, 'W')] {
            a.set(x, y, ix(c));
        }
    }
    a.outlined(ix('o'))
}

/// Wood art in the item colours redone in lauan's (gull palette).
pub fn as_lauan(art: &PixelArt) -> PixelArt {
    art.map(|v| {
        let key = ITEM_LEGEND.iter().find(|(_, i)| *i == v).map(|(k, _)| *k);
        key.and_then(|k| LAUAN_LEGEND.iter().find(|(c, _)| *c == k)).map(|(_, i)| *i).unwrap_or(v)
    })
}

/// A pine cone lying on the ground, 8x8: a small brown egg of scales.
pub fn pine_cone() -> PixelArt {
    let rows = ["...o....", "..odo...", ".owWdo..", ".odwWo..", ".oWdwo..", ".odwdo..", "..ooo...", "........"];
    PixelArt::parse(&rows, ITEM_LEGEND)
}

/// Wood art in the item colours redone pale, as pine (a lighter, yellower
/// wood than palm): each shade a step lighter, the lightest coconut-flesh
/// cream.
pub fn as_pine(art: &PixelArt) -> PixelArt {
    let shade = |k: char| ITEM_LEGEND.iter().find(|(c, _)| *c == k).map(|(_, i)| *i).unwrap_or(0);
    let (d, w, big_w, h, f) = (shade('d'), shade('w'), shade('W'), shade('h'), shade('f'));
    art.map(|v| {
        if v == d {
            w
        } else if v == w {
            big_w
        } else if v == big_w {
            h
        } else if v == h {
            f
        } else {
            v
        }
    })
}

/// A strip of bark lying on the ground, curling at one end, 8x8.
pub fn bark() -> PixelArt {
    let rows = ["........", "......o.", ".oooooho", "owwdwwho", "odwwdwo.", ".ooooo..", "........", "........"];
    PixelArt::parse(&rows, ITEM_LEGEND)
}

/// A bark pot on the ground, 8x8 (effects palette: bark, vine band).
pub fn bark_pot() -> PixelArt {
    let rows = ["........", ".oooooo.", "oWoooowo", "oWWwwwwo", "oGgGgGgo", "oWwwwwwo", ".oooooo.", "........"];
    PixelArt::parse(&rows, FX_LEGEND)
}

/// A coconut held up to eat, after `bites` (1..=3) bites from the top: the
/// husk is gnawed away to the white flesh, down to a last crescent of shell.
pub fn bitten_coconut(bites: usize) -> PixelArt {
    let whole = &coconut_frames()[0];
    let legend = |c: char| ITEM_LEGEND.iter().find(|(k, _)| *k == c).map(|(_, v)| *v).unwrap_or(0);
    let (outline, flesh, flesh_shade) = (legend('o'), legend('f'), legend('F'));
    // Each bite takes a round mouthful (centre, radius in px).
    const MOUTHFULS: [(f32, f32, f32); 3] = [(6.6, 1.2, 2.4), (3.2, 0.8, 2.6), (5.0, 3.4, 2.7)];
    let bitten = |x: i32, y: i32| MOUTHFULS[..bites.min(3)].iter().any(|&(cx, cy, r)| (x as f32 + 0.5 - cx).powi(2) + (y as f32 + 0.5 - cy).powi(2) < r * r);
    let solid = |x: i32, y: i32| (0..8).contains(&x) && (0..8).contains(&y) && whole.at(x, y) != 0 && !bitten(x, y);
    let near_bite = |x: i32, y: i32, d: i32| (-d..=d).any(|dy| (-d..=d).any(|dx| bitten(x + dx, y + dy) && whole.at(x + dx, y + dy) != 0));
    let mut art = PixelArt::filled(8, 8, 0);
    for y in 0..8i32 {
        for x in 0..8i32 {
            let v = if solid(x, y) {
                // The cut face shows the flesh, brightest along the bite.
                if whole.at(x, y) == outline && !near_bite(x, y, 1) {
                    outline
                } else if near_bite(x, y, 1) {
                    flesh
                } else if near_bite(x, y, 2) {
                    flesh_shade
                } else {
                    whole.at(x, y)
                }
            } else if whole.at(x, y) != 0 && [(0, 1), (1, 0), (0, -1), (-1, 0)].iter().any(|&(dx, dy)| solid(x + dx, y + dy)) {
                // Outline the bitten edge so it reads at this size.
                outline
            } else {
                0
            };
            art.set(x as usize, y as usize, v);
        }
    }
    art
}

/// Water held in cupped hands (water-fx palette, drawn translucent by day):
/// level, then tipped toward the mouth.
pub fn handful_frames() -> [PixelArt; 2] {
    let legend: &[(char, u8)] = &[('a', 1), ('b', 2), ('c', 3)];
    let level = ["........", "........", "..cccc..", ".cabbac.", ".caaaac.", "..cccc..", "........", "........"];
    let tipped = ["........", "...cc...", "..cbac..", ".cabaac.", "..caac..", "...cc...", "........", "........"];
    [PixelArt::parse(&level, legend), PixelArt::parse(&tipped, legend)]
}

/// A dead branch lying on the ground, 16x8: grey-brown, forked, with a
/// broken stub.
pub fn branch() -> PixelArt {
    let rows = [
        "................",
        "...........oo...",
        "..........oWdo..",
        ".oooooooooWdoo..",
        "oWWhWWWWWWwwwooo",
        "odwdddwddddwdWho",
        ".oooooooooooooo.",
        "................",
    ];
    PixelArt::parse(&rows, ITEM_LEGEND)
}

/// A fist-sized stone, 8x8.
pub fn stone() -> PixelArt {
    let rows = ["........", "........", "..ooooo.", ".ofFFsso", "oFfFFsso", "oFFssss.", ".oooooo.", "........"];
    PixelArt::parse(&rows, ITEM_LEGEND)
}

/// A coil of vine on the ground, 8x8 (effects palette greens).
pub fn vine_coil() -> PixelArt {
    let rows = ["........", "..oooo..", ".oGhGGo.", "oGoooGgo", "oGo.oGgo", "ogGGGgo.", ".ooooo..", "........"];
    PixelArt::parse(&rows, FX_LEGEND)
}

/// A coconut husk lying on the ground, 8x8 (item colours).
pub fn husk() -> PixelArt {
    let rows = ["........", "........", ".oooooo.", "ohhdddho", "oWhhhhWo", "owWdwWwo", ".odwwdo.", "..oooo.."];
    PixelArt::parse(&rows, ITEM_LEGEND)
}

/// A coil of rope lying on the ground, 8x8 (item colours).
pub fn rope_coil() -> PixelArt {
    let rows = ["........", "..oooo..", ".ohwhWo.", "ohWoowho", "oWho.oWo", "owhWhwo.", ".ooooowo", "......oo"];
    PixelArt::parse(&rows, ITEM_LEGEND)
}

/// A palm frond lying on the ground, 16x8 (effects palette).
pub fn palm_frond() -> PixelArt {
    let rows = [
        "................",
        "..oooooooooo....",
        ".ohGhGhGhGhGoo..",
        "ohGhGhGhGhGhGWo.",
        "oWWWWWWWWWWWWWwo",
        ".ogGgGgGgGgGgoo.",
        "..oooooooooooo..",
        "................",
    ];
    PixelArt::parse(&rows, FX_LEGEND)
}

/// A thatch panel lying on the ground, 16x8 (effects palette).
pub fn thatch() -> PixelArt {
    let rows = [
        "................",
        ".oooooooooooooo.",
        "ohGhgGhGhGgGhGho",
        "oGgGgGgGgGgGgGgo",
        "ohGhgGhGhGgGhGho",
        "ogggggggggggggo.",
        ".oooooooooooooo.",
        "................",
    ];
    PixelArt::parse(&rows, FX_LEGEND)
}

/// The rolled-up shelter lying on the ground, 16x8 (effects palette).
pub fn shelter_roll() -> PixelArt {
    let rows = [
        "................",
        ".oooooooooooooo.",
        "ohhhchhhhhchhhho",
        "oGGGcGGGGGcGGGGo",
        "WgggcggggWcggggW",
        "ogggWggggggWgggo",
        ".oooooooooooooo.",
        "................",
    ];
    PixelArt::parse(&rows, FX_LEGEND)
}

/// The shelter pitched for the night, 40x24 (effects palette), seen from
/// the side: a thatched roof down to the ground, lit toward the ridge, its
/// poles crossing over the ridge at each end, a dark doorway at the right.
pub fn lean_to() -> PixelArt {
    let i = |c: char| FX_LEGEND.iter().find(|(k, _)| *k == c).map(|(_, v)| *v).unwrap_or(0);
    let mut a = PixelArt::filled(40, 24, 0);
    // The roof: wider at the eaves, in courses of thatch with ragged tips.
    for y in 5..22 {
        let spread = (y - 5) / 4;
        for x in 5 - spread..35 + spread {
            let (course, k) = ((y - 5) / 3, (y - 5) % 3);
            let tip = k == 2 && (x + course) % 2 == 0;
            let shade = [['h', 'h', 'G'], ['h', 'G', 'g'], ['G', 'G', 'g'], ['G', 'g', 'g'], ['G', 'g', 'g'], ['g', 'g', 'g']][course.min(5)];
            a.set(x, y, i(if tip { shade[0] } else { shade[k] }));
        }
    }
    // The doorway in the gable end.
    for y in 12..22 {
        let half = (y - 12) / 2;
        for x in 35 - half..=35 {
            a.set(x + 1, y, i('o'));
        }
    }
    // Ridge pole, its crossed poles poking up at both ends, roped.
    for x in 3..37 {
        a.set(x, 4, i('W'));
        a.set(x, 5, i('w'));
    }
    for (x0, dir) in [(2, 1), (37, -1)] {
        for k in 0..6 {
            a.set((x0 + dir * k) as usize, (k) as usize, i('W'));
            a.set((x0 + dir * (5 - k)) as usize, (k) as usize, i('w'));
        }
        a.set((x0 + dir * 2) as usize, 4, i('c'));
        a.set((x0 + dir * 3) as usize, 4, i('c'));
    }
    a.outlined(i('o'))
}

/// A stone knife lying on the ground, 8x8 (effects palette).
pub fn stone_knife() -> PixelArt {
    let rows = ["........", "......o.", ".....oLo", "....oMLo", "...oMLo.", "..omMo..", ".ommo...", "..oo...."];
    PixelArt::parse(&rows, FX_LEGEND)
}

/// A coconut cut open (item colours), 8x8: on the ground, and in hand
/// before the first bite.
pub fn opened_coconut() -> PixelArt {
    let rows = ["........", "..oooo..", ".oddddo.", "odfFFfdo", "oWdFFdWo", "owwddwwo", ".owwwwo.", "..oooo.."];
    PixelArt::parse(&rows, ITEM_LEGEND)
}

/// A thought bubble, 16x16 (effects palette), with one, two then three
/// dots in it; trailing puffs lead down-left toward the thinker's head.
/// A speech bubble with a "!" in it, 16x16: a recipe has come to mind.
pub fn eureka_bubble() -> PixelArt {
    PixelArt::parse(
        &[
            "................",
            "...oooooooooo...",
            "..oLLLLmmLLLLo..",
            ".oLLLLLmmLLLLLo.",
            ".oLLLLLmmLLLLLo.",
            ".oLLLLLLLLLLLLo.",
            ".oLLLLLmmLLLLLo.",
            "..oLLLLLLLLLLo..",
            "...ooLLoooooo...",
            ".....oLo........",
            "......o.........",
            "................",
            "................",
            "................",
            "................",
            "................",
        ],
        FX_LEGEND,
    )
}

pub fn thought_frames() -> [PixelArt; 3] {
    let base = [
        "................",
        "...oooooooooo...",
        "..oLLLLLLLLLLo..",
        ".oLLLLLLLLLLLLo.",
        ".oLLLLLLLLLLLLo.",
        ".oLLLLLLLLLLLLo.",
        "..oLLLLLLLLLLo..",
        "...ooooooooooo..",
        "....oLLo........",
        "....oooo........",
        "..oLo...........",
        "..ooo...........",
        "................",
        "................",
        "................",
        "................",
    ];
    std::array::from_fn(|n| {
        let mut a = PixelArt::parse(&base, FX_LEGEND);
        let dot = FX_LEGEND.iter().find(|(k, _)| *k == 'm').map(|(_, v)| *v).unwrap_or(0);
        for k in 0..=n {
            let x = 4 + k * 3;
            for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                a.set(x + dx, 4 + dy, dot);
            }
        }
        a
    })
}

/// A fishing rod held out level, 32x8: the butt at the left, thinning to a
/// bent tip at the right (the line is drawn separately).
pub fn rod_side() -> PixelArt {
    PixelArt::parse(
        &[
            "................................",
            "................................",
            "............................ooo.",
            "...............ooooooooooooooWWo",
            "ooooooooooooooooWWWWWWWWWWWWWWoo",
            "oWWWWWWWWWWWWWWWwwwwwwwwwwwwwo..",
            "owwwwwwwwwwwwwwoooooooooooooo...",
            "ooooooooooooooo.................",
        ],
        FX_LEGEND,
    )
}

/// Where the line leaves the tip in `rod_side`.
pub const ROD_TIP: (i32, i32) = (30, 3);

/// The float on the line, 8x8: riding the water, bobbing down as a fish
/// nibbles, and pulled under (just the ring it leaves).
pub fn float_frames() -> [PixelArt; 3] {
    let float = ["........", "...oo...", "..oRqo..", "..owwo..", "...oo...", "........", "........", "........"];
    let bob = ["........", "........", "...oo...", "..oRqo..", "..owwo..", "........", "........", "........"];
    let under = ["........", "........", "........", "........", "...RR...", "........", "........", "........"];
    [PixelArt::parse(&float, CRAB_LEGEND), PixelArt::parse(&bob, CRAB_LEGEND), PixelArt::parse(&under, CRAB_LEGEND)]
}

/// One dot of fishing line, 8x8 (a dark grey pixel in the top-left, to
/// show on sand and water alike).
pub fn line_dot() -> PixelArt {
    let mut a = PixelArt::filled(8, 8, 0);
    a.set(0, 0, FX_LEGEND.iter().find(|(k, _)| *k == 'm').map(|(_, v)| *v).unwrap_or(0));
    a
}

/// A fish's dark shape under the water, 16x8, swimming left, its tail
/// swishing between two frames (drawn in the blended shadow palette).
pub fn fish_shadow_frames() -> [PixelArt; 2] {
    let rows = [
        [
            "................",
            "................",
            "...########..#..",
            "..###########...",
            "..###########...",
            "...########..#..",
            "................",
            "................",
        ],
        [
            "................",
            "............#...",
            "...########.#...",
            "..###########...",
            "..###########...",
            "...########.##..",
            "................",
            "................",
        ],
    ];
    rows.map(|r| PixelArt::parse(&r, &[('#', 1)]))
}

/// A drop of water, 8x8 (thrown up by a splash).
pub fn droplet() -> PixelArt {
    PixelArt::parse(&["........", "...i....", "..ili...", "..lbl...", "...l....", "........", "........", "........"], FX_LEGEND)
}

/// A digging stick lying on the ground, 16x8: a straight branch, its point
/// (right) pale.
pub fn digging_stick() -> PixelArt {
    let rows = [
        "................",
        "................",
        "................",
        "oooooooooooooo..",
        "oWWWwWWWWWWWhhoo",
        "odddwddddddwFFho",
        "ooooooooooooooo.",
        "................",
    ];
    PixelArt::parse(&rows, ITEM_LEGEND)
}

/// A bamboo spade lying on the ground, 32x8: the green handle, the rope
/// binding, and the split blade (right) showing its pale inside.
pub fn bamboo_spade() -> PixelArt {
    PixelArt::parse(
        &[
            "................................",
            "....................ooooooooooo.",
            "ooooooooooooooooooowWhhhhhhhhhho",
            "ohhhhhhghhhhhhhhhhhoWwccccccccco",
            "oGGGGGGgGGGGGGGGGGGoWwWWWWWWWWgo",
            "ogggggggggggggggggggwWgggggggggo",
            "oooooooooooooooooooooooooooooo..",
            "................................",
        ],
        FX_LEGEND,
    )
}

/// Marking stakes lying on the ground, 32x8: the bundle of split bamboo
/// stakes on its side, points to the right, the hank of rope round it.
pub fn marking_stakes() -> PixelArt {
    PixelArt::parse(
        &[
            "................................",
            ".ooooooooooooooooooooooooooooo..",
            "ocGGGGGGgGGGGWwWwWwGGGGGGGGGhhoo",
            "ochhhhhhhhhhhwWwWwWhhhhhhhhhhhho",
            "occccccgcccccWwWwWwccccccccccoo.",
            ".oooooooooooooooooooooooooooo...",
            "................................",
            "................................",
        ],
        FX_LEGEND,
    )
}

/// A lump of clay on the ground, 8x8 (effects palette greys).
pub fn clay_lump() -> PixelArt {
    let rows = ["........", "...oo...", "..oLMo..", ".ooMmoo.", "oLMooMMo", "oMmmoMmo", ".oooooo.", "........"];
    PixelArt::parse(&rows, FX_LEGEND)
}

/// A little heap of soil on the ground, 8x8.
pub fn soil_heap() -> PixelArt {
    let rows = ["........", "...oo...", "..oWwo..", ".oWwdwo.", "oWwdwddo", "odwdddwo", ".oooooo.", "........"];
    PixelArt::parse(&rows, ITEM_LEGEND)
}

/// A length of bamboo lying on the ground, 32x8: a green culm with joints,
/// its cut end (right) pale.
pub fn bamboo_lying() -> PixelArt {
    PixelArt::parse(
        &[
            "................................",
            "................................",
            "ooooooooooooooooooooooooooooooo.",
            "ohhhhhhghhhhhhhhhghhhhhhhhghhhco",
            "oGGGGGGgGGGGGGGGGgGGGGGGGGgGGGco",
            "oggggggggggggggggggggggggggggggo",
            "ooooooooooooooooooooooooooooooo.",
            "................................",
        ],
        FX_LEGEND,
    )
}

/// Puffs of smoke rising from a signal fire (the water effects palette's
/// greys, drawn half see-through by day): new and small, then growing
/// (16x16), then billowed out and thinning away (32x32).
pub fn smoke_frames() -> [PixelArt; 4] {
    use super::palettes::smoke;
    let puff = |size: usize, lumps: &[(f32, f32, f32)], thin: bool| {
        let mut a = PixelArt::filled(size, size, 0);
        for &(cx, cy, r) in lumps {
            fill_ellipse(&mut a, cx, cy, r, r * 0.85, |x, y, nx, ny| {
                if thin && bayer(x, y) < 0.45 {
                    return None;
                }
                let light = sphere_light(nx, ny);
                Some(if light > 0.62 {
                    smoke::LIGHT
                } else if light > 0.32 || bayer(x, y) < light {
                    smoke::MID
                } else {
                    smoke::SHADE
                })
            });
        }
        a
    };
    [
        puff(16, &[(8.0, 9.0, 3.2), (6.5, 7.5, 2.2), (9.8, 7.0, 2.0)], false),
        puff(16, &[(8.0, 9.5, 5.4), (5.0, 7.0, 3.8), (10.8, 6.0, 3.9), (8.0, 4.3, 3.2)], false),
        puff(32, &[(16.0, 19.0, 9.5), (9.5, 15.0, 6.8), (22.5, 14.0, 7.2), (15.5, 9.0, 6.6), (24.0, 21.0, 5.5), (8.0, 21.5, 5.0)], false),
        puff(32, &[(16.0, 18.0, 11.0), (8.5, 13.0, 7.0), (23.5, 12.5, 7.5)], true),
    ]
}

/// A bamboo flask lying on the ground, 16x8: its open mouth (left) shows
/// what's in it, a vine tied round its neck.
pub fn flask_lying(holds: super::item_art::FlaskHolds) -> PixelArt {
    use super::item_art::FlaskHolds;
    let mouth: [&str; 3] = match holds {
        FlaskHolds::Nothing => [".ocoGgGGGgGGGgo.", ".ocoGgGGGgGGGgo.", ".ocwGgGGGgGGGgo."],
        FlaskHolds::RawWater => [".oclGgGGGgGGGgo.", ".ocbGgGGGgGGGgo.", ".ocbGgGGGgGGGgo."],
        FlaskHolds::BoiledWater => [".ociGgGGGgGGGgo.", ".oclGgGGGgGGGgo.", ".oclGgGGGgGGGgo."],
    };
    let rows = ["................", "...ooooooooooo..", "..ochGhhhhhhhho.", mouth[0], mouth[1], mouth[2], "..ocggggggggggo.", "...ooooooooooo.."];
    PixelArt::parse(&rows, FX_LEGEND)
}

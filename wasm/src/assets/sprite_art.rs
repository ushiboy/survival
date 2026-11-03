//! Sprite artwork. Hand-drawn sprites use text with a per-palette legend
//! (see `palettes::*_LEGEND`); `.` is transparent.

use super::paint::{bayer, fill_ellipse};
use super::palettes::{ITEM_LEGEND, CRAB_LEGEND, FX_LEGEND, GULL_LEGEND};
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
    [
        make([".o.oorrrrrroo.o.", "o.o.o.oooo.o.o.o", "................"]),
        make(["o..oorrrrrroo..o", ".o.o..oooo..o.o.", "................"]),
    ]
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

/// Axe lying horizontally, head on the right with the edge facing right.
pub fn axe_side() -> PixelArt {
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

/// Axe held upright, head at the top with the edge facing up.
pub fn axe_up() -> PixelArt {
    axe_side().transposed().flipped_v()
}

/// Axe at 45°, head at the top right. Rotated from the side view without
/// its outline, then re-outlined so the edges stay clean.
pub fn axe_diag() -> PixelArt {
    let src = axe_side().map(|v| if v == 1 { 0 } else { v });
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

/// A coconut held up to eat, after `bites` (1..=3) bites from the top: the
/// husk is gnawed away to the white flesh, down to a last crescent of shell.
pub fn bitten_coconut(bites: usize) -> PixelArt {
    let whole = &coconut_frames()[0];
    let legend = |c: char| ITEM_LEGEND.iter().find(|(k, _)| *k == c).map(|(_, v)| *v).unwrap_or(0);
    let (outline, flesh, flesh_shade) = (legend('o'), legend('f'), legend('F'));
    // Each bite takes a round mouthful (centre, radius in px).
    const MOUTHFULS: [(f32, f32, f32); 3] = [(6.6, 1.2, 2.4), (3.2, 0.8, 2.6), (5.0, 3.4, 2.7)];
    let bitten = |x: i32, y: i32| {
        MOUTHFULS[..bites.min(3)].iter().any(|&(cx, cy, r)| (x as f32 + 0.5 - cx).powi(2) + (y as f32 + 0.5 - cy).powi(2) < r * r)
    };
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

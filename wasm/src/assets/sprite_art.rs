//! Sprite artwork. Hand-drawn sprites use text with a per-palette legend
//! (see `palettes::*_LEGEND`); `.` is transparent.

use super::paint::{bayer, fill_ellipse};
use super::palettes::{CRAB_LEGEND, FX_LEGEND, GULL_LEGEND, PLAYER_LEGEND};
use crate::ppu::pattern::PixelArt;

fn player(rows: &[&str]) -> PixelArt {
    PixelArt::parse(rows, PLAYER_LEGEND)
}

// Castaway, 16x24. o outline · h/H/i hair · s/S/k skin · r/R/q bandana ·
// t/T/W shirt · b/B shorts.

const HEAD_FRONT: [&str; 12] = [
    "................",
    ".....oooooo.....",
    "....oHHiiHHo....",
    "...oHiiHHHHHo...",
    "..oHHHHHHHHHho..",
    "..oRqqRRRRRRro..",
    "..orRRRRRRRrro..",
    "..ohSkkSSSSSho..",
    "..ohSoSSSSoShho.",
    "..ohSoSSSSoSho..",
    "...osSkSSSSso...",
    "....ossSSsso....",
];

const HEAD_BACK: [&str; 12] = [
    "................",
    ".....oooooo.....",
    "....oHHiiHHo....",
    "...oHiiHHHHHo...",
    "..oHHHHHHHHHho..",
    "..oRqqRRRRRRro..",
    "..orRRRRRRRrro..",
    "..ohHHHqRHHHho..",
    "..ohHHRrrRHHho..",
    "..ohhHHRrHHhho..",
    "...ohhHHHHhho...",
    "....osSSSSso....",
];

pub fn player_down_stand() -> PixelArt {
    let mut rows = HEAD_FRONT.to_vec();
    rows.extend_from_slice(&[
        "...otWWWWWWto...",
        "..oStTWWWWTtSo..",
        "..oStTTTTTTtSo..",
        "..oSotTTTTtoSo..",
        "..okottttttoko..",
        "...oobBBBBboo...",
        "....obBBBBbo....",
        "....obBoobBo....",
        "...oSso..oSso...",
        "...oSso..oSso...",
        "...ohho..ohho...",
        "...oooo..oooo...",
    ]);
    player(&rows)
}

pub fn player_down_step() -> PixelArt {
    let mut rows = HEAD_FRONT.to_vec();
    rows.extend_from_slice(&[
        "...otWWWWWWto...",
        "..oStTWWWWTtSo..",
        "..oStTTTTTTtoo..",
        "..oSotTTTTtto...",
        "..okottttttoo...",
        "...oobBBBBboo...",
        "....obBBBBbo....",
        "....obBoobBo....",
        "...oSso..oSso...",
        "...ohho..oSso...",
        "...oooo..oSso...",
        ".........oooo...",
    ]);
    player(&rows)
}

pub fn player_up_stand() -> PixelArt {
    let mut rows = HEAD_BACK.to_vec();
    rows.extend_from_slice(&[
        "...ottTTTTtto...",
        "..oSttTTTTttSo..",
        "..oSttTTTTttSo..",
        "..oSotTTTTtoSo..",
        "..okottttttoko..",
        "...oobBBBBboo...",
        "....obBBBBbo....",
        "....obBoobBo....",
        "...oSso..oSso...",
        "...oSso..oSso...",
        "...ohho..ohho...",
        "...oooo..oooo...",
    ]);
    player(&rows)
}

pub fn player_up_step() -> PixelArt {
    let mut rows = HEAD_BACK.to_vec();
    rows.extend_from_slice(&[
        "...ottTTTTtto...",
        "..oSttTTTTttSo..",
        "..oSttTTTTttoo..",
        "..oSotTTTTtto...",
        "..okottttttoo...",
        "...oobBBBBboo...",
        "....obBBBBbo....",
        "....obBoobBo....",
        "...oSso..oSso...",
        "...ohho..oSso...",
        "...oooo..oSso...",
        ".........oooo...",
    ]);
    player(&rows)
}

const HEAD_SIDE: [&str; 12] = [
    "................",
    ".....oooooo.....",
    "....oHHiiHHo....",
    "...oHiiHHHHHo...",
    "..oHHHHHHHHHo...",
    "..oRRRRRqqRRo...",
    ".oqrRRRRRRRro...",
    ".orohHHHSSSkSo..",
    "..oohHHSSSSoSo..",
    "...ohHSSSSSSSo..",
    "...ohsSSSSSSo...",
    "....ossSSSoo....",
];

/// Facing right; facing left is the horizontal flip.
pub fn player_side_stand() -> PixelArt {
    let mut rows = HEAD_SIDE.to_vec();
    rows.extend_from_slice(&[
        "....otTWWWto....",
        "....otTWSkto....",
        "....otTTSSto....",
        "....otttSSto....",
        "....otttkkoo....",
        "....obBBBBbo....",
        "....obBBBBbo....",
        ".....obBBbo.....",
        ".....oSsSo......",
        ".....oSsSo......",
        ".....ohhhhoo....",
        ".....ooooooo....",
    ]);
    player(&rows)
}

pub fn player_side_step_a() -> PixelArt {
    let mut rows = HEAD_SIDE.to_vec();
    rows.extend_from_slice(&[
        "....otTWWWto....",
        "....otTWWSSo....",
        "....otTTTtSko...",
        "....otttttoko...",
        "....otttttoo....",
        "....obBBBBbo....",
        "....obBoobBo....",
        "...oSso..oSso...",
        "..oSso....oSso..",
        "..oSso....ohhho.",
        ".ohhho.....oooo.",
        ".oooo...........",
    ]);
    player(&rows)
}

pub fn player_side_step_b() -> PixelArt {
    let mut rows = HEAD_SIDE.to_vec();
    rows.extend_from_slice(&[
        "...ootTWWWto....",
        "..okSotTWWto....",
        "..oSotTTTTto....",
        "...ootttttto....",
        "....otttttoo....",
        "....obBBBBbo....",
        "....obBoobBo....",
        "...oSso..oSso...",
        "..oSso....oSso..",
        "..oSso....ohhho.",
        ".ohhho.....oooo.",
        ".oooo...........",
    ]);
    player(&rows)
}

// ---------------------------------------------------------------- axe swing poses
// Hips and feet stay planted in a wide stance while the shoulders twist:
// the torso is redrawn from the angle it turns to and offset over the hips,
// and the head is shifted with it. `K`/`O` mark the hands (skin/outline) so
// they can also be drawn as an overlay on top of the axe handle.

/// A pose: the full body, and just its hands.
pub struct ActionPose {
    pub body: PixelArt,
    pub hands: PixelArt,
}

/// Rows of the 16x24 castaway that make up the head.
const HEAD_ROWS: std::ops::Range<usize> = 0..12;

fn action_pose(head: &[&str], head_dx: i32, body: [&str; 12]) -> ActionPose {
    let mut rows: Vec<&str> = head.to_vec();
    rows.extend_from_slice(&body);
    let hands: Vec<String> = rows
        .iter()
        .map(|r| r.chars().map(|c| if c == 'K' || c == 'O' { c } else { '.' }).collect())
        .collect();
    let hand_rows: Vec<&str> = hands.iter().map(String::as_str).collect();
    ActionPose { body: player(&rows).shift_rows(HEAD_ROWS, head_dx), hands: player(&hand_rows) }
}

/// Wide stance shared by the side-view poses.
const SIDE_LEGS: [&str; 7] = [
    "....obBBBBbo....",
    "...obBBBBBBbo...",
    "..obBo...obBo...",
    ".oSso.....oSso..",
    ".oSso......oSso.",
    "ohhho......ohhho",
    "oooo........oooo",
];

/// Wide stance shared by the front/back-view poses.
const FRONT_LEGS: [&str; 7] = [
    "...oobBBBBboo...",
    "...obBBBBBBbo...",
    "..obBo....obBo..",
    ".oSso......oSso.",
    ".oSso......oSso.",
    ".ohho......ohho.",
    ".oooo......oooo.",
];

fn with_legs(torso: [&'static str; 5], legs: [&'static str; 7]) -> [&'static str; 12] {
    let mut out = [""; 12];
    out[..5].copy_from_slice(&torso);
    out[5..].copy_from_slice(&legs);
    out
}

/// Facing right, wound up: shoulders turned back so the chest shows,
/// leaning over the back leg, axe pulled far behind.
pub fn chop_side_back() -> ActionPose {
    action_pose(&HEAD_SIDE, -1, with_legs([
        "...otWWWWWto....",
        "..ootTWWWTto....",
        "OKSotTTTTTo.....",
        "OKKottTTtto.....",
        "..ooottttoo.....",
    ], SIDE_LEGS))
}

/// Facing right, follow-through: shoulders swung past the hips so the back
/// shows, head leading, arms thrust forward.
pub fn chop_side_strike() -> ActionPose {
    action_pose(&HEAD_SIDE, 2, with_legs([
        ".....ottTTTto...",
        ".....otttTTtoKKO",
        "....otttttoKKKO.",
        "....otttttoo....",
        "....ottttto.....",
    ], [
        "....obBBBBbo....",
        "...obBBBBBBbo...",
        "..obBo...obBo...",
        ".oSso.....oSso..",
        "oSso.......oSso.",
        "ohho.......ohhho",
        "ooo........oooo.",
    ]))
}

/// Facing the camera, wound up toward the screen-left side.
pub fn chop_down_back() -> ActionPose {
    action_pose(&HEAD_FRONT, -1, with_legs([
        "..otWWWWWto.....",
        "..otTWWWTtoo....",
        ".ootTTTTTtSo....",
        "OKKSottTTttSo...",
        "OKKoottttooo....",
    ], FRONT_LEGS))
}

/// Facing the camera, shoulders swung through to the right, arms in front.
pub fn chop_down_strike() -> ActionPose {
    action_pose(&HEAD_FRONT, 1, with_legs([
        "....otWWWWWWto..",
        "...otTWWWWWWTto.",
        "...oStTTTTTTtSo.",
        "....oSttTTttSo..",
        "......otoOKKKO..",
    ], FRONT_LEGS))
}

/// Facing away, wound up toward the screen-right side.
pub fn chop_up_back() -> ActionPose {
    action_pose(&HEAD_BACK, 1, with_legs([
        "....ottTTTTtto..",
        "...oSttTTTTtto..",
        "....ottTTTTtoKKO",
        ".....ottttooKKKO",
        ".....otttto.....",
    ], FRONT_LEGS))
}

/// Facing away, shoulders swung through to the left, arms forward.
pub fn chop_up_strike() -> ActionPose {
    action_pose(&HEAD_BACK, -1, with_legs([
        "..ottTTTTtto....",
        ".ooSttTTttSoo...",
        "..ooSttttSoo....",
        "...ottTTtto.....",
        "....otttttto....",
    ], FRONT_LEGS))
}

/// Row where the waterline cuts the castaway while wading.
pub const PLAYER_WADE_ROW: usize = 18;

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

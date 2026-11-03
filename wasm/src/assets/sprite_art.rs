//! Sprite artwork. Hand-drawn sprites use text with a per-palette legend
//! (see `palettes::*_LEGEND`); `.` is transparent.

use super::paint::{bayer, fill_ellipse};
use super::palettes::{CRAB_LEGEND, GULL_LEGEND, PLAYER_LEGEND};
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

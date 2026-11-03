//! 2bpp HUD graphics (BG3): outlined font, hearts, gauges and icons.
//! Index 1 = outline, 2 = shade, 3 = highlight.

use crate::ppu::pattern::PixelArt;

const GLYPHS: &[(char, [&str; 6])] = &[
    ('0', [".###.", "#..##", "#.#.#", "##..#", "#...#", ".###."]),
    ('1', ["..#..", ".##..", "..#..", "..#..", "..#..", ".###."]),
    ('2', [".###.", "#...#", "...#.", "..#..", ".#...", "#####"]),
    ('3', ["####.", "....#", ".###.", "....#", "....#", "####."]),
    ('4', ["...#.", "..##.", ".#.#.", "#..#.", "#####", "...#."]),
    ('5', ["#####", "#....", "####.", "....#", "#...#", ".###."]),
    ('6', [".###.", "#....", "####.", "#...#", "#...#", ".###."]),
    ('7', ["#####", "....#", "...#.", "..#..", ".#...", ".#..."]),
    ('8', [".###.", "#...#", ".###.", "#...#", "#...#", ".###."]),
    ('9', [".###.", "#...#", "#...#", ".####", "....#", ".###."]),
    ('A', [".###.", "#...#", "#...#", "#####", "#...#", "#...#"]),
    ('D', ["####.", "#...#", "#...#", "#...#", "#...#", "####."]),
    ('M', ["#...#", "##.##", "#.#.#", "#...#", "#...#", "#...#"]),
    ('P', ["####.", "#...#", "#...#", "####.", "#....", "#...."]),
    ('Y', ["#...#", "#...#", ".#.#.", "..#..", "..#..", "..#.."]),
    (':', [".....", "..#..", ".....", ".....", "..#..", "....."]),
];

pub fn glyphs() -> Vec<(char, PixelArt)> {
    GLYPHS
        .iter()
        .map(|(c, rows)| {
            let mut a = PixelArt::filled(8, 8, 0);
            for (y, row) in rows.iter().enumerate() {
                for (x, ch) in row.chars().enumerate() {
                    if ch == '#' {
                        // Two-tone: bright top half, shaded bottom half.
                        a.set(x + 1, y + 1, if y < 3 { 3 } else { 2 });
                    }
                }
            }
            (*c, a.outlined(1))
        })
        .collect()
}

fn art(rows: [&str; 8]) -> PixelArt {
    PixelArt::parse(&rows, &[('1', 1), ('2', 2), ('3', 3)])
}

pub fn heart_full() -> PixelArt {
    art([
        ".11.11..",
        "1321221.",
        "1322221.",
        "1222221.",
        ".12221..",
        "..121...",
        "...1....",
        "........",
    ])
}

pub fn heart_empty() -> PixelArt {
    art([
        ".11.11..",
        "1..1..1.",
        "1.....1.",
        "1.....1.",
        ".1...1..",
        "..1.1...",
        "...1....",
        "........",
    ])
}

pub fn icon_food() -> PixelArt {
    art([
        "...111..",
        "..13321.",
        ".133221.",
        ".132221.",
        "..1221..",
        ".131.1..",
        "131.....",
        ".1......",
    ])
}

pub fn icon_water() -> PixelArt {
    art([
        "...1....",
        "..121...",
        ".12221..",
        ".13221..",
        "1322221.",
        "1322221.",
        ".12221..",
        "..111...",
    ])
}

pub fn icon_sun() -> PixelArt {
    art([
        "...2....",
        ".2.1.2..",
        "..133...",
        "213321..",
        "..1221.2",
        ".2.11...",
        "....2...",
        "........",
    ])
}

pub fn icon_moon() -> PixelArt {
    art([
        "..111...",
        ".1331...",
        "1331....",
        "1321....",
        "1322111.",
        ".122221.",
        "..1111..",
        "........",
    ])
}

pub fn icon_lantern() -> PixelArt {
    art([
        "..111...",
        ".1...1..",
        ".11111..",
        "1233321.",
        "1232321.",
        "1233321.",
        ".11111..",
        "........",
    ])
}

pub fn icon_wood() -> PixelArt {
    art([
        "........",
        ".11111..",
        "1333311.",
        "12222131",
        "12222131",
        "1222211.",
        ".11111..",
        "........",
    ])
}

pub fn icon_coconut() -> PixelArt {
    art([
        "........",
        "..111...",
        ".13321..",
        "1332221.",
        "1322221.",
        "1222221.",
        ".12221..",
        "..111...",
    ])
}

/// Gauge middle segment with `fill` (0..=8) pixels filled.
pub fn gauge_segment(fill: usize) -> PixelArt {
    let mut a = PixelArt::filled(8, 8, 0);
    for x in 0..8 {
        a.set(x, 1, 1);
        a.set(x, 6, 1);
        for y in 2..6 {
            let v = if x < fill { if y == 2 { 3 } else { 2 } } else { 1 };
            a.set(x, y, v);
        }
    }
    a
}

/// End cap closing the gauge (on the left edge of the tile for the right end).
pub fn gauge_cap(right: bool) -> PixelArt {
    let mut a = PixelArt::filled(8, 8, 0);
    let x = if right { 0 } else { 7 };
    for y in 1..7 {
        a.set(x, y, 1);
    }
    a
}

/// Size of the lantern light mask in tiles (128x128 px).
pub const LIGHT_MASK_TILES: usize = 16;

/// Radial light mask for the sub-screen: index 1 (centre) → 3 (rim) are
/// increasing amounts of darkness, 0 (beyond the rim) falls back to the
/// fixed colour, i.e. full darkness. Band edges are Bayer-dithered so the
/// light fades smoothly with only four levels.
pub fn light_mask() -> Vec<PixelArt> {
    use super::paint::bayer;
    let size = LIGHT_MASK_TILES * 8;
    let c = size as f32 / 2.0;
    let mut tiles = Vec::with_capacity(LIGHT_MASK_TILES * LIGHT_MASK_TILES);
    for ty in 0..LIGHT_MASK_TILES {
        for tx in 0..LIGHT_MASK_TILES {
            let mut a = PixelArt::filled(8, 8, 0);
            for y in 0..8 {
                for x in 0..8 {
                    let (px, py) = ((tx * 8 + x) as f32 + 0.5, (ty * 8 + y) as f32 + 0.5);
                    let r = ((px - c).powi(2) + (py - c).powi(2)).sqrt();
                    let level = ((r - 20.0) / 12.0 + bayer(px as i32, py as i32) - 0.5).floor().max(0.0) as u8 + 1;
                    a.set(x, y, if level > 3 { 0 } else { level });
                }
            }
            tiles.push(a);
        }
    }
    tiles
}

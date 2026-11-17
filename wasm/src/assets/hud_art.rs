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
    ('B', ["####.", "#...#", "####.", "#...#", "#...#", "####."]),
    ('C', [".###.", "#...#", "#....", "#....", "#...#", ".###."]),
    ('E', ["#####", "#....", "####.", "#....", "#....", "#####"]),
    ('F', ["#####", "#....", "####.", "#....", "#....", "#...."]),
    ('G', [".###.", "#....", "#.###", "#...#", "#...#", ".###."]),
    ('H', ["#...#", "#...#", "#####", "#...#", "#...#", "#...#"]),
    ('I', [".###.", "..#..", "..#..", "..#..", "..#..", ".###."]),
    ('J', ["..###", "...#.", "...#.", "...#.", "#..#.", ".##.."]),
    ('K', ["#...#", "#..#.", "###..", "#..#.", "#...#", "#...#"]),
    ('L', ["#....", "#....", "#....", "#....", "#....", "#####"]),
    ('N', ["#...#", "##..#", "#.#.#", "#..##", "#...#", "#...#"]),
    ('O', [".###.", "#...#", "#...#", "#...#", "#...#", ".###."]),
    ('Q', [".###.", "#...#", "#...#", "#.#.#", "#..#.", ".##.#"]),
    ('R', ["####.", "#...#", "#...#", "####.", "#..#.", "#...#"]),
    ('S', [".####", "#....", ".###.", "....#", "....#", "####."]),
    ('T', ["#####", "..#..", "..#..", "..#..", "..#..", "..#.."]),
    ('U', ["#...#", "#...#", "#...#", "#...#", "#...#", ".###."]),
    ('V', ["#...#", "#...#", "#...#", ".#.#.", ".#.#.", "..#.."]),
    ('W', ["#...#", "#...#", "#.#.#", "#.#.#", "##.##", "#...#"]),
    ('X', ["#...#", ".#.#.", "..#..", "..#..", ".#.#.", "#...#"]),
    ('Z', ["#####", "...#.", "..#..", ".#...", "#....", "#####"]),
    ('>', [".#...", "..#..", "...#.", "...#.", "..#..", ".#..."]),
    ('<', ["...#.", "..#..", ".#...", ".#...", "..#..", "...#."]),
    (',', [".....", ".....", ".....", ".....", "..#..", ".#..."]),
    // A flame (marks things made at a fire).
    ('^', ["..#..", "..##.", ".###.", ".####", "#####", ".###."]),
    ('-', [".....", ".....", "#####", ".....", ".....", "....."]),
    ('+', [".....", "..#..", "..#..", "#####", "..#..", "..#.."]),
    ('.', [".....", ".....", ".....", ".....", ".....", "..#.."]),
    // A cog: the lathe's mark, by recipes turned on it.
    ('*', [".#.#.", "#####", "##.##", "#####", ".#.#.", "....."]),
    // An ellipsis in one tile: what's cut off a name too long to fit.
    ('…', [".....", ".....", ".....", ".....", ".....", "#.#.#"]),
    ('!', ["..#..", "..#..", "..#..", "..#..", ".....", "..#.."]),
    ('?', [".###.", "#...#", "...#.", "..#..", ".....", "..#.."]),
    ('/', ["....#", "...#.", "..#..", "..#..", ".#...", "#...."]),
    // More above, or below, in a list that scrolls.
    ('▲', [".....", "..#..", ".###.", "#####", ".....", "....."]),
    ('▼', [".....", "#####", ".###.", "..#..", ".....", "....."]),
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

/// The title's letters: the font at twice the size, 16×16 (four tiles
/// each), with a highlight along the top and the outline kept one pixel.
pub fn title_glyph(c: char) -> Option<PixelArt> {
    let (_, rows) = GLYPHS.iter().find(|(g, _)| *g == c)?;
    let mut a = PixelArt::filled(16, 16, 0);
    for (y, row) in rows.iter().enumerate() {
        for (x, ch) in row.chars().enumerate() {
            if ch != '#' {
                continue;
            }
            for (dy, dx) in [(0, 0), (0, 1), (1, 0), (1, 1)] {
                let py = y * 2 + dy;
                a.set(x * 2 + dx + 3, py + 2, if py < 5 { 3 } else { 2 });
            }
        }
    }
    Some(a.outlined(1))
}

fn art(rows: [&str; 8]) -> PixelArt {
    PixelArt::parse(&rows, &[('1', 1), ('2', 2), ('3', 3)])
}

pub fn icon_heart() -> PixelArt {
    art([".11.11..", "1321221.", "1322221.", "1222221.", ".12221..", "..121...", "...1....", "........"])
}

pub fn icon_food() -> PixelArt {
    art(["...111..", "..13321.", ".133221.", ".132221.", "..1221..", ".131.1..", "131.....", ".1......"])
}

pub fn icon_water() -> PixelArt {
    art(["...1....", "..121...", ".12221..", ".13221..", "1322221.", "1322221.", ".12221..", "..111..."])
}

/// An upset stomach: a pictogram of someone bent over, an arm across the
/// belly. Seen from the side, the head set apart from the body as signs draw
/// people. Drawn in the text palette (navy outline, grey arm, white body).
pub fn icon_upset_stomach() -> PixelArt {
    art([".....11.", "....1331", "....1331", "...1111.", "..13331.", ".13321..", ".13231..", ".131131."])
}

/// Frames of the sign that the game is being saved: in a notebook, a pen
/// writes two lines, then the written page is turned over (its back grey)
/// to a fresh one. Drawn in the text palette.
pub const SAVED_FRAMES: usize = 10;

pub fn icon_saved() -> [PixelArt; SAVED_FRAMES] {
    const FRAMES: [[&str; 8]; SAVED_FRAMES] = [
        // The pen writes the first line...
        ["1111111.", "1331331.", "1313331.", "1333331.", "1333331.", "1333331.", "1111111.", "........"],
        ["1111111.", "1333131.", "1321331.", "1333331.", "1333331.", "1333331.", "1111111.", "........"],
        ["1111111.", "1333311.", "1322131.", "1333331.", "1333331.", "1333331.", "1111111.", "........"],
        // ...and the second.
        ["1111111.", "1333331.", "1322131.", "1331331.", "1313331.", "1333331.", "1111111.", "........"],
        ["1111111.", "1333331.", "1322211.", "1333131.", "1321331.", "1333331.", "1111111.", "........"],
        ["1111111.", "1333331.", "1322231.", "1333311.", "1322131.", "1333331.", "1111111.", "........"],
        ["1111111.", "1333331.", "1322231.", "1333331.", "1322231.", "1333331.", "1111111.", "........"],
        // The page's corner lifts, it turns over, and a fresh page shows.
        ["1111111.", "1333331.", "1322231.", "1333331.", "1322211.", "1333121.", "1111111.", "........"],
        ["1111111.", "1333221.", "1322211.", "1322131.", "1221331.", "1213331.", "1111111.", "........"],
        ["1111111.", "1221331.", "1213331.", "1133331.", "1333331.", "1333331.", "1111111.", "........"],
    ];
    FRAMES.map(art)
}

pub fn icon_sun() -> PixelArt {
    art(["...2....", ".2.1.2..", "..133...", "213321..", "..1221.2", ".2.11...", "....2...", "........"])
}

pub fn icon_moon() -> PixelArt {
    art(["..111...", ".1331...", "1331....", "1321....", "1322111.", ".122221.", "..1111..", "........"])
}

/// Quarters of a 16px bag cell: a sunken slot with a dark rim on the top
/// and left and a light one on the bottom and right.
pub fn bag_cell() -> [PixelArt; 4] {
    std::array::from_fn(|q| {
        let (right, bottom) = (q % 2 == 1, q / 2 == 1);
        let mut a = PixelArt::filled(8, 8, 1);
        for i in 0..8 {
            if !bottom {
                a.set(i, 0, 2);
            }
            if !right {
                a.set(0, i, 2);
            }
            if bottom {
                a.set(i, 7, 3);
            }
            if right {
                a.set(7, i, 3);
            }
        }
        a
    })
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

/// Size of the light mask in tiles (128x128 px).
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

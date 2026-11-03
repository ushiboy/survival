//! Metasprite helpers: build larger sprites out of 8x8 OAM entries.

use crate::ppu::banks::TileId;
use crate::ppu::oam::{Oam, OamEntry, attr};

/// Pushes a 16x16 metasprite (`[TL, TR, BL, BR]`), handling flips by swapping
/// the tile order the way NES games did.
pub fn push_16x16(oam: &mut Oam, tiles: [TileId; 4], x: i32, y: i32, attrs: u8) {
    let fh = attrs & attr::FLIP_H != 0;
    let fv = attrs & attr::FLIP_V != 0;
    for (i, &tile) in tiles.iter().enumerate() {
        let (mut col, mut row) = ((i % 2) as i32, (i / 2) as i32);
        if fh {
            col = 1 - col;
        }
        if fv {
            row = 1 - row;
        }
        oam.push(OamEntry { x: x + col * 8, y: y + row * 8, tile, attr: attrs });
    }
}

/// Pushes a 16x8 metasprite (`[L, R]`).
pub fn push_16x8(oam: &mut Oam, tiles: [TileId; 2], x: i32, y: i32, attrs: u8) {
    let fh = attrs & attr::FLIP_H != 0;
    for (i, &tile) in tiles.iter().enumerate() {
        let col = if fh { 1 - i as i32 } else { i as i32 };
        oam.push(OamEntry { x: x + col * 8, y, tile, attr: attrs });
    }
}

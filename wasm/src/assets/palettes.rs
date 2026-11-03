//! Palette assignment. The universal background colour is black, which every
//! tile can use for outlines — a common late-NES look.

use crate::ppu::palette::{BLACK, PaletteRam};

// Background palettes
/// Deep sea: navy / blue / light blue
pub const BG_DEEP: u8 = 0;
/// Shallows & surf: light blue / white foam / sand
pub const BG_SHORE: u8 = 1;
/// Beach: sand / orange / brown
pub const BG_SAND: u8 = 2;
/// Vegetation: dark green / light green / sand
pub const BG_VEG: u8 = 3;

// Sprite palettes
/// Castaway: black / skin / red
pub const SPR_PLAYER: u8 = 0;
/// Water effects: light blue / white / blue
pub const SPR_WATER: u8 = 1;
/// Crabs: black / red / white
pub const SPR_CRAB: u8 = 2;
/// UI sprites (hearts): black / red / white
pub const SPR_UI: u8 = 3;

pub fn base_palette() -> PaletteRam {
    PaletteRam::new(
        BLACK,
        [
            [0x01, 0x11, 0x21],
            [0x21, 0x30, 0x37],
            [0x37, 0x27, 0x17],
            [0x19, 0x2A, 0x37],
        ],
        [
            [BLACK, 0x36, 0x16],
            [0x21, 0x30, 0x11],
            [BLACK, 0x16, 0x30],
            [BLACK, 0x16, 0x30],
        ],
    )
}

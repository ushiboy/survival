//! Object Attribute Memory: the sprite list for one frame.

use super::banks::TileId;

/// Sprite attribute bits. Bits 0-1, 5, 6, 7 follow the NES layout.
/// Bit 2 (unused on hardware) marks a sprite as belonging to the HUD strip,
/// standing in for the sprite-0-hit split many games used.
pub mod attr {
    pub const PALETTE_MASK: u8 = 0x03;
    pub const HUD_LAYER: u8 = 0x04;
    pub const BEHIND_BG: u8 = 0x20;
    pub const FLIP_H: u8 = 0x40;
    pub const FLIP_V: u8 = 0x80;
}

pub const OAM_SIZE: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OamEntry {
    /// Screen position of the top-left pixel.
    pub x: i32,
    pub y: i32,
    pub tile: TileId,
    pub attr: u8,
}

#[derive(Default, Debug)]
pub struct Oam {
    entries: Vec<OamEntry>,
}

impl Oam {
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Returns false when OAM is full (the sprite is dropped, as on hardware).
    pub fn push(&mut self, e: OamEntry) -> bool {
        if self.entries.len() >= OAM_SIZE {
            return false;
        }
        self.entries.push(e);
        true
    }

    pub fn entries(&self) -> &[OamEntry] {
        &self.entries
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Rotates entries from `start` onward. Games did this every frame to turn
    /// per-scanline sprite dropout into flicker.
    pub fn rotate_from(&mut self, start: usize, by: usize) {
        if start < self.entries.len() {
            let tail = &mut self.entries[start..];
            let n = by % tail.len();
            tail.rotate_left(n);
        }
    }
}

//! Object Attribute Memory: the sprite list for one frame.

use super::banks::SpriteId;

pub const OAM_SIZE: usize = 128;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ObjEntry {
    /// Screen position of the top-left pixel.
    pub x: i32,
    pub y: i32,
    pub gfx: SpriteId,
    /// Sprite palette 0..=7. Palettes 4-7 take part in colour math.
    pub palette: u8,
    /// 0..=3, relative to the background priority levels.
    pub priority: u8,
    pub flip_h: bool,
    pub flip_v: bool,
}

impl ObjEntry {
    pub fn new(x: i32, y: i32, gfx: SpriteId, palette: u8, priority: u8) -> Self {
        Self { x, y, gfx, palette, priority, flip_h: false, flip_v: false }
    }

    pub fn flipped(mut self, h: bool) -> Self {
        self.flip_h = h;
        self
    }
}

#[derive(Default, Debug)]
pub struct Oam {
    entries: Vec<ObjEntry>,
}

impl Oam {
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Returns false when OAM is full (the sprite is dropped, as on hardware).
    pub fn push(&mut self, e: ObjEntry) -> bool {
        if self.entries.len() >= OAM_SIZE {
            return false;
        }
        self.entries.push(e);
        true
    }

    pub fn entries(&self) -> &[ObjEntry] {
        &self.entries
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

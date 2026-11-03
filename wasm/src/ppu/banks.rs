//! Pattern banks: the equivalent of CHR ROM/RAM contents.

use super::pattern::{PixelArt, Tile16, Tile8};

pub type MetatileId = u16;
pub type TileId = u16;

/// A 16x16 background block with one palette (the NES attribute granularity).
/// Multiple frames emulate CHR bank switching for animated tiles.
#[derive(Clone, Debug)]
pub struct Metatile {
    pub palette: u8,
    pub frames: Vec<Tile16>,
    /// Frames shown per animation step.
    pub period: u32,
}

#[derive(Default, Debug)]
pub struct MetatileBank {
    tiles: Vec<Metatile>,
}

impl MetatileBank {
    pub fn add(&mut self, palette: u8, frames: Vec<Tile16>, period: u32) -> MetatileId {
        assert!(!frames.is_empty());
        self.tiles.push(Metatile { palette, frames, period: period.max(1) });
        (self.tiles.len() - 1) as MetatileId
    }

    pub fn add_art(&mut self, palette: u8, art: &PixelArt) -> MetatileId {
        self.add(palette, vec![art.to_tile16()], 1)
    }

    #[inline]
    pub fn frame(&self, id: MetatileId, clock: u32) -> (&Tile16, u8) {
        let t = &self.tiles[id as usize];
        let f = (clock / t.period) as usize % t.frames.len();
        (&t.frames[f], t.palette)
    }

    pub fn len(&self) -> usize {
        self.tiles.len()
    }
}

/// Bank of 8x8 tiles (sprites, HUD text).
#[derive(Default, Debug)]
pub struct TileBank8 {
    tiles: Vec<Tile8>,
}

impl TileBank8 {
    pub fn add(&mut self, tile: Tile8) -> TileId {
        self.tiles.push(tile);
        (self.tiles.len() - 1) as TileId
    }

    pub fn add_art(&mut self, art: &PixelArt) -> Vec<TileId> {
        art.split8().into_iter().map(|t| self.add(t)).collect()
    }

    /// Adds a 16x16 art and returns `[TL, TR, BL, BR]`.
    pub fn add_16x16(&mut self, art: &PixelArt) -> [TileId; 4] {
        let ids = self.add_art(art);
        [ids[0], ids[1], ids[2], ids[3]]
    }

    /// Adds a 16x8 art and returns `[L, R]`.
    pub fn add_16x8(&mut self, art: &PixelArt) -> [TileId; 2] {
        let ids = self.add_art(art);
        [ids[0], ids[1]]
    }

    #[inline]
    pub fn get(&self, id: TileId) -> &Tile8 {
        &self.tiles[id as usize]
    }
}

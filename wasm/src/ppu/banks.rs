//! Pattern banks: the contents of VRAM.

use super::pattern::{PixelArt, Tile16, Tile8};

pub type MetatileId = u16;
pub type HudTileId = u16;
pub type SpriteId = u16;

/// Bytes for one 8x8 tile at a given bit depth.
const fn tile_bytes(bpp: usize) -> usize {
    8 * bpp
}

/// A 16x16 4bpp background block (four 8x8 tiles) with its palette.
/// Multiple frames stand in for tiles that are re-uploaded by DMA to animate.
#[derive(Clone, Debug)]
pub struct Metatile {
    pub palette: u8,
    pub frames: Vec<Tile16>,
    /// Video frames per animation step.
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

    /// VRAM bytes, counting one resident frame per animated tile.
    pub fn vram_bytes(&self) -> usize {
        self.tiles.len() * 4 * tile_bytes(4)
    }
}

/// 8x8 2bpp tiles for BG3.
#[derive(Default, Debug)]
pub struct HudTileBank {
    tiles: Vec<Tile8>,
}

impl HudTileBank {
    pub fn add(&mut self, art: &PixelArt) -> HudTileId {
        self.tiles.push(art.to_tile8());
        (self.tiles.len() - 1) as HudTileId
    }

    #[inline]
    pub fn get(&self, id: HudTileId) -> &Tile8 {
        &self.tiles[id as usize]
    }

    pub fn vram_bytes(&self) -> usize {
        self.tiles.len() * tile_bytes(2)
    }
}

/// A 4bpp sprite image. On hardware this is a block of 8x8 tiles drawn by
/// one or more OAM entries; sizes are multiples of 8.
#[derive(Clone, Debug)]
pub struct SpriteGfx {
    pub w: usize,
    pub h: usize,
    pub px: Vec<u8>,
}

#[derive(Default, Debug)]
pub struct SpriteBank {
    sprites: Vec<SpriteGfx>,
}

impl SpriteBank {
    pub fn add(&mut self, art: &PixelArt) -> SpriteId {
        assert!(art.w % 8 == 0 && art.h % 8 == 0, "sprite size must be a multiple of 8");
        self.sprites.push(SpriteGfx { w: art.w, h: art.h, px: art.px.clone() });
        (self.sprites.len() - 1) as SpriteId
    }

    #[inline]
    pub fn get(&self, id: SpriteId) -> &SpriteGfx {
        &self.sprites[id as usize]
    }

    pub fn vram_bytes(&self) -> usize {
        self.sprites.iter().map(|s| s.w * s.h / 2).sum()
    }
}

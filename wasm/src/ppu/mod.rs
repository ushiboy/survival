//! A software "PPU" that renders a frame under NES-like constraints:
//!
//! * 256x240 output, 64-colour master palette, 8 palettes of 3 colours (+ shared bg).
//! * Background built from 16x16 metatiles (one palette each = attribute granularity).
//! * A fixed HUD strip at the top made of 8x8 tiles (emulates a split-screen status bar).
//! * Up to 64 8x8 sprites, max 8 per scanline, lower OAM index wins, optional
//!   "behind background" priority.
//!
//! The PPU knows nothing about the game; it only reads `Vram` + `FrameState`.

pub mod banks;
pub mod oam;
pub mod palette;
pub mod pattern;

use banks::{MetatileBank, MetatileId, TileBank8, TileId};
use oam::{Oam, attr};
use palette::{NES_RGB, PaletteRam};

pub const SCREEN_W: usize = 256;
pub const SCREEN_H: usize = 240;
pub const MAX_SPRITES_PER_LINE: usize = 8;

/// Supplies background metatiles for world coordinates (in metatile units).
pub trait BgSource {
    fn metatile_at(&self, mx: i32, my: i32) -> MetatileId;
}

/// All pattern data the PPU can draw from.
#[derive(Default)]
pub struct Vram {
    pub metatiles: MetatileBank,
    pub hud_tiles: TileBank8,
    pub sprites: TileBank8,
}

pub const HUD_COLS: usize = SCREEN_W / 8;

#[derive(Clone, Copy, Debug, Default)]
pub struct HudCell {
    pub tile: TileId,
    pub palette: u8,
}

/// The fixed status bar at the top of the screen.
#[derive(Debug)]
pub struct HudLayer {
    rows: usize,
    cells: Vec<HudCell>,
}

impl HudLayer {
    pub fn new(rows: usize, blank: TileId) -> Self {
        Self { rows, cells: vec![HudCell { tile: blank, palette: 0 }; rows * HUD_COLS] }
    }

    pub fn height_px(&self) -> usize {
        self.rows * 8
    }

    pub fn fill(&mut self, tile: TileId) {
        self.cells.iter_mut().for_each(|c| *c = HudCell { tile, palette: 0 });
    }

    pub fn put(&mut self, col: usize, row: usize, tile: TileId, palette: u8) {
        if col < HUD_COLS && row < self.rows {
            self.cells[row * HUD_COLS + col] = HudCell { tile, palette };
        }
    }

    #[inline]
    fn cell(&self, col: usize, row: usize) -> HudCell {
        self.cells[row * HUD_COLS + col]
    }
}

/// Everything that changes per frame (the game's "PPU register writes").
pub struct FrameState {
    pub scroll_x: i32,
    pub scroll_y: i32,
    pub palette: PaletteRam,
    pub oam: Oam,
    pub hud: HudLayer,
    /// Global animation clock used for animated metatiles.
    pub clock: u32,
}

pub struct Ppu {
    rgba: Vec<u8>,
    line_sprites: Vec<usize>,
    /// Enforce the 8-sprites-per-scanline hardware limit.
    pub sprite_limit: bool,
    /// Set when the last frame dropped sprites on some scanline.
    pub overflowed: bool,
}

impl Default for Ppu {
    fn default() -> Self {
        Self::new()
    }
}

impl Ppu {
    pub fn new() -> Self {
        Self {
            rgba: vec![0; SCREEN_W * SCREEN_H * 4],
            line_sprites: Vec::with_capacity(64),
            sprite_limit: true,
            overflowed: false,
        }
    }

    pub fn rgba(&self) -> &[u8] {
        &self.rgba
    }

    pub fn render(&mut self, vram: &Vram, frame: &FrameState, bg: &dyn BgSource) {
        let hud_h = frame.hud.height_px();
        let mut color = [0u8; SCREEN_W];
        let mut opaque = [false; SCREEN_W];
        self.overflowed = false;

        for y in 0..SCREEN_H {
            let in_hud = y < hud_h;
            if in_hud {
                Self::hud_line(vram, frame, y, &mut color, &mut opaque);
            } else {
                Self::bg_line(vram, frame, bg, (y - hud_h) as i32, &mut color, &mut opaque);
            }
            self.sprite_line(vram, frame, y as i32, in_hud, &mut color, &opaque);

            let row = &mut self.rgba[y * SCREEN_W * 4..(y + 1) * SCREEN_W * 4];
            for (x, c) in color.iter().enumerate() {
                let rgb = NES_RGB[(*c & 0x3F) as usize];
                row[x * 4..x * 4 + 4].copy_from_slice(&[rgb[0], rgb[1], rgb[2], 255]);
            }
        }
    }

    fn hud_line(vram: &Vram, frame: &FrameState, y: usize, color: &mut [u8], opaque: &mut [bool]) {
        let (row, fy) = (y / 8, y % 8);
        for col in 0..HUD_COLS {
            let cell = frame.hud.cell(col, row);
            let tile = vram.hud_tiles.get(cell.tile);
            for fx in 0..8 {
                let idx = tile.0[fy * 8 + fx];
                color[col * 8 + fx] = frame.palette.bg_color(cell.palette, idx);
                opaque[col * 8 + fx] = idx != 0;
            }
        }
    }

    fn bg_line(
        vram: &Vram,
        frame: &FrameState,
        bg: &dyn BgSource,
        view_y: i32,
        color: &mut [u8],
        opaque: &mut [bool],
    ) {
        let wy = frame.scroll_y + view_y;
        let my = wy.div_euclid(16);
        let fy = wy.rem_euclid(16) as usize;
        let mut x = 0usize;
        let mut wx = frame.scroll_x;
        while x < SCREEN_W {
            let mx = wx.div_euclid(16);
            let fx = wx.rem_euclid(16) as usize;
            let (tile, pal) = vram.metatiles.frame(bg.metatile_at(mx, my), frame.clock);
            let row = &tile.0[fy * 16..fy * 16 + 16];
            let span = (16 - fx).min(SCREEN_W - x);
            for i in 0..span {
                let idx = row[fx + i];
                color[x + i] = frame.palette.bg_color(pal, idx);
                opaque[x + i] = idx != 0;
            }
            x += span;
            wx += span as i32;
        }
    }

    fn sprite_line(
        &mut self,
        vram: &Vram,
        frame: &FrameState,
        y: i32,
        in_hud: bool,
        color: &mut [u8],
        bg_opaque: &[bool],
    ) {
        // Sprite evaluation: first 8 matching sprites in OAM order.
        self.line_sprites.clear();
        for (i, e) in frame.oam.entries().iter().enumerate() {
            if (e.attr & attr::HUD_LAYER != 0) != in_hud {
                continue;
            }
            let row = y - e.y;
            if !(0..8).contains(&row) {
                continue;
            }
            if self.sprite_limit && self.line_sprites.len() >= MAX_SPRITES_PER_LINE {
                self.overflowed = true;
                break;
            }
            self.line_sprites.push(i);
        }
        if self.line_sprites.is_empty() {
            return;
        }

        // Lowest index opaque pixel wins, then its priority bit decides vs bg.
        let mut taken = [false; SCREEN_W];
        for &i in &self.line_sprites {
            let e = frame.oam.entries()[i];
            let mut row = (y - e.y) as usize;
            if e.attr & attr::FLIP_V != 0 {
                row = 7 - row;
            }
            let tile = vram.sprites.get(e.tile);
            let pal = e.attr & attr::PALETTE_MASK;
            for c in 0..8 {
                let sx = e.x + c;
                if !(0..SCREEN_W as i32).contains(&sx) {
                    continue;
                }
                let sx = sx as usize;
                if taken[sx] {
                    continue;
                }
                let col = if e.attr & attr::FLIP_H != 0 { 7 - c } else { c } as usize;
                let idx = tile.0[row * 8 + col];
                if idx == 0 {
                    continue;
                }
                taken[sx] = true;
                if e.attr & attr::BEHIND_BG == 0 || !bg_opaque[sx] {
                    color[sx] = frame.palette.sprite_color(pal, idx);
                }
            }
        }
    }
}

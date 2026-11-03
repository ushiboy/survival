//! A software PPU modelled on the Super Famicom / SNES in Mode 1:
//!
//! * 256x224 output, 15-bit colour, 256-entry CGRAM.
//! * BG1 / BG2: 4bpp layers of 16x16 tiles (per-tile palette, h-flip, priority bit).
//! * BG3: 2bpp layer of 8x8 tiles with its own 512x512 map and per-line
//!   scroll and main/sub-screen enables (HDMA). Used for the HUD and for a
//!   light mask on the sub-screen.
//! * Up to 128 sprites, 4bpp, priority 0-3; per-scanline limits of 32 sprites
//!   and 34 8-pixel slivers.
//! * Colour math (add/subtract, optional halving) against the sub-screen or a
//!   fixed colour, gated by a window. Fixed colour and window are per-scanline
//!   tables (HDMA).
//! * Master brightness.
//!
//! The PPU knows nothing about the game; it only reads `Vram` + `FrameState`.

pub mod banks;
pub mod effects;
pub mod oam;
pub mod palette;
pub mod pattern;

use banks::{HudTileBank, HudTileId, MetatileBank, MetatileId, SpriteBank};
use effects::{ColorMath, LineTables, MathSource, WindowRegion, layer};
use oam::Oam;
use palette::Cgram;

pub const SCREEN_W: usize = 256;
pub const SCREEN_H: usize = 224;
pub const MAX_SPRITES_PER_LINE: usize = 32;
pub const MAX_SLIVERS_PER_LINE: usize = 34;

/// One cell of a BG1/BG2 tile map.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BgCell {
    pub tile: MetatileId,
    pub hflip: bool,
    /// High priority: drawn above priority-2 sprites.
    pub high: bool,
}

impl BgCell {
    pub const EMPTY: Self = Self { tile: MetatileId::MAX, hflip: false, high: false };

    pub const fn new(tile: MetatileId) -> Self {
        Self { tile, hflip: false, high: false }
    }

    pub fn is_empty(&self) -> bool {
        self.tile == MetatileId::MAX
    }
}

/// Supplies tile-map cells for world coordinates (in 16px cell units).
pub trait BgLayer {
    fn cell(&self, mx: i32, my: i32) -> BgCell;
}

/// All pattern data the PPU can draw from.
#[derive(Default)]
pub struct Vram {
    pub metatiles: MetatileBank,
    pub hud_tiles: HudTileBank,
    pub sprites: SpriteBank,
}

/// BG3 map size in tiles (64x64 = 512x512 px, wrapping).
pub const BG3_COLS: usize = 64;
pub const BG3_ROWS: usize = 64;

#[derive(Clone, Copy, Debug, Default)]
pub struct Bg3Cell {
    /// 0 is the blank tile.
    pub tile: HudTileId,
    pub palette: u8,
}

/// BG3 tile map.
#[derive(Debug)]
pub struct Bg3Layer {
    cells: Vec<Bg3Cell>,
}

impl Default for Bg3Layer {
    fn default() -> Self {
        Self { cells: vec![Bg3Cell::default(); BG3_COLS * BG3_ROWS] }
    }
}

impl Bg3Layer {
    /// Blanks a rectangle of cells.
    pub fn clear_rect(&mut self, col: usize, row: usize, w: usize, h: usize) {
        for r in row..(row + h).min(BG3_ROWS) {
            for c in col..(col + w).min(BG3_COLS) {
                self.cells[r * BG3_COLS + c] = Bg3Cell::default();
            }
        }
    }

    pub fn put(&mut self, col: usize, row: usize, tile: HudTileId, palette: u8) {
        if col < BG3_COLS && row < BG3_ROWS {
            self.cells[row * BG3_COLS + col] = Bg3Cell { tile, palette };
        }
    }

    #[inline]
    fn cell(&self, col: usize, row: usize) -> Bg3Cell {
        self.cells[(row % BG3_ROWS) * BG3_COLS + col % BG3_COLS]
    }
}

/// Everything the game sets up per frame (register writes, DMA'd tables).
pub struct FrameState {
    /// BG1 and BG2 share the camera scroll.
    pub scroll_x: i32,
    pub scroll_y: i32,
    pub cgram: Cgram,
    pub oam: Oam,
    pub bg3: Bg3Layer,
    pub math: ColorMath,
    pub lines: LineTables,
    /// Master brightness 0..=15.
    pub brightness: u8,
    /// Global animation clock for animated tiles.
    pub clock: u32,
}

impl Default for FrameState {
    fn default() -> Self {
        Self {
            scroll_x: 0,
            scroll_y: 0,
            cgram: Cgram::default(),
            oam: Oam::default(),
            bg3: Bg3Layer::default(),
            math: ColorMath::OFF,
            lines: LineTables::default(),
            brightness: 15,
            clock: 0,
        }
    }
}

/// A background pixel: CGRAM index, or 0 for transparent.
#[derive(Clone, Copy, Default)]
struct BgPx {
    color: u16,
    high: bool,
}

#[derive(Clone, Copy, Default)]
struct ObjPx {
    color: u16,
    priority: u8,
    math: bool,
}

/// Which layer produced a main-screen pixel.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Src {
    Bg1,
    Bg2,
    Bg3,
    Obj,
    Backdrop,
}

impl Src {
    fn bit(self) -> u8 {
        match self {
            Src::Bg1 => layer::BG1,
            Src::Bg2 => layer::BG2,
            Src::Bg3 => layer::BG3,
            Src::Obj => layer::OBJ,
            Src::Backdrop => layer::BACKDROP,
        }
    }
}

pub struct Ppu {
    rgba: Vec<u8>,
    line_sprites: Vec<usize>,
    /// Enforce per-scanline sprite limits.
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
            line_sprites: Vec::with_capacity(MAX_SPRITES_PER_LINE),
            sprite_limit: true,
            overflowed: false,
        }
    }

    pub fn rgba(&self) -> &[u8] {
        &self.rgba
    }

    pub fn render(&mut self, vram: &Vram, frame: &FrameState, bg1: &dyn BgLayer, bg2: &dyn BgLayer) {
        let mut l1 = [BgPx::default(); SCREEN_W];
        let mut l2 = [BgPx::default(); SCREEN_W];
        let mut l3 = [BgPx::default(); SCREEN_W];
        let mut obj = [ObjPx::default(); SCREEN_W];
        self.overflowed = false;
        let cg = &frame.cgram.colors;
        let math = frame.math;
        let shift = 15 - frame.brightness.min(15) as u32;

        for y in 0..SCREEN_H {
            Self::bg_line(vram, frame, bg1, y, &mut l1);
            Self::bg_line(vram, frame, bg2, y, &mut l2);
            Self::bg3_line(vram, frame, y, &mut l3);
            let (bg3_main, bg3_sub) = (frame.lines.bg3_main[y], frame.lines.bg3_sub[y]);
            self.obj_line(vram, frame, y, &mut obj);

            let row = &mut self.rgba[y * SCREEN_W * 4..(y + 1) * SCREEN_W * 4];
            for x in 0..SCREEN_W {
                let (a, b, c, o) = (l1[x], l2[x], l3[x], obj[x]);
                // Mode 1 priority (front to back), BG3 tiles always high.
                let (src, idx) = if bg3_main && c.color != 0 {
                    (Src::Bg3, c.color)
                } else if o.color != 0 && o.priority == 3 {
                    (Src::Obj, o.color)
                } else if a.color != 0 && a.high {
                    (Src::Bg1, a.color)
                } else if b.color != 0 && b.high {
                    (Src::Bg2, b.color)
                } else if o.color != 0 && o.priority == 2 {
                    (Src::Obj, o.color)
                } else if a.color != 0 {
                    (Src::Bg1, a.color)
                } else if b.color != 0 {
                    (Src::Bg2, b.color)
                } else if o.color != 0 {
                    (Src::Obj, o.color)
                } else {
                    (Src::Backdrop, 0)
                };
                let mut color = cg[idx as usize];

                let math_on = math.layers & src.bit() != 0
                    && (src != Src::Obj || o.math)
                    && match math.region {
                        WindowRegion::Everywhere => true,
                        WindowRegion::Inside => frame.lines.in_window(y, x),
                        WindowRegion::Outside => !frame.lines.in_window(y, x),
                    };
                if math_on {
                    let sub = match math.source {
                        MathSource::Fixed => None,
                        MathSource::SubScreen => {
                            let c = if bg3_sub { c } else { BgPx::default() };
                            Self::sub_pixel(math.sub_layers, a, b, c)
                        }
                    };
                    color = match sub {
                        Some(s) => math.apply(color, cg[s as usize], math.half),
                        // Empty sub-screen: fixed colour, never halved.
                        None if math.source == MathSource::SubScreen => {
                            math.apply(color, frame.lines.fixed_color[y], false)
                        }
                        None => math.apply(color, frame.lines.fixed_color[y], math.half),
                    };
                }

                let mut rgba = color.to_rgba8();
                if shift > 0 {
                    for ch in rgba.iter_mut().take(3) {
                        *ch = ((*ch as u32 * (15 - shift)) / 15) as u8;
                    }
                }
                row[x * 4..x * 4 + 4].copy_from_slice(&rgba);
            }
        }
    }

    fn sub_pixel(mask: u8, a: BgPx, b: BgPx, c: BgPx) -> Option<u16> {
        if mask & layer::BG3 != 0 && c.color != 0 {
            return Some(c.color);
        }
        let use1 = mask & layer::BG1 != 0 && a.color != 0;
        let use2 = mask & layer::BG2 != 0 && b.color != 0;
        if use1 && a.high {
            Some(a.color)
        } else if use2 && b.high {
            Some(b.color)
        } else if use1 {
            Some(a.color)
        } else if use2 {
            Some(b.color)
        } else {
            None
        }
    }

    fn bg_line(vram: &Vram, frame: &FrameState, bg: &dyn BgLayer, y: usize, out: &mut [BgPx; SCREEN_W]) {
        let wy = frame.scroll_y + y as i32;
        let my = wy.div_euclid(16);
        let fy = wy.rem_euclid(16) as usize;
        let mut x = 0usize;
        let mut wx = frame.scroll_x;
        while x < SCREEN_W {
            let mx = wx.div_euclid(16);
            let fx = wx.rem_euclid(16) as usize;
            let span = (16 - fx).min(SCREEN_W - x);
            let cell = bg.cell(mx, my);
            if cell.is_empty() {
                out[x..x + span].fill(BgPx::default());
            } else {
                let (tile, pal) = vram.metatiles.frame(cell.tile, frame.clock);
                let row = &tile.0[fy * 16..fy * 16 + 16];
                let base = pal as u16 * 16;
                for i in 0..span {
                    let col = if cell.hflip { 15 - (fx + i) } else { fx + i };
                    let idx = row[col] as u16;
                    out[x + i] = BgPx { color: if idx == 0 { 0 } else { base + idx }, high: cell.high };
                }
            }
            x += span;
            wx += span as i32;
        }
    }

    fn bg3_line(vram: &Vram, frame: &FrameState, y: usize, out: &mut [BgPx; SCREEN_W]) {
        let lines = &frame.lines;
        if !lines.bg3_main[y] && !lines.bg3_sub[y] {
            out.fill(BgPx::default());
            return;
        }
        let (hofs, vofs) = lines.bg3_scroll[y];
        let wy = (vofs + y as i32).rem_euclid((BG3_ROWS * 8) as i32) as usize;
        let (row, fy) = (wy / 8, wy % 8);
        for (x, px) in out.iter_mut().enumerate() {
            let wx = (hofs + x as i32).rem_euclid((BG3_COLS * 8) as i32) as usize;
            let cell = frame.bg3.cell(wx / 8, row);
            let idx = vram.hud_tiles.get(cell.tile).0[fy * 8 + wx % 8] as u16;
            *px = BgPx { color: if idx == 0 { 0 } else { cell.palette as u16 * 4 + idx }, high: true };
        }
    }

    fn obj_line(&mut self, vram: &Vram, frame: &FrameState, y: usize, out: &mut [ObjPx; SCREEN_W]) {
        out.fill(ObjPx::default());
        let y = y as i32;

        // Range evaluation: first 32 sprites on this line; then up to 34 slivers.
        self.line_sprites.clear();
        let mut slivers = 0;
        for (i, e) in frame.oam.entries().iter().enumerate() {
            let g = vram.sprites.get(e.gfx);
            if y < e.y || y >= e.y + g.h as i32 || e.x + g.w as i32 <= 0 || e.x >= SCREEN_W as i32 {
                continue;
            }
            if self.sprite_limit {
                if self.line_sprites.len() >= MAX_SPRITES_PER_LINE || slivers + g.w / 8 > MAX_SLIVERS_PER_LINE {
                    self.overflowed = true;
                    break;
                }
                slivers += g.w / 8;
            }
            self.line_sprites.push(i);
        }

        // Lowest OAM index with an opaque pixel wins; its priority is used afterwards.
        for &i in &self.line_sprites {
            let e = frame.oam.entries()[i];
            let g = vram.sprites.get(e.gfx);
            let mut row = (y - e.y) as usize;
            if e.flip_v {
                row = g.h - 1 - row;
            }
            let base = palette::OBJ_BASE as u16 + e.palette as u16 * 16;
            for c in 0..g.w {
                let sx = e.x + c as i32;
                if !(0..SCREEN_W as i32).contains(&sx) || out[sx as usize].color != 0 {
                    continue;
                }
                let col = if e.flip_h { g.w - 1 - c } else { c };
                let idx = g.px[row * g.w + col] as u16;
                if idx != 0 {
                    out[sx as usize] = ObjPx { color: base + idx, priority: e.priority, math: e.palette >= 4 };
                }
            }
        }
    }
}

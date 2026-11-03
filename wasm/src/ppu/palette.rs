//! SNES colour model: 15-bit BGR555 colours in a 256-entry CGRAM.
//!
//! CGRAM layout (Mode 1):
//! * 0..128   — 8 background palettes of 16 colours (4bpp). Entry 0 is the backdrop.
//!   BG3 (2bpp, used for the HUD) reads 8 palettes of 4 colours from 0..32,
//!   overlapping BG palettes 0-1, so those two are reserved for the HUD.
//! * 128..256 — 8 sprite palettes of 16 colours. Entry 0 of each is transparent.
//!   Sprite palettes 4-7 are the ones affected by colour math.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rgb555(pub u16);

impl Rgb555 {
    pub const BLACK: Self = Self(0);

    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self((r as u16 & 31) | ((g as u16 & 31) << 5) | ((b as u16 & 31) << 10))
    }

    /// From 8-bit-per-channel RGB (low 3 bits are lost, as on hardware).
    pub const fn rgb8(r: u8, g: u8, b: u8) -> Self {
        Self::new(r >> 3, g >> 3, b >> 3)
    }

    #[inline]
    pub const fn r(self) -> u8 {
        (self.0 & 31) as u8
    }

    #[inline]
    pub const fn g(self) -> u8 {
        ((self.0 >> 5) & 31) as u8
    }

    #[inline]
    pub const fn b(self) -> u8 {
        ((self.0 >> 10) & 31) as u8
    }

    #[inline]
    pub fn to_rgba8(self) -> [u8; 4] {
        let x = |c: u8| (c << 3) | (c >> 2);
        [x(self.r()), x(self.g()), x(self.b()), 255]
    }

    /// Channel-wise transform in 0..=31 space (used for palette fades/tints).
    pub fn map(self, f: impl Fn(usize, f32) -> f32) -> Self {
        let c = |i: usize, v: u8| f(i, v as f32).round().clamp(0.0, 31.0) as u8;
        Self::new(c(0, self.r()), c(1, self.g()), c(2, self.b()))
    }

    pub fn lerp(self, other: Self, t: f32) -> Self {
        let o = [other.r(), other.g(), other.b()];
        self.map(|i, v| v + (o[i] as f32 - v) * t)
    }
}

pub const CGRAM_SIZE: usize = 256;
pub const OBJ_BASE: usize = 128;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cgram {
    pub colors: [Rgb555; CGRAM_SIZE],
}

impl Default for Cgram {
    fn default() -> Self {
        Self { colors: [Rgb555::BLACK; CGRAM_SIZE] }
    }
}

impl Cgram {
    #[inline]
    pub const fn bg_index(palette: u8, index: u8) -> usize {
        palette as usize * 16 + index as usize
    }

    #[inline]
    pub const fn obj_index(palette: u8, index: u8) -> usize {
        OBJ_BASE + palette as usize * 16 + index as usize
    }

    #[inline]
    pub const fn hud_index(palette: u8, index: u8) -> usize {
        palette as usize * 4 + index as usize
    }

    pub fn backdrop(&self) -> Rgb555 {
        self.colors[0]
    }

    /// Writes entries 1.. of a 16-colour background palette.
    pub fn set_bg_palette(&mut self, palette: u8, colors: &[Rgb555]) {
        let base = Self::bg_index(palette, 1);
        self.colors[base..base + colors.len()].copy_from_slice(colors);
    }

    pub fn set_obj_palette(&mut self, palette: u8, colors: &[Rgb555]) {
        let base = Self::obj_index(palette, 1);
        self.colors[base..base + colors.len()].copy_from_slice(colors);
    }

    /// Writes entries 1..=3 of a 4-colour BG3 (HUD) palette.
    pub fn set_hud_palette(&mut self, palette: u8, colors: [Rgb555; 3]) {
        let base = Self::hud_index(palette, 1);
        self.colors[base..base + 3].copy_from_slice(&colors);
    }

    pub fn obj_palette(&self, palette: u8) -> [Rgb555; 15] {
        let base = Self::obj_index(palette, 1);
        let mut out = [Rgb555::BLACK; 15];
        out.copy_from_slice(&self.colors[base..base + 15]);
        out
    }

    /// Rotates `len` entries starting at `start` by `by` (palette cycling).
    pub fn rotate(&mut self, start: usize, len: usize, by: usize) {
        self.colors[start..start + len].rotate_right(by % len);
    }
}

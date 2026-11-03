//! NES master palette and the 32-byte palette RAM.

/// The 64-colour NTSC 2C02 master palette (RGB).
pub const NES_RGB: [[u8; 3]; 64] = [
    [84, 84, 84], [0, 30, 116], [8, 16, 144], [48, 0, 136],
    [68, 0, 100], [92, 0, 48], [84, 4, 0], [60, 24, 0],
    [32, 42, 0], [8, 58, 0], [0, 64, 0], [0, 60, 0],
    [0, 50, 60], [0, 0, 0], [0, 0, 0], [0, 0, 0],
    [152, 150, 152], [8, 76, 196], [48, 50, 236], [92, 30, 228],
    [136, 20, 176], [160, 20, 100], [152, 34, 32], [120, 60, 0],
    [84, 90, 0], [40, 114, 0], [8, 124, 0], [0, 118, 40],
    [0, 102, 120], [0, 0, 0], [0, 0, 0], [0, 0, 0],
    [236, 238, 236], [76, 154, 236], [120, 124, 236], [176, 98, 236],
    [228, 84, 236], [236, 88, 180], [236, 106, 100], [212, 136, 32],
    [160, 170, 0], [116, 196, 0], [76, 208, 32], [56, 204, 108],
    [56, 180, 204], [60, 60, 60], [0, 0, 0], [0, 0, 0],
    [236, 238, 236], [168, 204, 236], [188, 188, 236], [212, 178, 236],
    [236, 174, 236], [236, 174, 212], [236, 180, 176], [228, 196, 144],
    [204, 210, 120], [180, 222, 120], [168, 226, 144], [152, 226, 180],
    [160, 214, 228], [160, 162, 160], [0, 0, 0], [0, 0, 0],
];

pub const BLACK: u8 = 0x0F;

/// Palette RAM layout matches the PPU: 4 background palettes followed by
/// 4 sprite palettes, 4 entries each. Entry 0 of every background palette
/// is the shared "universal" background colour; entry 0 of sprite palettes
/// is transparent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PaletteRam {
    pub data: [u8; 32],
}

impl PaletteRam {
    pub fn new(universal: u8, bg: [[u8; 3]; 4], sprites: [[u8; 3]; 4]) -> Self {
        let mut data = [universal; 32];
        for (p, cols) in bg.iter().enumerate() {
            data[p * 4 + 1..p * 4 + 4].copy_from_slice(cols);
        }
        for (p, cols) in sprites.iter().enumerate() {
            data[16 + p * 4 + 1..16 + p * 4 + 4].copy_from_slice(cols);
        }
        Self { data }
    }

    #[inline]
    pub fn bg_color(&self, palette: u8, index: u8) -> u8 {
        if index == 0 {
            self.data[0]
        } else {
            self.data[palette as usize * 4 + index as usize]
        }
    }

    #[inline]
    pub fn sprite_color(&self, palette: u8, index: u8) -> u8 {
        self.data[16 + palette as usize * 4 + index as usize]
    }

    /// Applies a colour transform to every entry (palette fades etc.).
    pub fn map_colors(&self, f: impl Fn(u8) -> u8) -> Self {
        let mut out = *self;
        for c in out.data.iter_mut() {
            *c = f(*c);
        }
        out
    }
}

/// Classic NES fade: subtract 0x10 per step; anything below the darkest row
/// becomes black.
pub fn darken(color: u8, steps: u8) -> u8 {
    if color & 0x0F >= 0x0D {
        return BLACK;
    }
    let lum = color >> 4;
    if lum < steps { BLACK } else { color - steps * 0x10 }
}

/// Night look: one fade step plus a hue rotation toward blue (hue 2).
pub fn nightify(color: u8) -> u8 {
    let c = darken(color, 1);
    let hue = c & 0x0F;
    if c == BLACK || hue == 0 || hue > 0x0C {
        return c;
    }
    // Rotate up to two steps around the 12-hue wheel toward hue 2.
    let target = 2i32;
    let mut h = hue as i32;
    for _ in 0..2 {
        let diff = (target - h).rem_euclid(12);
        if diff == 0 {
            break;
        }
        h += if diff <= 6 { 1 } else { -1 };
        h = (h - 1).rem_euclid(12) + 1;
    }
    (c & 0x30) | h as u8
}

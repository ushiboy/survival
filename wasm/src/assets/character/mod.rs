//! The castaway as a paper doll: a pose skeleton rendered in layers (body,
//! clothes per body part, hair, headwear). Equipment changes swap the shape
//! style of a slot and its colour ramp in the sprite palette, then frames are
//! re-rendered — the way SNES games streamed character tiles into VRAM.

pub mod pose;
pub mod render;

use crate::ppu::palette::Rgb555;

/// Character canvas (pixels). Wider/taller than the 16x24 collision frame so
/// limbs and tools can extend past it.
pub const CHAR_W: usize = 24;
pub const CHAR_H: usize = 32;
/// Where the 16x24 logical frame sits inside the canvas.
pub const CHAR_ORIGIN: (i32, i32) = (4, 8);
/// Canvas row of the waterline when wading.
pub const WADE_ROW: usize = 25;

/// Sprite palette slots (shared by every outfit).
pub mod slot {
    pub const OUTLINE: u8 = 1;
    pub const SKIN: [u8; 3] = [2, 3, 4];
    pub const HAIR: [u8; 3] = [5, 6, 7];
    pub const TOP: [u8; 3] = [8, 9, 10];
    pub const BOTTOM: [u8; 3] = [11, 12, 13];
    /// Accent: dark, base (bandana, hat, straps).
    pub const ACCENT: [u8; 2] = [14, 15];
}

/// The lying pose turned a quarter so the body lies head-left: 32x24.
pub fn render_lying(outfit: &Outfit, eyes_open: bool) -> crate::ppu::pattern::PixelArt {
    // Rotate counter-clockwise: the top of the standing render goes left.
    render::render(&pose::lying(eyes_open), outfit).body.transposed().flipped_v()
}

/// The sleeping pose turned a quarter so the sleeper lies head-left, front
/// toward the camera: 32x24.
pub fn render_sleeping(outfit: &Outfit) -> crate::ppu::pattern::PixelArt {
    render::render(&pose::sleeping(), outfit).body.transposed()
}

/// A colour ramp: dark, base, light.
pub type Ramp = [Rgb555; 3];

const fn c(r: u8, g: u8, b: u8) -> Rgb555 {
    Rgb555::rgb8(r, g, b)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Headwear {
    None,
    Bandana,
    StrawHat,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sleeve {
    None,
    Short,
    /// Long sleeves rolled up just past the elbow.
    Rolled,
    Long,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Top {
    Bare,
    /// `collar` gives a dress-shirt collar and button placket.
    Shirt { sleeve: Sleeve, torn: bool, collar: bool },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Legwear {
    Shorts,
    Pants,
    /// Denim trousers: faded fronts, a ripped knee, rolled hems.
    Jeans,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Footwear {
    Barefoot,
    Sandals,
    /// Leather boots (drawn with the hair ramp, a dark brown).
    Boots,
    /// Canvas sneakers in the accent colours (sole/laces, upper).
    Sneakers,
}

/// Everything worn, by shape and colour.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Outfit {
    pub head: Headwear,
    pub top: Top,
    pub legs: Legwear,
    pub feet: Footwear,
    pub top_colors: Ramp,
    pub bottom_colors: Ramp,
    pub accent: [Rgb555; 2],
}

impl Outfit {
    /// Torn white dress shirt with rolled sleeves, ripped jeans, sneakers.
    pub const CASTAWAY: Self = Self {
        head: Headwear::None,
        top: Top::Shirt { sleeve: Sleeve::Rolled, torn: true, collar: true },
        legs: Legwear::Jeans,
        feet: Footwear::Sneakers,
        top_colors: [c(150, 150, 156), c(216, 214, 204), c(250, 250, 244)],
        bottom_colors: [c(34, 54, 100), c(58, 90, 152), c(104, 140, 196)],
        accent: [c(104, 110, 128), c(234, 234, 226)],
    };

    /// Straw hat, long-sleeved shirt, trousers, boots.
    pub const EXPLORER: Self = Self {
        head: Headwear::StrawHat,
        top: Top::Shirt { sleeve: Sleeve::Long, torn: false, collar: true },
        legs: Legwear::Pants,
        feet: Footwear::Boots,
        top_colors: [c(54, 96, 70), c(84, 140, 96), c(132, 186, 132)],
        bottom_colors: [c(92, 70, 48), c(138, 108, 72), c(184, 152, 108)],
        accent: [c(176, 136, 64), c(232, 200, 120)],
    };

    /// Shirtless, cut-off shorts, sandals.
    pub const SUNBURNT: Self = Self {
        head: Headwear::None,
        top: Top::Bare,
        legs: Legwear::Shorts,
        feet: Footwear::Sandals,
        top_colors: [c(0, 0, 0), c(0, 0, 0), c(0, 0, 0)],
        bottom_colors: [c(28, 86, 90), c(44, 134, 130), c(96, 186, 170)],
        accent: [c(92, 58, 34), c(150, 104, 60)],
    };

    pub const PRESETS: [Self; 3] = [Self::CASTAWAY, Self::EXPLORER, Self::SUNBURNT];

    /// The 15 sprite palette colours for this outfit.
    pub fn palette(&self) -> [Rgb555; 15] {
        let skin = [c(170, 98, 66), c(226, 154, 108), c(250, 200, 154)];
        let hair = [c(50, 30, 24), c(94, 56, 34), c(148, 98, 58)];
        let mut p = [Rgb555::BLACK; 15];
        p[0] = c(36, 20, 26);
        p[1..4].copy_from_slice(&skin);
        p[4..7].copy_from_slice(&hair);
        p[7..10].copy_from_slice(&self.top_colors);
        p[10..13].copy_from_slice(&self.bottom_colors);
        p[13..15].copy_from_slice(&self.accent);
        p
    }
}

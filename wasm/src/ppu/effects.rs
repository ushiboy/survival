//! Colour math, windows and per-scanline (HDMA) tables.

use super::SCREEN_H;
use super::palette::Rgb555;

/// Layer bits used by colour-math and sub-screen masks.
pub mod layer {
    pub const BG1: u8 = 0x01;
    pub const BG2: u8 = 0x02;
    pub const BG3: u8 = 0x04;
    /// Only sprites using palettes 4-7 are affected, as on hardware.
    pub const OBJ: u8 = 0x10;
    pub const BACKDROP: u8 = 0x20;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MathOp {
    Add,
    Sub,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MathSource {
    /// Blend with the sub-screen (falls back to the fixed colour where it is empty).
    SubScreen,
    /// Blend with the per-scanline fixed colour.
    Fixed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowRegion {
    Everywhere,
    Inside,
    Outside,
}

/// The CGWSEL/CGADSUB register state, simplified.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ColorMath {
    pub layers: u8,
    pub op: MathOp,
    pub half: bool,
    pub source: MathSource,
    pub region: WindowRegion,
    /// Layers rendered to the sub-screen.
    pub sub_layers: u8,
}

impl ColorMath {
    pub const OFF: Self = Self {
        layers: 0,
        op: MathOp::Add,
        half: false,
        source: MathSource::Fixed,
        region: WindowRegion::Everywhere,
        sub_layers: 0,
    };

    #[inline]
    pub fn apply(&self, main: Rgb555, other: Rgb555, half: bool) -> Rgb555 {
        let ch = |a: u8, b: u8| -> u8 {
            let v = match self.op {
                MathOp::Add => a as i32 + b as i32,
                MathOp::Sub => a as i32 - b as i32,
            };
            let v = if half { v >> 1 } else { v };
            v.clamp(0, 31) as u8
        };
        Rgb555::new(ch(main.r(), other.r()), ch(main.g(), other.g()), ch(main.b(), other.b()))
    }
}

/// Values the game writes per scanline through HDMA.
#[derive(Clone, Debug)]
pub struct LineTables {
    pub fixed_color: Vec<Rgb555>,
    /// Window 1 span per line as `[left, right]` inclusive; empty when left > right.
    pub window: Vec<(i16, i16)>,
    /// BG3 scroll (h, v) per line.
    pub bg3_scroll: Vec<(i32, i32)>,
    /// BG3 enabled on the main screen / sub-screen per line (TM/TS writes).
    pub bg3_main: Vec<bool>,
    pub bg3_sub: Vec<bool>,
    /// Extra BG2 horizontal scroll per line (e.g. shaking a tree canopy).
    pub bg2_hofs: Vec<i32>,
}

impl Default for LineTables {
    fn default() -> Self {
        Self {
            fixed_color: vec![Rgb555::BLACK; SCREEN_H],
            window: vec![(1, 0); SCREEN_H],
            bg3_scroll: vec![(0, 0); SCREEN_H],
            bg3_main: vec![true; SCREEN_H],
            bg3_sub: vec![false; SCREEN_H],
            bg2_hofs: vec![0; SCREEN_H],
        }
    }
}

impl LineTables {
    pub fn clear(&mut self) {
        self.fixed_color.fill(Rgb555::BLACK);
        self.window.fill((1, 0));
        self.bg3_scroll.fill((0, 0));
        self.bg3_main.fill(true);
        self.bg3_sub.fill(false);
        self.bg2_hofs.fill(0);
    }

    #[inline]
    pub fn in_window(&self, line: usize, x: usize) -> bool {
        let (l, r) = self.window[line];
        (l as i32..=r as i32).contains(&(x as i32))
    }
}

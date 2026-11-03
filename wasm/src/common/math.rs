//! Fixed-point helpers and simple geometry.
//!
//! Positions of moving objects are kept in 8.8 fixed point (like a lot of NES
//! games did with a "subpixel" byte), which keeps movement deterministic and
//! lets speeds such as 1.25 px/frame be expressed exactly.

pub const FX_SHIFT: i32 = 8;
pub const FX_ONE: i32 = 1 << FX_SHIFT;

#[inline]
pub const fn px_to_fx(px: i32) -> i32 {
    px << FX_SHIFT
}

#[inline]
pub const fn fx_to_px(fx: i32) -> i32 {
    fx >> FX_SHIFT
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dir {
    Up,
    Down,
    Left,
    Right,
}

/// Axis-aligned rectangle in pixels. `right()`/`bottom()` are exclusive.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    pub const fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self { x, y, w, h }
    }

    #[inline]
    pub const fn right(&self) -> i32 {
        self.x + self.w
    }

    #[inline]
    pub const fn bottom(&self) -> i32 {
        self.y + self.h
    }

    #[inline]
    pub const fn translate(self, dx: i32, dy: i32) -> Self {
        Self::new(self.x + dx, self.y + dy, self.w, self.h)
    }

    pub const fn center(&self) -> (i32, i32) {
        (self.x + self.w / 2, self.y + self.h / 2)
    }
}

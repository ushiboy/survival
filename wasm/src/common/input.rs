//! Controller input modelled on the SNES joypad (12 buttons, the 16-bit
//! auto-read layout). The host (JS) maps keyboard / gamepad to these bits.

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct Buttons(u16);

impl Buttons {
    pub const B: u16 = 0x8000;
    pub const Y: u16 = 0x4000;
    pub const SELECT: u16 = 0x2000;
    pub const START: u16 = 0x1000;
    pub const UP: u16 = 0x0800;
    pub const DOWN: u16 = 0x0400;
    pub const LEFT: u16 = 0x0200;
    pub const RIGHT: u16 = 0x0100;
    pub const A: u16 = 0x0080;
    pub const X: u16 = 0x0040;
    pub const L: u16 = 0x0020;
    pub const R: u16 = 0x0010;

    /// Builds a button state, cancelling opposite directions the way a
    /// physical D-pad cannot produce them.
    pub fn from_raw(raw: u16) -> Self {
        let mut b = raw;
        if b & (Self::UP | Self::DOWN) == Self::UP | Self::DOWN {
            b &= !(Self::UP | Self::DOWN);
        }
        if b & (Self::LEFT | Self::RIGHT) == Self::LEFT | Self::RIGHT {
            b &= !(Self::LEFT | Self::RIGHT);
        }
        Self(b)
    }

    #[inline]
    pub fn held(self, mask: u16) -> bool {
        self.0 & mask != 0
    }

    /// D-pad as a (-1..=1, -1..=1) vector.
    pub fn axis(self) -> (i32, i32) {
        let x = self.held(Self::RIGHT) as i32 - self.held(Self::LEFT) as i32;
        let y = self.held(Self::DOWN) as i32 - self.held(Self::UP) as i32;
        (x, y)
    }
}

/// Current + previous frame, for edge detection.
#[derive(Default, Debug)]
pub struct InputState {
    cur: Buttons,
    prev: Buttons,
}

impl InputState {
    pub fn update(&mut self, raw: u16) {
        self.prev = self.cur;
        self.cur = Buttons::from_raw(raw);
    }

    pub fn held(&self, mask: u16) -> bool {
        self.cur.held(mask)
    }

    pub fn pressed(&self, mask: u16) -> bool {
        self.cur.held(mask) && !self.prev.held(mask)
    }

    pub fn axis(&self) -> (i32, i32) {
        self.cur.axis()
    }
}

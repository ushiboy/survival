//! Controller input modelled on the NES joypad (8 buttons, one byte).
//! The host (JS) is responsible for mapping keyboard / gamepad to these bits.

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct Buttons(u8);

impl Buttons {
    pub const A: u8 = 0x01;
    pub const B: u8 = 0x02;
    pub const SELECT: u8 = 0x04;
    pub const START: u8 = 0x08;
    pub const UP: u8 = 0x10;
    pub const DOWN: u8 = 0x20;
    pub const LEFT: u8 = 0x40;
    pub const RIGHT: u8 = 0x80;

    /// Builds a button state, cancelling opposite directions the way a
    /// physical D-pad cannot produce them.
    pub fn from_raw(raw: u8) -> Self {
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
    pub fn held(self, mask: u8) -> bool {
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
    pub fn update(&mut self, raw: u8) {
        self.prev = self.cur;
        self.cur = Buttons::from_raw(raw);
    }

    pub fn held(&self, mask: u8) -> bool {
        self.cur.held(mask)
    }

    pub fn pressed(&self, mask: u8) -> bool {
        self.cur.held(mask) && !self.prev.held(mask)
    }

    pub fn axis(&self) -> (i32, i32) {
        self.cur.axis()
    }
}

use crate::common::math::Rect;
use crate::ppu::{SCREEN_H, SCREEN_W};

/// Viewport into the world, in pixels.
#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    /// Screen row where the playfield starts (below the HUD).
    pub screen_top: i32,
}

impl Camera {
    pub fn new(hud_height: i32) -> Self {
        Self { x: 0, y: 0, w: SCREEN_W as i32, h: SCREEN_H as i32 - hud_height, screen_top: hud_height }
    }

    /// Centres on a world point, clamped to the world bounds.
    pub fn follow(&mut self, at: (i32, i32), (world_w, world_h): (i32, i32)) {
        self.follow_within(at, Rect::new(0, 0, world_w, world_h));
    }

    /// Centres on a world point, clamped to the bounds `within` (world
    /// px).
    pub fn follow_within(&mut self, (cx, cy): (i32, i32), within: Rect) {
        self.x = (cx - self.w / 2).clamp(within.x, (within.right() - self.w).max(within.x));
        self.y = (cy - self.h / 2).clamp(within.y, (within.bottom() - self.h).max(within.y));
    }

    pub fn to_screen(&self, wx: i32, wy: i32) -> (i32, i32) {
        (wx - self.x, wy - self.y + self.screen_top)
    }

    /// Whether a world-space box is (at least partly) on screen.
    pub fn sees(&self, wx: i32, wy: i32, w: i32, h: i32) -> bool {
        wx + w > self.x && wy + h > self.y && wx < self.x + self.w && wy < self.y + self.h
    }
}

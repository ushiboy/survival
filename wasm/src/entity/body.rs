//! Position + collision box, and tile-collision movement shared by all actors.

use crate::common::math::{FX_ONE, Rect, fx_to_px, px_to_fx};

#[derive(Clone, Debug)]
pub struct Body {
    /// Sprite top-left in 8.8 fixed point.
    pub x: i32,
    pub y: i32,
    /// Collision box relative to the sprite top-left (usually just the feet).
    pub hitbox: Rect,
}

impl Body {
    pub fn new(px: i32, py: i32, hitbox: Rect) -> Self {
        Self { x: px_to_fx(px), y: px_to_fx(py), hitbox }
    }

    pub fn px(&self) -> (i32, i32) {
        (fx_to_px(self.x), fx_to_px(self.y))
    }

    pub fn hitbox_at(&self, px: i32, py: i32) -> Rect {
        self.hitbox.translate(px, py)
    }

    pub fn world_hitbox(&self) -> Rect {
        let (x, y) = self.px();
        self.hitbox_at(x, y)
    }

    pub fn feet(&self) -> (i32, i32) {
        self.world_hitbox().center()
    }

    /// Moves by a fixed-point delta, one axis at a time so walls can be slid
    /// along. When blocked head-on, `corner_assist` (px) nudges the body
    /// sideways around corners — the "forgiving" feel of good action games.
    /// Returns true if the position changed.
    pub fn move_by(&mut self, dx: i32, dy: i32, corner_assist: i32, blocked: &impl Fn(Rect) -> bool) -> bool {
        let before = (self.x, self.y);
        if !self.move_axis(dx, true, blocked) && dy == 0 {
            self.assist(true, dx.signum(), corner_assist, blocked);
        }
        if !self.move_axis(dy, false, blocked) && dx == 0 {
            self.assist(false, dy.signum(), corner_assist, blocked);
        }
        before != (self.x, self.y)
    }

    /// Returns false if movement was stopped by a wall.
    fn move_axis(&mut self, delta: i32, horizontal: bool, blocked: &impl Fn(Rect) -> bool) -> bool {
        if delta == 0 {
            return true;
        }
        let cur = if horizontal { self.x } else { self.y };
        let target = cur + delta;
        let (from_px, to_px) = (fx_to_px(cur), fx_to_px(target));
        let step = (to_px - from_px).signum();
        let mut p = from_px;
        while p != to_px {
            let next = p + step;
            let r = if horizontal { self.hitbox_at(next, fx_to_px(self.y)) } else { self.hitbox_at(fx_to_px(self.x), next) };
            if blocked(r) {
                // Stop flush against the wall, keeping the subpixel pressed into it.
                let snapped = px_to_fx(p) + if step > 0 { FX_ONE - 1 } else { 0 };
                self.set_axis(horizontal, snapped);
                return false;
            }
            p = next;
        }
        self.set_axis(horizontal, target);
        true
    }

    fn set_axis(&mut self, horizontal: bool, v: i32) {
        if horizontal { self.x = v } else { self.y = v }
    }

    fn assist(&mut self, horizontal: bool, dir: i32, range: i32, blocked: &impl Fn(Rect) -> bool) {
        let (px, py) = self.px();
        for o in 1..=range {
            for side in [-1, 1] {
                let shift = side * o;
                let (probe, nudge) = if horizontal {
                    (self.hitbox_at(px + dir, py + shift), self.hitbox_at(px, py + side))
                } else {
                    (self.hitbox_at(px + shift, py + dir), self.hitbox_at(px + side, py))
                };
                if !blocked(probe) && !blocked(nudge) {
                    if horizontal {
                        self.y += px_to_fx(side);
                    } else {
                        self.x += px_to_fx(side);
                    }
                    return;
                }
            }
        }
    }
}

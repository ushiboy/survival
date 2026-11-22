//! The handcart on the island: where it stands, whether the castaway is
//! pulling it, and which way its poles point. A pulled cart is hitched
//! right behind the castaway, its poles' crossbar in their hands, and
//! swings round behind them as they turn (it's drawn so, with the
//! castaway in a pose holding the poles). What it carries is kept with
//! what the castaway carries (`Inventory::cart`).

use crate::common::math::{Dir, Rect};

/// The cart's load: a grid of this many cells across and rows deep (twice
/// a bag's, more than a bag with a pack frame on).
pub const CART_W: i32 = 8;
pub const CART_ROWS: i32 = 6;
/// Half the cart's footprint on the ground (px), across and along its
/// poles (the mill's door opens to it). It's in no
/// one's way (the castaway walks through it, and pulling it bumps into
/// things only as they do). Seen side on (poles left or right) its middle
/// is its wheel's hub, and it rests on the ground under that.
pub const CART_HALF: (i32, i32) = (11, 4);
pub const CART_HALF_ALONG: (i32, i32) = (10, 6);
const SIDE_GROUND: i32 = 4;
/// How close (px) the castaway's feet must be to the middle of a cart set
/// down for its load to be at hand, and for Y to take it up (whichever way
/// they face).
pub const CART_REACH: i32 = 32;

/// Where a pulled cart's middle stands from the feet of the castaway
/// pulling it (px), facing `facing`: behind them, placed so the crossbar
/// at the end of its poles is in their hands as the pulling pose holds
/// them (`character::pose::pull`); side on, its wheel's foot is level with
/// the soles of their shoes (2px below their feet). Pulled down the screen
/// it's just behind them, its bed showing over their head by half a head;
/// pulled up it, just before them, hiding them from the waist down.
pub fn hitch_offset(facing: Dir) -> (i32, i32) {
    match facing {
        Dir::Right => (-19, -4),
        Dir::Left => (19, -4),
        Dir::Down => (0, -16),
        Dir::Up => (0, 6),
    }
}

/// The ground a cart with its middle at `at` and its poles pointing
/// `poles` takes (world px): longer along its poles than across.
pub fn footprint(at: (i32, i32), poles: Dir) -> Rect {
    let ((hw, hh), y) = match poles {
        Dir::Left | Dir::Right => (CART_HALF, at.1 + SIDE_GROUND),
        Dir::Up | Dir::Down => (CART_HALF_ALONG, at.1),
    };
    Rect::new(at.0 - hw, y - hh, hw * 2, hh * 2)
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Cart {
    /// Its middle on the ground (world px).
    pub at: (i32, i32),
    /// Being pulled (hitched behind the castaway), or set down.
    pub pulled: bool,
    /// Which way its poles point: the way the castaway pulling it faces,
    /// and left that way when it's set down.
    pub poles: Dir,
}

impl Cart {
    /// A cart just made, pulled by feet at `feet` facing `facing`.
    pub fn behind(feet: (i32, i32), facing: Dir) -> Self {
        let mut cart = Self { at: feet, pulled: true, poles: facing };
        cart.hitch(feet, facing);
        cart
    }

    /// The ground it takes (world px).
    pub fn rect(&self) -> Rect {
        footprint(self.at, self.poles)
    }

    /// Takes hold of it to pull it again.
    pub fn take_up(&mut self) {
        self.pulled = true;
    }

    /// Sets it down where it stands.
    pub fn set_down(&mut self) {
        self.pulled = false;
    }

    /// A frame of being pulled by feet at `feet` facing `facing`: it's
    /// right behind them. A cart set down stays put.
    pub fn hitch(&mut self, feet: (i32, i32), facing: Dir) {
        if !self.pulled {
            return;
        }
        let (dx, dy) = hitch_offset(facing);
        self.at = (feet.0 + dx, feet.1 + dy);
        self.poles = facing;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pulled_cart_is_hitched_right_behind_whichever_way_the_castaway_faces() {
        let feet = (100, 100);
        /// A facing, and whether a cart's middle is behind feet at
        /// (100, 100) facing that way.
        type Behind = (Dir, fn((i32, i32)) -> bool);
        let checks: [Behind; 4] = [(Dir::Right, |c| c.0 < 100), (Dir::Left, |c| c.0 > 100), (Dir::Down, |c| c.1 < 100), (Dir::Up, |c| c.1 > 100)];
        for (facing, behind) in checks {
            let mut cart = Cart::behind(feet, Dir::Right);
            cart.hitch(feet, facing);
            assert!(behind(cart.at), "{facing:?}");
            assert_eq!(cart.poles, facing, "its poles point the way the castaway faces");
        }
    }

    #[test]
    fn a_cart_set_down_stays_put() {
        let mut cart = Cart::behind((100, 100), Dir::Right);
        cart.set_down();
        let at = cart.at;
        cart.hitch((200, 200), Dir::Up);
        assert_eq!(cart.at, at);
    }

    #[test]
    fn a_cart_takes_its_room_where_it_stands_behind() {
        for facing in [Dir::Right, Dir::Left, Dir::Down, Dir::Up] {
            let mut cart = Cart::behind((100, 100), Dir::Right);
            cart.hitch((100, 100), facing);
            let r = cart.rect();
            let (cx, cy) = cart.at;
            assert!(r.x < cx && cx < r.x + r.w && r.y <= cy && cy <= r.y + r.h, "round its middle, {facing:?}");
            assert!(r.w > r.h, "wider across the screen than deep, {facing:?}");
        }
    }
}

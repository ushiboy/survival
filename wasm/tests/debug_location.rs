//! Scenario: the debug panel's "where am I" — the cell, the feet in pixels,
//! the facing, and the hours since the start (what `SNAP_HOURS` takes) — so
//! a spot met in the browser can be found again and drawn by the snapshot.

mod common;

use common::{hold, new_game};
use survival_core::common::input::Buttons;
use survival_core::common::math::Dir;
use survival_core::world::TILE_SIZE;

#[test]
fn the_feet_lie_in_the_cell_the_castaway_stands_in() {
    let game = new_game();
    let (x, y) = game.player_feet();
    assert_eq!(game.player_tile(), (x.div_euclid(TILE_SIZE), y.div_euclid(TILE_SIZE)));
}

#[test]
fn the_facing_follows_the_way_the_castaway_walked() {
    let mut game = new_game();
    hold(&mut game, Buttons::LEFT, 4);
    assert_eq!(game.player_facing(), Dir::Left);
}

#[test]
fn the_hours_since_the_start_count_up_with_the_clock() {
    let mut game = new_game();
    assert_eq!(game.debug_hours_since_start(), 0);
    game.debug_advance_hours(5);
    assert_eq!(game.debug_hours_since_start(), 5);
}

//! Driftwood washing up: on the west coast's beach only, now and then, up
//! to as many logs as it held at first.

mod common;

use common::{hold, new_game};
use survival_core::game::Game;

/// Lets `days` mornings come, a frame each.
fn mornings(game: &mut Game, days: u32) {
    for _ in 0..days {
        game.debug_advance_hours(24);
        hold(game, 0, 1);
    }
}

#[test]
fn split_driftwood_on_the_west_coast_washes_up_again() {
    let mut game = new_game();
    let (_, west) = game.debug_driftwood();
    assert!(west > 0, "the west coast has driftwood");
    game.debug_clear_driftwood();
    mornings(&mut game, 60);
    assert_eq!(game.debug_driftwood().1, west, "back to what the west coast held at first, no more");
}

#[test]
fn split_driftwood_in_the_first_area_is_gone_for_good() {
    let mut game = new_game();
    let (first, _) = game.debug_driftwood();
    assert!(first > 0, "the first area has driftwood");
    game.debug_clear_driftwood();
    mornings(&mut game, 60);
    assert_eq!(game.debug_driftwood().0, 0);
}

#[test]
fn driftwood_washes_up_only_some_mornings() {
    let mut game = new_game();
    let mut mornings_with_a_log = 0;
    for _ in 0..12 {
        game.debug_clear_driftwood();
        mornings(&mut game, 1);
        if game.debug_driftwood().1 > 0 {
            mornings_with_a_log += 1;
        }
    }
    assert!(mornings_with_a_log > 0 && mornings_with_a_log < 12, "{mornings_with_a_log} of 12 mornings");
}

#[test]
fn driftwood_that_washed_up_is_still_there_after_a_save() {
    let mut game = new_game();
    game.debug_clear_driftwood();
    mornings(&mut game, 60);
    let before = game.debug_driftwood();
    let again = Game::debug_continue(&game.debug_save_json()).expect("the save loads");
    assert_eq!(again.debug_driftwood(), before);
}

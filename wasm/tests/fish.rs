//! Scenario: fish swim in the lake on the high ground as well as the sea,
//! and those caught there come back there.

mod common;

use common::{hold, new_game};

#[test]
fn fish_swim_in_the_lake_from_the_start() {
    let game = new_game();
    assert!(game.debug_fish_in_lake() > 0);
    assert!(game.debug_fish_count() > game.debug_fish_in_lake(), "and in the sea and the river too");
}

#[test]
fn a_fish_caught_from_the_lake_comes_back_to_the_lake() {
    let mut game = new_game();
    let (lake, all) = (game.debug_fish_in_lake(), game.debug_fish_count());
    assert!(game.debug_catch_fish_in_lake(), "a fish is caught in the lake");
    assert_eq!(game.debug_fish_in_lake(), lake - 1);
    game.debug_advance_hours(4);
    hold(&mut game, 0, 2);
    assert_eq!(game.debug_fish_in_lake(), lake, "it came back to the lake");
    assert_eq!(game.debug_fish_count(), all, "and only to the lake");
}

//! Scenario: coming to the river's narrow from the first area, the
//! castaway is struck by an idea, walks to the bank where a log bridge
//! would go across and stops to think (once).

mod common;

use common::{hold, new_game};
use survival_core::common::input::Buttons;
use survival_core::game::Game;
use survival_core::items::ItemKind;

/// The east bank cell a log bridge would rest on, at the narrow's middle
/// row.
fn bridge_bank(game: &Game) -> (i32, i32) {
    game.map().bridge_bank().expect("a bridge would go across the narrow")
}

/// A game with the castaway a few cells east of the narrow's bank, not
/// yet having come near it.
fn short_of_the_narrow() -> Game {
    let mut game = new_game();
    let (x, y) = bridge_bank(&game);
    game.debug_place_player((x + 6, y));
    game
}

/// Walks west toward the narrow until the castaway has come near it.
fn walk_up_to_the_narrow(game: &mut Game) {
    hold(game, Buttons::LEFT, 40);
    hold(game, 0, 1);
}

/// Lets the castaway be for a while: whether they stopped to think.
fn wait(game: &mut Game) -> bool {
    let mut pondered = false;
    for _ in 0..600 {
        hold(game, 0, 1);
        pondered |= game.is_pondering();
    }
    pondered
}

#[test]
fn coming_to_the_narrow_the_castaway_walks_to_the_bank_and_stops_to_think() {
    let mut game = short_of_the_narrow();
    walk_up_to_the_narrow(&mut game);
    assert!(wait(&mut game), "the castaway stops to think");
    assert_eq!(game.player_tile(), bridge_bank(&game), "on the bank where the bridge would go");
}

#[test]
fn coming_to_the_narrow_an_idea_strikes_first() {
    let mut game = short_of_the_narrow();
    walk_up_to_the_narrow(&mut game);
    let mut struck = false;
    for _ in 0..30 {
        hold(&mut game, 0, 1);
        struck |= game.is_struck_by_an_idea();
    }
    assert!(struck);
}

#[test]
fn holding_a_log_bridge_the_castaway_walks_to_the_bank_without_stopping_to_think() {
    let mut game = short_of_the_narrow();
    game.debug_give(ItemKind::LogBridge);
    walk_up_to_the_narrow(&mut game);
    assert!(!wait(&mut game));
    assert_eq!(game.player_tile(), bridge_bank(&game));
}

#[test]
fn from_the_bank_the_bridge_goes_across_with_b() {
    let mut game = short_of_the_narrow();
    game.debug_give(ItemKind::LogBridge);
    walk_up_to_the_narrow(&mut game);
    wait(&mut game);
    common::press(&mut game, Buttons::B);
    hold(&mut game, 0, 120);
    assert!(game.map().bridge_bank().is_none(), "the bridge is laid");
}

#[test]
fn the_castaway_comes_to_the_narrow_that_way_only_once() {
    let mut game = short_of_the_narrow();
    walk_up_to_the_narrow(&mut game);
    wait(&mut game);
    let (x, y) = bridge_bank(&game);
    game.debug_place_player((x + 6, y));
    walk_up_to_the_narrow(&mut game);
    assert!(!wait(&mut game));
}

#[test]
fn having_come_to_the_narrow_is_kept_in_the_save() {
    let mut game = short_of_the_narrow();
    walk_up_to_the_narrow(&mut game);
    wait(&mut game);
    let mut again = Game::debug_continue(&game.debug_save_json()).expect("the save loads");
    let (x, y) = bridge_bank(&again);
    again.debug_place_player((x + 6, y));
    walk_up_to_the_narrow(&mut again);
    assert!(!wait(&mut again));
}

#[test]
fn once_the_bridge_is_laid_there_is_no_coming_to_the_narrow() {
    let mut game = short_of_the_narrow();
    game.debug_lay_bridge();
    walk_up_to_the_narrow(&mut game);
    assert!(!wait(&mut game));
}

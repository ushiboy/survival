//! Scenario: a game saved and carried on is the game as it was.

mod common;

use common::{SEED, new_game, press, walk_about};
use survival_core::common::input::Buttons;
use survival_core::game::Game;
use survival_core::items::ItemKind;

#[test]
fn the_same_play_on_the_same_island_ends_the_same() {
    let play = || {
        let mut game = new_game();
        let start = game.player_tile();
        walk_about(&mut game);
        assert_ne!(game.player_tile(), start, "the castaway walked");
        game.debug_save_json()
    };
    assert_eq!(play(), play());
}

#[test]
fn a_saved_game_carries_on_as_it_was() {
    let mut game = new_game();
    game.debug_give_n(ItemKind::Stone, 2);
    walk_about(&mut game);
    let saved = game.debug_save_json();

    let carried_on = Game::debug_continue(&saved).expect("the save loads");
    assert_eq!(carried_on.debug_save_json(), saved);
}

#[test]
fn continue_on_the_title_carries_on_from_the_save() {
    let mut game = new_game();
    walk_about(&mut game);
    let saved = game.debug_save_json();

    // The cursor starts on CONTINUE when there is a saved game.
    let mut next = Game::with_save(SEED, Some(&saved));
    press(&mut next, Buttons::A);
    assert_eq!(next.debug_save_json(), saved);
    assert_eq!(next.player_tile(), game.player_tile());
}

#[test]
fn a_game_carried_on_plays_on_like_the_one_saved() {
    let mut game = new_game();
    walk_about(&mut game);
    let mut carried_on = Game::debug_continue(&game.debug_save_json()).unwrap();
    walk_about(&mut game);
    walk_about(&mut carried_on);
    assert_eq!(carried_on.player_tile(), game.player_tile());
}


/// Chunks awake when the game was saved are awake when it is carried on,
/// even those beyond where chunks wake but not yet far enough behind to
/// fall asleep: their creatures keep moving.
#[test]
fn chunks_awake_when_saved_are_awake_on_carrying_on() {
    let mut game = new_game();
    walk_about(&mut game);
    let mut carried_on = Game::debug_continue(&game.debug_save_json()).unwrap();
    common::hold(&mut game, 0, 1);
    common::hold(&mut carried_on, 0, 1);
    let sleep = |g: &Game| serde_json::from_str::<serde_json::Value>(&g.debug_save_json()).unwrap()["island"]["asleep_since"].clone();
    assert_eq!(sleep(&carried_on), sleep(&game));
}

#[test]
fn a_bridge_laid_across_the_river_is_still_there_on_carrying_on() {
    let mut game = new_game();
    assert!(game.debug_lay_bridge(), "a bridge goes across the narrow");
    let carried_on = Game::debug_continue(&game.debug_save_json()).expect("the save loads");
    let (x, y) = carried_on.map().spots.bridge.expect("the river has a narrow");
    assert!(matches!(carried_on.map().feature(x, y), survival_core::world::Feature::Bridge(_)));
}

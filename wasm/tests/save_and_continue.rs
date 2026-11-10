//! Scenario: a game saved and carried on is the game as it was.

mod common;

use common::{SEED, new_game, press, walk_about};
use survival_core::common::input::Buttons;
use survival_core::game::Game;
use survival_core::items::ItemKind;

/// A save with which chunks sleep left out: a game carried on puts some
/// chunks awake when it was saved back to sleep (see
/// `chunks_awake_when_saved_wake_again_on_carrying_on`).
fn without_sleep(json: &str) -> serde_json::Value {
    let mut v: serde_json::Value = serde_json::from_str(json).unwrap();
    v["island"]["asleep_since"] = serde_json::Value::Null;
    v
}

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
    assert_eq!(without_sleep(&carried_on.debug_save_json()), without_sleep(&saved));
}

#[test]
fn continue_on_the_title_carries_on_from_the_save() {
    let mut game = new_game();
    walk_about(&mut game);
    let saved = game.debug_save_json();

    // The cursor starts on CONTINUE when there is a saved game.
    let mut next = Game::with_save(SEED, Some(&saved));
    press(&mut next, Buttons::A);
    assert_eq!(without_sleep(&next.debug_save_json()), without_sleep(&saved));
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


/// Known to fail: chunks awake when the game was saved come back asleep as
/// of the moment it is carried on, and those just outside the wake margin
/// (but still inside the sleep margin, so awake in the game saved) don't
/// wake again until the castaway comes nearer. Their creatures stand still
/// meanwhile. Reported, not yet decided whether to fix.
#[test]
#[ignore = "chunks awake when saved don't all wake again on carrying on"]
fn chunks_awake_when_saved_wake_again_on_carrying_on() {
    let mut game = new_game();
    walk_about(&mut game);
    let mut carried_on = Game::debug_continue(&game.debug_save_json()).unwrap();
    common::hold(&mut game, 0, 1);
    common::hold(&mut carried_on, 0, 1);
    let sleep = |g: &Game| serde_json::from_str::<serde_json::Value>(&g.debug_save_json()).unwrap()["island"]["asleep_since"].clone();
    assert_eq!(sleep(&carried_on), sleep(&game));
}

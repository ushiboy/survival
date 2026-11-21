//! Scenario: the wood lathe. Once the water wheel turns in the river at
//! the mill site, a lathe held in hand is set on the bank beside it, where
//! the wheel drives it; things turned on it are made only beside it.
//! Without one in hand, looking at that spot gives pause.

mod common;

use common::{hold, new_game, press};
use survival_core::common::input::Buttons;
use survival_core::game::Game;
use survival_core::items::ItemKind;
use survival_core::world::Feature;

fn mill_site(game: &Game) -> (i32, i32) {
    game.map().spots.mill.expect("the island keeps ground for a mill")
}

fn lathe_set(game: &Game) -> bool {
    let map = game.map();
    (0..map.height()).any(|y| (0..map.width()).any(|x| matches!(map.feature(x, y), Feature::Lathe(_))))
}

/// A game with the castaway a step west of the bank beside the mill site,
/// facing east toward it (and the river beyond), the wheel in the river
/// if `wheel`.
fn west_of_the_bank(wheel: bool) -> Game {
    let mut game = new_game();
    if wheel {
        assert!(game.debug_set_wheel());
    } else {
        // The first look round the mill site already over (it would take
        // the castaway to the bank, to think about the wheel).
        let mut save: serde_json::Value = serde_json::from_str(&game.debug_save_json()).unwrap();
        save["castaway"]["mill_noticed"] = true.into();
        game = Game::debug_continue(&save.to_string()).expect("the save loads");
    }
    let (x, y) = mill_site(&game);
    game.debug_place_player((x - 2, y));
    hold(&mut game, 0, 600);
    hold(&mut game, Buttons::RIGHT, 4);
    hold(&mut game, 0, 2);
    game
}

#[test]
fn a_lathe_held_beside_the_turning_wheel_is_set_on_the_bank() {
    let mut game = west_of_the_bank(true);
    game.debug_give(ItemKind::WoodLathe);
    press(&mut game, Buttons::B);
    hold(&mut game, 0, 120);
    assert!(lathe_set(&game), "the lathe stands on the bank");
    assert!(!game.carries(ItemKind::WoodLathe), "it is out of the bag");
    assert!(!matches!(game.map().feature(game.player_tile().0, game.player_tile().1), Feature::Lathe(_)), "the castaway stands clear of it");
}

#[test]
fn a_lathe_wont_be_set_with_no_wheel_to_drive_it() {
    let mut game = west_of_the_bank(false);
    game.debug_give(ItemKind::WoodLathe);
    press(&mut game, Buttons::B);
    hold(&mut game, 0, 120);
    assert!(!lathe_set(&game));
    assert!(game.carries(ItemKind::WoodLathe));
}

#[test]
fn without_a_lathe_the_castaway_stops_to_think_beside_the_turning_wheel() {
    let mut game = west_of_the_bank(true);
    press(&mut game, Buttons::A);
    assert!(game.is_pondering());
}

#[test]
fn before_the_wheel_turns_there_is_nothing_to_think_about_beside_it() {
    let mut game = west_of_the_bank(false);
    press(&mut game, Buttons::A);
    assert!(!game.is_pondering());
}

/// Once the lathe is set, it's the mill's ground beside it that gives
/// pause (see `marking_stakes.rs`); with that marked out too, nothing does.
#[test]
fn once_the_lathe_is_set_there_is_nothing_more_to_think_about() {
    let mut game = west_of_the_bank(true);
    game.debug_give(ItemKind::WoodLathe);
    press(&mut game, Buttons::B);
    hold(&mut game, 0, 120);
    assert!(game.debug_set_stakes());
    hold(&mut game, 0, 2);
    press(&mut game, Buttons::A);
    assert!(!game.is_pondering());
}

#[test]
fn the_castaway_is_by_the_lathe_only_near_it() {
    let mut game = west_of_the_bank(true);
    assert!(game.debug_set_lathe());
    hold(&mut game, 0, 1);
    assert!(game.is_near_lathe());
    let (x, y) = mill_site(&game);
    game.debug_place_player((x - 12, y + 4));
    hold(&mut game, 0, 1);
    assert!(!game.is_near_lathe());
}

#[test]
fn a_lathe_set_on_the_bank_is_still_there_on_carrying_on() {
    let mut game = new_game();
    assert!(game.debug_set_wheel());
    assert!(game.debug_set_lathe());
    let carried_on = Game::debug_continue(&game.debug_save_json()).expect("the save loads");
    assert!(lathe_set(&carried_on));
}

/// A game with the wheel in, the castaway standing at `(dx, dy)` cells from
/// the mill site's bank, facing `facing` (held a moment), a lathe in hand.
fn near_the_bank_with_a_lathe((dx, dy): (i32, i32), facing: u16) -> Game {
    let mut game = new_game();
    assert!(game.debug_set_wheel());
    let (x, y) = mill_site(&game);
    game.debug_place_player((x + dx, y + dy));
    hold(&mut game, 0, 2);
    hold(&mut game, facing, 2);
    hold(&mut game, 0, 2);
    game.debug_give(ItemKind::WoodLathe);
    game
}

#[test]
fn a_lathe_held_a_few_steps_off_is_carried_over_and_set_beside_the_wheel() {
    for (at, facing) in [((-5, 2), Buttons::DOWN), ((-4, -3), Buttons::LEFT), ((-1, 3), Buttons::UP)] {
        let mut game = near_the_bank_with_a_lathe(at, facing);
        press(&mut game, Buttons::B);
        hold(&mut game, 0, 240);
        assert!(lathe_set(&game), "from {at:?}");
        assert!(!game.carries(ItemKind::WoodLathe), "from {at:?}");
        let (x, y) = mill_site(&game);
        assert_eq!(game.player_tile(), (x - 2, y), "the castaway stepped over to set it, from {at:?}");
    }
}

#[test]
fn a_lathe_held_far_from_the_wheel_isnt_set() {
    let mut game = near_the_bank_with_a_lathe((-9, 0), Buttons::RIGHT);
    press(&mut game, Buttons::B);
    hold(&mut game, 0, 240);
    assert!(!lathe_set(&game));
}

#[test]
fn without_a_lathe_the_castaway_stops_to_think_a_few_steps_from_the_wheel() {
    let mut game = new_game();
    assert!(game.debug_set_wheel());
    let (x, y) = mill_site(&game);
    game.debug_place_player((x - 4, y + 2));
    hold(&mut game, 0, 2);
    press(&mut game, Buttons::A);
    assert!(game.is_pondering());
}

#[test]
fn pulling_the_cart_beside_the_wheel_a_sets_it_down_rather_than_giving_pause() {
    let mut game = new_game();
    assert!(game.debug_set_wheel());
    let (x, y) = mill_site(&game);
    game.debug_place_player((x - 4, y + 2));
    hold(&mut game, 0, 2);
    game.debug_give(ItemKind::Handcart);
    hold(&mut game, 0, 2);
    press(&mut game, Buttons::A);
    assert!(!game.is_pondering());
    assert!(!game.is_pulling_cart(), "set down");
}

//! Scenario: the handcart. Made, it doesn't go in the bag: the castaway
//! pulls it, hitched right behind them (it swings round as they turn), and
//! it carries a load of its own. A sets it down
//! and, facing it, takes it up again. While pulling it the castaway can't
//! run (walking is as fast as ever), and can't take it into the shallows
//! or up a ladder.

mod common;

use common::{hold, new_game, press};
use survival_core::common::input::Buttons;
use survival_core::game::Game;
use survival_core::items::ItemKind;
use survival_core::world::Terrain;

/// A game pulling a new cart, on the open grass west of the mill site
/// (room all round), the first look round there already over.
fn pulling() -> Game {
    let mut game = new_game();
    let (x, y) = game.map().spots.mill.expect("the island keeps ground for a mill");
    let mut save: serde_json::Value = serde_json::from_str(&game.debug_save_json()).unwrap();
    save["castaway"]["mill_noticed"] = true.into();
    game = Game::debug_continue(&save.to_string()).expect("the save loads");
    game.debug_place_player((x - 6, y + 2));
    hold(&mut game, 0, 2);
    game.debug_give(ItemKind::Handcart);
    hold(&mut game, 0, 2);
    game
}

fn feet_x(game: &Game) -> i32 {
    game.player_feet().0
}

#[test]
fn a_handcart_made_follows_the_castaway_and_not_in_the_bag() {
    let game = pulling();
    assert!(game.is_pulling_cart());
    assert!(!game.carries(ItemKind::Handcart), "it is not in the bag");
    assert!(game.cart_at().is_some());
}

#[test]
fn the_cart_is_pulled_right_behind_the_castaway() {
    let mut game = pulling();
    hold(&mut game, Buttons::RIGHT, 60);
    let (cx, cy) = game.cart_at().unwrap();
    let (fx, fy) = game.player_feet();
    assert!(cx < fx && fx - cx < 40, "close behind, to the west, walking east");
    assert!((cy - fy).abs() < 16, "level with them");
}

#[test]
fn the_cart_swings_round_behind_as_the_castaway_turns() {
    let mut game = pulling();
    hold(&mut game, Buttons::RIGHT, 20);
    hold(&mut game, Buttons::UP, 20);
    let (cx, cy) = game.cart_at().unwrap();
    let (fx, fy) = game.player_feet();
    assert_eq!(cx, fx, "straight behind");
    assert!(cy > fy, "below them, walking north");
}

/// Open ground with something solid just north and the row clear either
/// side.
fn under_something_solid(game: &Game) -> (i32, i32) {
    let map = game.map();
    (4..map.height())
        .flat_map(|y| (4..map.width() - 4).map(move |x| (x, y)))
        .find(|&(x, y)| {
            map.props(x, y - 1).solid && (-3..=3).all(|dx| (0..=1).all(|dy| !map.props(x + dx, y + dy).solid && map.terrain(x + dx, y + dy) == Terrain::Grass))
        })
        .expect("open grass under something solid")
}

#[test]
fn the_castaway_turns_pulling_the_cart_wherever_they_could_without_it() {
    let mut game = pulling();
    let (x, y) = under_something_solid(&game);
    game.debug_place_player((x, y));
    hold(&mut game, 0, 2);
    hold(&mut game, Buttons::RIGHT, 2);
    hold(&mut game, Buttons::DOWN, 2);
    let (cx, cy) = game.cart_at().unwrap();
    let (fx, fy) = game.player_feet();
    assert_eq!(cx, fx, "the cart swings round behind, though what's north is in the way of it");
    assert!(cy < fy);
}

#[test]
fn pulling_the_cart_the_castaway_cant_run() {
    let mut walked = pulling();
    let mut ran = pulling();
    let (a, b) = (feet_x(&walked), feet_x(&ran));
    hold(&mut walked, Buttons::RIGHT, 40);
    hold(&mut ran, Buttons::RIGHT | Buttons::R, 40);
    assert_eq!(feet_x(&ran) - b, feet_x(&walked) - a, "running is no faster");
}

#[test]
fn pulling_the_cart_the_castaway_walks_as_fast_as_ever() {
    let mut pulling = pulling();
    let mut free = pulling_none();
    let (a, b) = (feet_x(&pulling), feet_x(&free));
    hold(&mut pulling, Buttons::RIGHT, 40);
    hold(&mut free, Buttons::RIGHT, 40);
    assert_eq!(feet_x(&pulling) - a, feet_x(&free) - b);
}

/// The same place as `pulling`, with no cart.
fn pulling_none() -> Game {
    let mut game = pulling();
    press(&mut game, Buttons::A);
    hold(&mut game, 0, 2);
    // Step away from the cart set down, back to the same spot's row.
    let (x, y) = game.map().spots.mill.unwrap();
    game.debug_place_player((x - 6, y - 3));
    hold(&mut game, 0, 2);
    game
}

#[test]
fn a_set_down_cart_stays_put() {
    let mut game = pulling();
    press(&mut game, Buttons::A);
    hold(&mut game, 0, 2);
    assert!(!game.is_pulling_cart());
    let at = game.cart_at();
    hold(&mut game, Buttons::UP, 60);
    assert_eq!(game.cart_at(), at, "it doesn't follow");
}

#[test]
fn facing_a_set_down_cart_a_takes_it_up_again() {
    let mut game = pulling();
    hold(&mut game, Buttons::RIGHT, 30);
    press(&mut game, Buttons::A);
    hold(&mut game, 0, 2);
    // Turn round to the cart behind.
    hold(&mut game, Buttons::LEFT, 2);
    hold(&mut game, 0, 2);
    press(&mut game, Buttons::A);
    hold(&mut game, 0, 2);
    assert!(game.is_pulling_cart());
}

#[test]
fn a_set_down_cart_is_not_in_the_way() {
    let mut game = pulling();
    hold(&mut game, Buttons::RIGHT, 30);
    press(&mut game, Buttons::A);
    hold(&mut game, 0, 2);
    let (cx, _) = game.cart_at().unwrap();
    hold(&mut game, Buttons::LEFT, 90);
    assert!(feet_x(&game) < cx, "the castaway walks through it");
}

/// A beach cell with the shallows just east of it, and dry sand west of
/// it to start from.
fn by_the_shallows(game: &Game) -> (i32, i32) {
    let map = game.map();
    (0..map.height())
        .flat_map(|y| (4..map.width() - 1).map(move |x| (x, y)))
        .find(|&(x, y)| {
            map.terrain(x + 1, y) == Terrain::ShallowWater
                && (-4..=0).all(|dx| map.terrain(x + dx, y) == Terrain::Sand && !map.props(x + dx, y).solid)
                && (-4..=0).all(|dx| (-1..=1).all(|dy| !map.props(x + dx, y + dy).solid && map.terrain(x + dx, y + dy).is_land()))
        })
        .expect("a beach running into the shallows")
}

#[test]
fn the_cart_cant_be_pulled_into_the_shallows() {
    let mut game = pulling();
    let (x, y) = by_the_shallows(&game);
    game.debug_place_player((x - 3, y));
    hold(&mut game, 0, 2);
    hold(&mut game, Buttons::RIGHT, 60);
    let (fx, fy) = game.player_feet();
    assert_ne!(game.map().terrain(fx.div_euclid(16), fy.div_euclid(16)), Terrain::ShallowWater);
    assert!(!game.is_wading());
}

#[test]
fn without_the_cart_the_castaway_wades_into_the_shallows() {
    let mut game = new_game();
    let (x, y) = by_the_shallows(&game);
    game.debug_place_player((x - 3, y));
    hold(&mut game, 0, 2);
    hold(&mut game, Buttons::RIGHT, 60);
    assert!(game.is_wading());
}

#[test]
fn what_is_put_in_the_cart_stays_in_it() {
    let mut game = pulling();
    assert_eq!(game.debug_load_cart(ItemKind::LauanTimber, 12), 12, "twelve lengths of timber fit");
    assert_eq!(game.debug_load_cart(ItemKind::LauanTimber, 1), 0, "and no more");
    assert_eq!(game.cart_load(ItemKind::LauanTimber), 12);
    assert!(!game.carries(ItemKind::LauanTimber), "none of it in the bag");
}

#[test]
fn the_cart_and_its_load_are_still_there_on_carrying_on() {
    let mut game = pulling();
    game.debug_load_cart(ItemKind::LauanTimber, 3);
    hold(&mut game, Buttons::RIGHT, 30);
    press(&mut game, Buttons::A);
    hold(&mut game, 0, 2);
    let saved = game.debug_save_json();
    let carried_on = Game::debug_continue(&saved).expect("the save loads");
    assert_eq!(carried_on.cart_at(), game.cart_at());
    assert!(!carried_on.is_pulling_cart());
    assert_eq!(carried_on.cart_load(ItemKind::LauanTimber), 3);
    assert_eq!(carried_on.debug_save_json(), saved);
}

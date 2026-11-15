//! Scenario: the water wheel goes into the river at the mill site on the
//! west bank below the pool; coming there first, the castaway looks round,
//! walks to that bank and stops to think.

mod common;

use common::{hold, new_game, press};
use survival_core::common::input::Buttons;
use survival_core::game::Game;
use survival_core::items::ItemKind;
use survival_core::world::{Feature, Terrain};

/// The bank the water wheel goes in from (a cell), on the island the tests
/// play on.
fn mill_site(game: &Game) -> (i32, i32) {
    game.map().spots.mill.expect("the island keeps ground for a mill")
}

fn wheel_in_river(game: &Game) -> bool {
    let map = game.map();
    (0..map.height()).any(|y| (0..map.width()).any(|x| matches!(map.feature(x, y), Feature::WaterWheel(_))))
}

/// A game with the castaway standing on the mill site's bank, the first
/// look round there already over.
fn at_the_bank() -> Game {
    let mut game = new_game();
    game.debug_place_player(mill_site(&game));
    hold(&mut game, 0, 600);
    game
}

/// Turns to the river (east, held into it for a moment).
fn face_the_river(game: &mut Game) {
    hold(game, Buttons::RIGHT, 4);
    hold(game, 0, 2);
}

#[test]
fn a_water_wheel_held_on_the_bank_goes_into_the_river() {
    let mut game = at_the_bank();
    game.debug_give(ItemKind::WaterWheel);
    face_the_river(&mut game);
    press(&mut game, Buttons::B);
    hold(&mut game, 0, 120);
    assert!(wheel_in_river(&game), "the wheel stands in the river");
    assert!(!game.carries(ItemKind::WaterWheel), "it is out of the bag");
}

#[test]
fn a_water_wheel_held_a_few_rows_off_still_goes_in_at_the_mill_site() {
    let mut game = at_the_bank();
    let (x, y) = mill_site(&game);
    game.debug_place_player((x, y + 2));
    game.debug_give(ItemKind::WaterWheel);
    face_the_river(&mut game);
    press(&mut game, Buttons::B);
    hold(&mut game, 0, 240);
    assert!(wheel_in_river(&game));
    assert_eq!(game.player_tile().1, y, "the castaway steps along the bank to the site first");
}

#[test]
fn a_water_wheel_wont_go_into_the_river_away_from_the_mill_site() {
    let mut game = new_game();
    let narrow = game.map().spots.bridge.expect("the river has a narrow");
    game.debug_lay_bridge();
    game.debug_place_player((narrow.0 - 2, narrow.1 + 6));
    game.debug_give(ItemKind::WaterWheel);
    face_the_river(&mut game);
    press(&mut game, Buttons::B);
    hold(&mut game, 0, 240);
    assert!(!wheel_in_river(&game));
    assert!(game.carries(ItemKind::WaterWheel));
}

#[test]
fn without_a_water_wheel_the_castaway_stops_to_think_at_the_mill_site() {
    let mut game = at_the_bank();
    face_the_river(&mut game);
    press(&mut game, Buttons::A);
    assert!(game.is_pondering());
}

#[test]
fn once_the_water_wheel_is_in_there_is_nothing_more_to_think_about() {
    let mut game = at_the_bank();
    game.debug_give(ItemKind::WaterWheel);
    face_the_river(&mut game);
    press(&mut game, Buttons::B);
    hold(&mut game, 0, 120);
    // With a lathe in hand (there's then nothing to think about the lathe
    // either: the wheel turning, the bank beside it is where it goes).
    game.debug_give(ItemKind::WoodLathe);
    press(&mut game, Buttons::A);
    assert!(!game.is_pondering());
}

/// A game with the castaway a few cells west of the mill site, not yet
/// having come near it.
fn short_of_the_mill_site() -> Game {
    let mut game = new_game();
    let (x, y) = mill_site(&game);
    game.debug_place_player((x - 8, y));
    game
}

/// Walks east toward the mill site until the castaway has come near it,
/// then lets them be.
fn walk_up_to_the_mill_site(game: &mut Game) {
    hold(game, Buttons::RIGHT, 70);
    hold(game, 0, 1);
}

#[test]
fn coming_to_the_mill_site_the_castaway_walks_to_the_bank_and_stops_to_think() {
    let mut game = short_of_the_mill_site();
    walk_up_to_the_mill_site(&mut game);
    let mut pondered = false;
    for _ in 0..600 {
        hold(&mut game, 0, 1);
        pondered |= game.is_pondering();
    }
    assert!(pondered, "the castaway stops to think");
    assert_eq!(game.player_tile(), mill_site(&game), "on the bank where the wheel would go");
}

#[test]
fn holding_a_water_wheel_the_castaway_walks_to_the_bank_without_stopping_to_think() {
    let mut game = short_of_the_mill_site();
    game.debug_give(ItemKind::WaterWheel);
    walk_up_to_the_mill_site(&mut game);
    let mut pondered = false;
    for _ in 0..600 {
        hold(&mut game, 0, 1);
        pondered |= game.is_pondering();
    }
    assert!(!pondered);
    assert_eq!(game.player_tile(), mill_site(&game));
}

#[test]
fn the_castaway_comes_to_the_mill_site_that_way_only_once() {
    let mut game = short_of_the_mill_site();
    walk_up_to_the_mill_site(&mut game);
    hold(&mut game, 0, 600);
    let (x, y) = mill_site(&game);
    game.debug_place_player((x - 8, y));
    walk_up_to_the_mill_site(&mut game);
    hold(&mut game, 0, 600);
    assert_ne!(game.player_tile(), mill_site(&game), "no walk to the bank this time");
}

#[test]
fn having_come_to_the_mill_site_is_kept_in_the_save() {
    let mut game = short_of_the_mill_site();
    walk_up_to_the_mill_site(&mut game);
    hold(&mut game, 0, 600);
    let mut again = Game::debug_continue(&game.debug_save_json()).expect("the save loads");
    let (x, y) = mill_site(&again);
    again.debug_place_player((x - 8, y));
    walk_up_to_the_mill_site(&mut again);
    hold(&mut again, 0, 600);
    assert_ne!(again.player_tile(), mill_site(&again));
}

#[test]
fn a_water_wheel_in_the_river_is_kept_in_the_save() {
    let mut game = at_the_bank();
    game.debug_give(ItemKind::WaterWheel);
    face_the_river(&mut game);
    press(&mut game, Buttons::B);
    hold(&mut game, 0, 120);
    let again = Game::debug_continue(&game.debug_save_json()).expect("the save loads");
    assert!(wheel_in_river(&again));
}

#[test]
fn coming_near_the_mill_site_from_across_the_river_the_castaway_takes_no_notice() {
    let mut game = new_game();
    let (x, y) = mill_site(&game);
    // The east bank, across the river from the site (the first area).
    let east = (x + 1..x + 8).find(|&cx| game.map().terrain(cx, y) != Terrain::River).expect("the river has an east bank");
    game.debug_place_player((east + 1, y));
    hold(&mut game, Buttons::LEFT, 20);
    hold(&mut game, 0, 400);
    assert!(game.player_tile().0 > x, "still on the east bank");
    let mut again = Game::debug_continue(&game.debug_save_json()).expect("the save loads");
    again.debug_place_player((x - 8, y));
    walk_up_to_the_mill_site(&mut again);
    hold(&mut again, 0, 600);
    assert_eq!(again.player_tile(), mill_site(&again), "the event still waits for the west bank");
}

#[test]
fn coming_to_the_mill_site_an_idea_strikes_before_the_castaway_walks_over() {
    let mut game = short_of_the_mill_site();
    walk_up_to_the_mill_site(&mut game);
    let mut struck = false;
    for _ in 0..30 {
        hold(&mut game, 0, 1);
        struck |= game.is_struck_by_an_idea();
    }
    assert!(struck, "a \"!\" over the head first");
}

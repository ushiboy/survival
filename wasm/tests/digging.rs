//! Scenario: digging. Soil comes out of bare earth in the grass with a
//! digging stick, as often as it's dug; clay comes out of the beds on the
//! lake's shore with a bamboo spade, once a day each, the lake's water
//! seeping back overnight. The wrong tool, or none, only gives pause.

mod common;

use common::{hold, new_game, press};
use survival_core::common::input::Buttons;
use survival_core::game::Game;
use survival_core::items::ItemKind;
use survival_core::world::{Feature, WorldMap};

/// The top-left cell of the first patch (in reading order) `is` picks out.
fn first_patch(map: &WorldMap, is: impl Fn(Feature) -> bool) -> (i32, i32) {
    (0..map.height()).flat_map(|y| (0..map.width()).map(move |x| (x, y))).find(|&(x, y)| is(map.feature(x, y))).expect("the island has one")
}

fn clay_bed(game: &Game) -> (i32, i32) {
    first_patch(game.map(), |f| matches!(f, Feature::ClayBed(c) if c.part == 0))
}

fn bare_earth(game: &Game) -> (i32, i32) {
    first_patch(game.map(), |f| f == Feature::BareEarth(0))
}

/// A game with the castaway standing on the patch at `cell`, holding
/// `tool` (if any).
fn standing_on(at: fn(&Game) -> (i32, i32), tool: Option<ItemKind>) -> Game {
    let mut game = new_game();
    let cell = at(&game);
    game.debug_place_player(cell);
    if let Some(tool) = tool {
        game.debug_give(tool);
    }
    hold(&mut game, 0, 2);
    game
}

/// Digs (B) and waits for the dig to finish.
fn dig(game: &mut Game) {
    press(game, Buttons::B);
    hold(game, 0, 90);
}

fn next_morning(game: &mut Game) {
    game.debug_advance_hours(24);
    hold(game, 0, 1);
}

#[test]
fn a_digging_stick_digs_soil_out_of_bare_earth() {
    let mut game = standing_on(bare_earth, Some(ItemKind::DiggingStick));
    dig(&mut game);
    assert_eq!(game.carried(ItemKind::Soil), 1);
}

#[test]
fn bare_earth_gives_soil_as_often_as_it_is_dug() {
    let mut game = standing_on(bare_earth, Some(ItemKind::DiggingStick));
    dig(&mut game);
    dig(&mut game);
    dig(&mut game);
    assert_eq!(game.carried(ItemKind::Soil), 3);
}

#[test]
fn a_bamboo_spade_digs_clay_out_of_a_clay_bed() {
    let mut game = standing_on(clay_bed, Some(ItemKind::BambooSpade));
    dig(&mut game);
    assert!(game.carried(ItemKind::Clay) > 1, "a spadeful is more than a lump");
}

#[test]
fn a_clay_bed_gives_clay_once_a_day() {
    let mut game = standing_on(clay_bed, Some(ItemKind::BambooSpade));
    dig(&mut game);
    let once = game.carried(ItemKind::Clay);
    dig(&mut game);
    assert_eq!(game.carried(ItemKind::Clay), once);
}

#[test]
fn a_dug_clay_bed_fills_again_by_the_next_morning() {
    let mut game = standing_on(clay_bed, Some(ItemKind::BambooSpade));
    dig(&mut game);
    let once = game.carried(ItemKind::Clay);
    next_morning(&mut game);
    dig(&mut game);
    assert!(game.carried(ItemKind::Clay) > once);
}

#[test]
fn a_digging_stick_wont_lift_clay_and_gives_pause() {
    let mut game = standing_on(clay_bed, Some(ItemKind::DiggingStick));
    press(&mut game, Buttons::B);
    assert!(game.is_pondering());
    hold(&mut game, 0, 90);
    assert!(!game.carries(ItemKind::Clay));
}

#[test]
fn a_bamboo_spade_wont_bite_into_hard_earth_and_gives_pause() {
    let mut game = standing_on(bare_earth, Some(ItemKind::BambooSpade));
    press(&mut game, Buttons::B);
    assert!(game.is_pondering());
    hold(&mut game, 0, 90);
    assert!(!game.carries(ItemKind::Soil));
}

#[test]
fn without_a_digging_stick_the_castaway_stops_to_think_at_bare_earth() {
    let mut game = standing_on(bare_earth, None);
    press(&mut game, Buttons::A);
    assert!(game.is_pondering());
}

#[test]
fn without_a_spade_the_castaway_stops_to_think_at_a_clay_bed() {
    let mut game = standing_on(clay_bed, None);
    press(&mut game, Buttons::A);
    assert!(game.is_pondering());
}

#[test]
fn a_clay_bed_dug_today_gives_no_pause() {
    let mut game = standing_on(clay_bed, Some(ItemKind::BambooSpade));
    dig(&mut game);
    press(&mut game, Buttons::A);
    assert!(!game.is_pondering(), "there's nothing more to be had there today");
}

#[test]
fn a_dug_clay_bed_is_kept_in_the_save() {
    let mut game = standing_on(clay_bed, Some(ItemKind::BambooSpade));
    dig(&mut game);
    let once = game.carried(ItemKind::Clay);
    let mut again = Game::debug_continue(&game.debug_save_json()).expect("the save loads");
    let cell = clay_bed(&again);
    again.debug_place_player(cell);
    hold(&mut again, 0, 2);
    dig(&mut again);
    assert_eq!(again.carried(ItemKind::Clay), once, "still dug out today");
}

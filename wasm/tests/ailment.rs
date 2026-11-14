//! Scenario: eating or drinking something raw may upset the castaway's
//! stomach; cooked food and boiled water never do.

mod common;

use common::{hold, new_game, press};
use survival_core::common::input::Buttons;
use survival_core::game::Game;
use survival_core::game::state::Ailment;
use survival_core::items::ItemKind;

/// Takes `kind` from the bag and eats or drinks it (the bag holding only
/// that).
fn consume(game: &mut Game, kind: ItemKind) {
    game.debug_give(kind);
    press(game, Buttons::X);
    press(game, Buttons::A);
    press(game, Buttons::A);
    hold(game, 0, 120);
}

fn upset(game: &Game) -> bool {
    game.vitals().has(Ailment::UpsetStomach)
}

/// A game where whatever can upset the stomach always does.
fn unlucky() -> Game {
    let mut game = new_game();
    game.debug_stomach_luck(Some(false));
    game
}

#[test]
fn raw_crab_and_fish_may_upset_the_stomach_as_soon_as_they_are_eaten() {
    for raw in [ItemKind::Crab, ItemKind::Fish] {
        let mut game = unlucky();
        game.debug_set_vitals(None, Some(10), None);
        let food = game.vitals().food;
        consume(&mut game, raw);
        assert!(game.vitals().food > food, "{raw:?} is eaten");
        assert!(upset(&game), "{raw:?}");
    }
}

#[test]
fn with_luck_raw_food_does_no_harm() {
    let mut game = new_game();
    game.debug_stomach_luck(Some(true));
    consume(&mut game, ItemKind::Crab);
    assert!(!upset(&game));
}

#[test]
fn the_first_raw_water_is_always_safe() {
    let mut game = unlucky();
    consume(&mut game, ItemKind::RawWater);
    assert!(!upset(&game), "the first draw of raw water");
    consume(&mut game, ItemKind::RawWater);
    assert!(upset(&game), "but not the next");
}

#[test]
fn cooked_food_boiled_water_and_coconut_never_upset_the_stomach() {
    for safe in [ItemKind::GrilledCrab, ItemKind::GrilledFish, ItemKind::BoiledWater, ItemKind::OpenedCoconut] {
        let mut game = unlucky();
        consume(&mut game, safe);
        assert!(!upset(&game), "{safe:?}");
    }
}

#[test]
fn some_raw_things_are_riskier_than_others() {
    use survival_core::items::ItemKind as K;
    let risk = |k: K| k.risk().map_or(0.0, |(_, chance)| chance);
    assert!(risk(K::RawWater) < risk(K::Fish));
    assert!(risk(K::Fish) < risk(K::Crab));
    assert!(risk(K::RawWater) > 0.0);
}

#[test]
fn with_an_upset_stomach_the_castaway_walks_instead_of_running() {
    let mut well = new_game();
    let mut ill = new_game();
    ill.debug_upset_stomach();
    let mut walker = new_game();
    hold(&mut well, Buttons::LEFT | Buttons::R, 60);
    hold(&mut ill, Buttons::LEFT | Buttons::R, 60);
    hold(&mut walker, Buttons::LEFT, 60);
    let start = new_game().player_tile();
    let gone = |g: &Game| (g.player_tile().0 - start.0).abs();
    assert!(gone(&well) > gone(&walker), "running goes further");
    assert_eq!(gone(&ill), gone(&walker));
}

#[test]
fn an_upset_stomach_is_kept_in_the_save() {
    let mut game = new_game();
    game.debug_upset_stomach();
    let continued = Game::debug_continue(&game.debug_save_json()).unwrap();
    assert!(upset(&continued));
}

#[test]
fn the_first_raw_water_is_remembered_in_the_save() {
    let mut game = unlucky();
    consume(&mut game, ItemKind::RawWater);
    let mut continued = Game::debug_continue(&game.debug_save_json()).unwrap();
    continued.debug_stomach_luck(Some(false));
    consume(&mut continued, ItemKind::RawWater);
    assert!(upset(&continued), "the bonus is already spent");
}

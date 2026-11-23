//! Scenario: a bamboo flask holds several cups of water. It's filled full
//! at a water spot, boiled whole over a fire with the bark pot, or topped
//! up from a pot of boiled water (freeing the pot); a cup is drunk at a
//! time, and the last leaves the flask empty.

mod common;

use common::{hold, new_game, press};
use survival_core::common::input::Buttons;
use survival_core::game::Game;
use survival_core::game::mill::Furnishing;
use survival_core::game::state::Ailment;
use survival_core::items::{FLASK_CUPS, ItemKind};

/// Cups of water in the bag's flasks of `kind`.
fn cups(game: &Game, kind: ItemKind) -> u64 {
    let save: serde_json::Value = serde_json::from_str(&game.debug_save_json()).unwrap();
    let name = serde_json::to_value(kind).unwrap();
    save["inventory"]["bag"]["items"].as_array().unwrap().iter().filter(|p| p["kind"] == name).map(|p| p["count"].as_u64().unwrap()).sum()
}

/// Opens the bag and picks command `n` (counted from the top) for the
/// first thing in it.
fn command(game: &mut Game, n: usize) {
    press(game, Buttons::X);
    press(game, Buttons::A);
    for _ in 0..n {
        press(game, Buttons::DOWN);
    }
    press(game, Buttons::A);
}

/// Drinks from the first thing in the bag (DRINK is its first command).
fn drink(game: &mut Game) {
    command(game, 0);
    hold(game, 0, 120);
}

/// The castaway at a water spot, having just pressed A facing the water
/// (turned each way until something was drawn).
fn draw_water(game: &mut Game) {
    let spot = game.debug_places().into_iter().find(|(name, _)| name.contains("水汲み場")).expect("a water spot").1;
    let before = game.debug_save_json();
    for dir in [Buttons::UP, Buttons::LEFT, Buttons::RIGHT, Buttons::DOWN] {
        game.debug_place_player(spot);
        hold(game, 0, 2);
        game.step(dir);
        hold(game, 0, 2);
        press(game, Buttons::A);
        hold(game, 0, 90);
        if game.debug_save_json() != before {
            return;
        }
    }
    panic!("no water drawn at {spot:?}");
}

#[test]
fn a_flask_is_filled_full_at_a_water_spot_without_using_a_leaf() {
    let mut game = new_game();
    game.debug_give(ItemKind::BambooFlask);
    game.debug_give_n(ItemKind::Leaf, 1);
    draw_water(&mut game);
    assert_eq!(cups(&game, ItemKind::RawWaterFlask), FLASK_CUPS as u64);
    assert_eq!(game.carried(ItemKind::BambooFlask), 0);
    assert_eq!(game.carried(ItemKind::Leaf), 1, "the leaf is kept");
    assert_eq!(game.carried(ItemKind::RawWater), 0);
}

#[test]
fn a_flask_part_full_of_raw_water_is_topped_up_at_a_water_spot() {
    let mut game = new_game();
    game.debug_give_n(ItemKind::RawWaterFlask, 1);
    draw_water(&mut game);
    assert_eq!(cups(&game, ItemKind::RawWaterFlask), FLASK_CUPS as u64);
    assert_eq!(game.carried(ItemKind::RawWaterFlask), 1);
}

#[test]
fn with_its_flasks_full_a_leaf_is_used_to_draw_water_as_before() {
    let mut game = new_game();
    game.debug_give(ItemKind::RawWaterFlask);
    game.debug_give_n(ItemKind::Leaf, 1);
    draw_water(&mut game);
    assert_eq!(game.carried(ItemKind::RawWater), 1);
    assert_eq!(game.carried(ItemKind::Leaf), 0);
}

#[test]
fn a_cup_at_a_time_is_drunk_from_a_flask_and_the_last_leaves_it_empty() {
    let mut game = new_game();
    game.debug_give(ItemKind::BoiledWaterFlask);
    game.debug_set_vitals(Some(10), None, Some(0));
    drink(&mut game);
    assert!(game.vitals().water > 0, "a cup quenches thirst");
    assert!(game.vitals().hp > 10, "boiled water heals");
    assert_eq!(cups(&game, ItemKind::BoiledWaterFlask), FLASK_CUPS as u64 - 1);
    for _ in 1..FLASK_CUPS {
        drink(&mut game);
    }
    assert_eq!(game.carried(ItemKind::BoiledWaterFlask), 0);
    assert_eq!(game.carried(ItemKind::BambooFlask), 1, "the flask is kept, empty");
}

#[test]
fn raw_water_from_a_flask_may_upset_the_stomach_like_any_raw_water() {
    let mut game = new_game();
    game.debug_stomach_luck(Some(false));
    game.debug_give(ItemKind::RawWaterFlask);
    drink(&mut game);
    assert!(!game.vitals().has(Ailment::UpsetStomach), "the first draw of raw water is always safe");
    drink(&mut game);
    assert!(game.vitals().has(Ailment::UpsetStomach));
}

#[test]
fn a_pot_of_boiled_water_poured_into_a_flask_frees_the_pot() {
    let mut game = new_game();
    game.debug_give(ItemKind::BoiledWater);
    game.debug_give(ItemKind::BambooFlask);
    // DRINK, then FILL FLASK.
    command(&mut game, 1);
    assert_eq!(game.carried(ItemKind::BoiledWater), 0);
    assert_eq!(game.carried(ItemKind::BarkContainer), 1);
    assert_eq!(cups(&game, ItemKind::BoiledWaterFlask), 1);
}

#[test]
fn a_flask_poured_out_is_kept_empty() {
    let mut game = new_game();
    game.debug_give(ItemKind::RawWaterFlask);
    // DRINK, then POUR OUT.
    command(&mut game, 1);
    assert_eq!(game.carried(ItemKind::RawWaterFlask), 0);
    assert_eq!(game.carried(ItemKind::BambooFlask), 1);
}

#[test]
fn a_flask_put_down_and_picked_up_again_keeps_its_water() {
    let mut game = new_game();
    game.debug_give_n(ItemKind::BoiledWaterFlask, 2);
    // DRINK, POUR OUT, MOVE, then PUT DOWN.
    command(&mut game, 3);
    hold(&mut game, 0, 30);
    assert_eq!(game.carried(ItemKind::BoiledWaterFlask), 0, "put down");
    press(&mut game, Buttons::X);
    hold(&mut game, 0, 30);
    press(&mut game, Buttons::A);
    hold(&mut game, 0, 30);
    assert_eq!(cups(&game, ItemKind::BoiledWaterFlask), 2);
}

#[test]
fn a_bamboo_flask_is_made_from_bamboo_and_a_branch() {
    let mut game = new_game();
    game.debug_give_n(ItemKind::Bamboo, 1);
    game.debug_give_n(ItemKind::Branch, 1);
    game.debug_give(ItemKind::StoneAxe);
    game.debug_give(ItemKind::StoneKnife);
    hold(&mut game, 0, 2);
    craft_the_only_recipe(&mut game);
    hold(&mut game, 0, 10);
    assert_eq!(game.carried(ItemKind::BambooFlask), 1);
    assert_eq!(game.carried(ItemKind::Bamboo), 0);
    assert_eq!(game.carried(ItemKind::Branch), 0, "whittled into the stopper");
}

/// The bag's CRAFT page, on the only recipe known: make it.
fn craft_the_only_recipe(game: &mut Game) {
    press(game, Buttons::X);
    press(game, Buttons::UP);
    press(game, Buttons::RIGHT);
    press(game, Buttons::A);
    press(game, Buttons::A);
    press(game, Buttons::A);
    press(game, Buttons::A);
}

/// The mill fitted out with `f`, the castaway in front of it (the cell
/// south of its south-easternmost), facing it.
fn facing(f: Furnishing, cell: impl Fn(&[(i32, i32)]) -> (i32, i32)) -> Game {
    let mut game = new_game();
    while game.debug_mill_advance() {}
    assert!(game.debug_furnish(f));
    let (x, y) = cell(&game.map().furnishing_cells(f));
    game.debug_place_player((x, y + 1));
    hold(&mut game, 0, 2);
    hold(&mut game, Buttons::UP, 2);
    hold(&mut game, 0, 2);
    game
}

#[test]
fn a_flask_of_raw_water_is_boiled_whole_at_the_fire_keeping_its_cups_and_the_pot() {
    let mut game = facing(Furnishing::Hearth, |cells| *cells.iter().max_by_key(|c| (c.1, -c.0)).unwrap());
    game.debug_give_n(ItemKind::PineCone, 1);
    press(&mut game, Buttons::A);
    hold(&mut game, 0, 60);
    assert!(game.is_by_a_fire());
    game.debug_give_n(ItemKind::RawWaterFlask, 3);
    game.debug_give(ItemKind::BarkContainer);
    hold(&mut game, 0, 2);
    craft_the_only_recipe(&mut game);
    hold(&mut game, 0, 300);
    assert_eq!(game.carried(ItemKind::RawWaterFlask), 0);
    assert_eq!(cups(&game, ItemKind::BoiledWaterFlask), 3);
    assert_eq!(game.carried(ItemKind::BarkContainer), 1, "the pot is kept");
}

#[test]
fn only_an_empty_flask_goes_on_the_shelf() {
    let mut game = facing(Furnishing::Shelf, |cells| cells[1]);
    game.debug_give(ItemKind::BoiledWaterFlask);
    game.debug_give(ItemKind::BambooFlask);
    press(&mut game, Buttons::A);
    // The flask of water first: it stays in the bag.
    press(&mut game, Buttons::A);
    assert_eq!(game.carried(ItemKind::BoiledWaterFlask), 1);
    press(&mut game, Buttons::Y);
    assert_eq!(game.carried(ItemKind::BoiledWaterFlask), 1, "nor all of its kind");
    assert_eq!(game.shelf().count(ItemKind::BoiledWaterFlask), 0);
    // The empty one beside it.
    press(&mut game, Buttons::RIGHT);
    press(&mut game, Buttons::A);
    assert_eq!(game.shelf().count(ItemKind::BambooFlask), 1);
}

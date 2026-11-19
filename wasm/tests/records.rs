//! Scenario: what the end tells of the game: the kinds of things made (of
//! all there are to make) and the recipes found (of all there are).

mod common;

use common::{new_game, press};
use survival_core::common::input::Buttons;
use survival_core::crafting::RECIPES;
use survival_core::game::Game;
use survival_core::items::ItemKind;

/// Opens the bag on the CRAFT list's first recipe, and makes it.
fn make_first_recipe(game: &mut Game) {
    press(game, Buttons::X);
    press(game, Buttons::UP);
    press(game, Buttons::RIGHT);
    press(game, Buttons::A);
    press(game, Buttons::A);
    press(game, Buttons::A);
    press(game, Buttons::A);
    press(game, Buttons::X);
}

#[test]
fn nothing_is_made_or_found_at_first() {
    let r = new_game().records();
    assert_eq!((r.made, r.recipes_known), (0, 0));
}

#[test]
fn the_kinds_to_make_are_counted_once_however_many_ways_there_are() {
    let r = new_game().records();
    let mut kinds: Vec<ItemKind> = RECIPES.iter().map(|r| r.makes).collect();
    kinds.sort_by_key(|k| format!("{k:?}"));
    kinds.dedup();
    assert_eq!(r.makeable, kinds.len());
    assert!(r.makeable < r.recipes, "rope is made two ways");
    assert_eq!(r.recipes, RECIPES.len());
}

#[test]
fn something_made_twice_counts_once() {
    let mut game = new_game();
    game.debug_give_n(ItemKind::Stone, 2);
    make_first_recipe(&mut game);
    assert!(game.carries(ItemKind::StoneKnife));
    game.debug_give_n(ItemKind::Stone, 2);
    make_first_recipe(&mut game);
    let r = game.records();
    assert_eq!(r.made, 1);
    assert!(r.recipes_known >= 1);
}

#[test]
fn what_was_made_is_saved() {
    let mut game = new_game();
    game.debug_give_n(ItemKind::Stone, 2);
    make_first_recipe(&mut game);
    let carried_on = Game::debug_continue(&game.debug_save_json()).expect("the save loads");
    assert_eq!(carried_on.records().made, 1);
}

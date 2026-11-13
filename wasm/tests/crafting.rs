//! Scenario: making things in the bag's CRAFT page.

mod common;

use common::{new_game, press};
use survival_core::common::input::Buttons;
use survival_core::game::Game;
use survival_core::items::ItemKind;

/// How many of `kind` are in the bag.
fn carried(game: &Game, kind: ItemKind) -> u64 {
    let save: serde_json::Value = serde_json::from_str(&game.debug_save_json()).unwrap();
    let name = serde_json::to_value(kind).unwrap();
    save["inventory"]["bag"]["items"].as_array().unwrap().iter().filter(|p| p["kind"] == name).map(|p| p["count"].as_u64().unwrap()).sum()
}

/// Opens the bag on the first entry of the CRAFT list's first recipe page.
fn open_first_recipe(game: &mut Game) {
    press(game, Buttons::X);
    press(game, Buttons::UP);
    press(game, Buttons::RIGHT);
    press(game, Buttons::A);
    press(game, Buttons::A);
}

/// A game carrying enough husks and bark for a rope either way, and the
/// knife to split the bark.
fn ready_for_rope() -> Game {
    let mut game = new_game();
    game.debug_give_n(ItemKind::Husk, 2);
    game.debug_give_n(ItemKind::Bark, 3);
    game.debug_give(ItemKind::StoneKnife);
    game
}

#[test]
fn rope_is_made_the_way_shown_on_its_page() {
    let mut game = ready_for_rope();
    open_first_recipe(&mut game);
    press(&mut game, Buttons::A);
    assert_eq!(carried(&game, ItemKind::Rope), 1);
    assert_eq!(carried(&game, ItemKind::Husk), 0, "the first way uses husks");
    assert_eq!(carried(&game, ItemKind::Bark), 3);
}

#[test]
fn left_and_right_on_the_page_change_the_way_it_is_made() {
    let mut game = ready_for_rope();
    open_first_recipe(&mut game);
    press(&mut game, Buttons::RIGHT);
    press(&mut game, Buttons::A);
    assert_eq!(carried(&game, ItemKind::Rope), 1);
    assert_eq!(carried(&game, ItemKind::Bark), 0, "the second way uses bark");
    assert_eq!(carried(&game, ItemKind::Husk), 2);
}

#[test]
fn the_page_opens_on_the_way_last_made() {
    let mut game = ready_for_rope();
    game.debug_give_n(ItemKind::Bark, 3);
    open_first_recipe(&mut game);
    press(&mut game, Buttons::RIGHT);
    press(&mut game, Buttons::A);
    press(&mut game, Buttons::X);
    open_first_recipe(&mut game);
    press(&mut game, Buttons::A);
    assert_eq!(carried(&game, ItemKind::Rope), 2);
    assert_eq!(carried(&game, ItemKind::Bark), 0, "made from bark both times");
    assert_eq!(carried(&game, ItemKind::Husk), 2);
}

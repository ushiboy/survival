//! Scenario: the tool belt. Worn at the waist, it holds two tools out of
//! the bag; a tool hung on it is still carried, and can be held and used.

mod common;

use common::{hold, new_game};
use survival_core::common::input::Buttons;
use survival_core::game::Game;
use survival_core::items::ItemKind;
use survival_core::world::Feature;

/// A game wearing a tool belt with `tool` hung on it, held in hand.
fn belted_with(tool: ItemKind) -> Game {
    let mut game = new_game();
    game.debug_give(ItemKind::ToolBelt);
    game.debug_give(tool);
    assert!(game.debug_wear(ItemKind::ToolBelt));
    assert!(game.debug_hang(tool));
    game
}

#[test]
fn the_tool_belt_comes_to_mind_with_bark_and_rope() {
    let mut game = new_game();
    game.debug_give_n(ItemKind::Bark, 1);
    game.debug_give_n(ItemKind::Rope, 1);
    hold(&mut game, 0, 2);
    assert!(game.is_struck_by_an_idea());
}

#[test]
fn a_tool_on_the_belt_is_carried_out_of_the_bag() {
    let game = belted_with(ItemKind::DiggingStick);
    assert!(game.carries(ItemKind::DiggingStick));
    assert_eq!(game.carried(ItemKind::DiggingStick), 0);
}

#[test]
fn a_tool_on_the_belt_is_used_from_the_hand() {
    let mut game = belted_with(ItemKind::DiggingStick);
    let map = game.map();
    let earth = (0..map.height())
        .flat_map(|y| (0..map.width()).map(move |x| (x, y)))
        .find(|&(x, y)| map.feature(x, y) == Feature::BareEarth(0))
        .expect("bare earth on the island");
    game.debug_place_player(earth);
    hold(&mut game, 0, 2);
    common::press(&mut game, Buttons::B);
    hold(&mut game, 0, 90);
    assert_eq!(game.carried(ItemKind::Soil), 1);
}

#[test]
fn a_belt_and_its_tools_are_still_worn_on_carrying_on() {
    let game = belted_with(ItemKind::StoneAxe);
    let saved = game.debug_save_json();
    let carried_on = Game::debug_continue(&saved).expect("the save loads");
    assert!(carried_on.carries(ItemKind::StoneAxe));
    assert_eq!(carried_on.carried(ItemKind::StoneAxe), 0);
    assert_eq!(carried_on.debug_save_json(), saved);
}

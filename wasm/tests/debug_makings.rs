//! Scenario: the dev tool that hands over all a thing takes to make: a
//! recipe's materials and tools, or a furnishing's for the workbench, so
//! something new can be tried without giving each material one by one.

mod common;

use common::new_game;
use survival_core::crafting::RECIPES;
use survival_core::game::mill::Furnishing;
use survival_core::game::{Making, debug_makings};
use survival_core::items::ItemKind;

/// Where `making` is in the list the dev tools give from.
fn index_of(making: Making) -> usize {
    debug_makings().iter().position(|&m| m == making).expect("listed")
}

fn recipe(name: &str) -> Making {
    Making::Recipe(RECIPES.iter().position(|r| r.name == name).expect("a recipe"))
}

#[test]
fn every_recipe_and_every_furnishing_is_listed() {
    let makings = debug_makings();
    for i in 0..RECIPES.len() {
        assert!(makings.contains(&Making::Recipe(i)));
    }
    for f in Furnishing::ALL {
        assert!(makings.contains(&Making::Furnishing(f)));
    }
}

#[test]
fn a_recipes_materials_and_tools_are_handed_over() {
    let mut game = new_game();
    assert_eq!(game.debug_give_makings(index_of(recipe("FishOil"))), 0, "it all fits");
    assert_eq!(game.carried(ItemKind::Fish), 2);
    assert_eq!(game.carried(ItemKind::Leaf), 1);
    assert!(game.carries(ItemKind::StoneKnife));
    assert!(game.carries(ItemKind::BarkContainer));
}

#[test]
fn a_furnishings_materials_and_tools_are_handed_over() {
    let mut game = new_game();
    assert_eq!(game.debug_give_makings(index_of(Making::Furnishing(Furnishing::Hearth))), 0);
    for &(kind, n) in Furnishing::Hearth.needs() {
        assert!(game.carried(kind) >= n, "{kind:?}");
    }
    for &tool in Furnishing::Hearth.tools() {
        assert!(game.carries(tool), "{tool:?}");
    }
}

#[test]
fn only_whats_missing_is_handed_over() {
    let mut game = new_game();
    game.debug_give_n(ItemKind::Fish, 1);
    game.debug_give(ItemKind::StoneKnife);
    game.debug_give_makings(index_of(recipe("FishOil")));
    assert_eq!(game.carried(ItemKind::Fish), 2, "topped up to what it takes");
    assert_eq!(game.carried(ItemKind::StoneKnife), 1, "no second knife");
    game.debug_give_makings(index_of(recipe("FishOil")));
    assert_eq!(game.carried(ItemKind::Fish), 2, "nothing more the second time");
}

#[test]
fn what_doesnt_fit_is_counted() {
    let mut game = new_game();
    while game.debug_give_n(ItemKind::LauanTimber, 1) == 1 {}
    assert!(game.debug_give_makings(index_of(recipe("FishOil"))) > 0);
}

#[test]
fn past_the_end_of_the_list_nothing_is_handed_over() {
    let mut game = new_game();
    assert_eq!(game.debug_give_makings(debug_makings().len()), 0);
    assert_eq!(game.carried(ItemKind::Fish), 0);
}

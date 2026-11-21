//! Scenario: the dev tools for building the mill step by step: how far it
//! has got, setting it up one step further (the wheel, the lathe, the
//! stakes, then each stage built), and handing over all the next stage
//! takes.

mod common;

use common::new_game;
use survival_core::game::mill::MILL_STAGES;
use survival_core::items::ItemKind;

#[test]
fn the_mill_is_set_up_one_step_at_a_time_through_every_stage() {
    let mut game = new_game();
    let steps = game.debug_mill_steps();
    assert_eq!(steps.len(), 3 + MILL_STAGES.len(), "the wheel, the lathe, the stakes, then the stages");
    assert_eq!(game.debug_mill_progress(), 0);
    for done in 1..=steps.len() {
        assert!(game.debug_mill_advance(), "step {done}");
        assert_eq!(game.debug_mill_progress(), done);
    }
    assert!(!game.debug_mill_advance(), "nothing further");
    assert_eq!(game.mill_stages_built(), MILL_STAGES.len());
}

#[test]
fn everything_the_next_stage_takes_is_handed_over() {
    let mut game = new_game();
    for _ in 0..3 {
        assert!(game.debug_mill_advance());
    }
    let stage = &MILL_STAGES[0];
    assert_eq!(game.debug_give_mill_stage(), 0, "it all fits");
    for &(kind, n) in stage.needs {
        assert!(game.carried(kind) >= n, "{kind:?}");
    }
    for &tool in stage.tools {
        assert!(game.carries(tool), "{tool:?}");
    }
}

#[test]
fn what_doesnt_fit_the_bag_goes_in_the_cart() {
    let mut game = new_game();
    for _ in 0..3 {
        assert!(game.debug_mill_advance());
    }
    game.debug_give(ItemKind::Handcart);
    // Fill the bag up first.
    while game.debug_give_n(ItemKind::LauanTimber, 1) == 1 {}
    assert_eq!(game.debug_give_mill_stage(), 0, "the cart takes the rest");
    assert!(game.cart_load(ItemKind::Stone) > 0);
}

#[test]
fn with_no_stage_left_nothing_is_handed_over() {
    let mut game = new_game();
    while game.debug_mill_advance() {}
    assert_eq!(game.debug_give_mill_stage(), 0);
    assert_eq!(game.carried(ItemKind::Stone), 0);
}

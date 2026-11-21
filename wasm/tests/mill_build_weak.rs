//! Scenario: building a stage of the mill takes hours of work, which wear
//! the castaway as hours awake do. Too weak to see it through with some
//! health to spare, they can't set to it.

mod common;

use common::{hold, new_game, press};
use survival_core::common::input::Buttons;
use survival_core::game::Game;
use survival_core::game::mill::MILL_STAGES;
use survival_core::world::map::site_bench;

/// The ground marked out, everything the foundation takes handed over, at
/// the bench with the BUILD screen open and BUILD chosen (and confirmed,
/// if it can be).
fn build_foundation_with_hp(hp: u32) -> Game {
    let mut game = new_game();
    while game.debug_mill_progress() < 3 {
        assert!(game.debug_mill_advance());
    }
    assert_eq!(game.debug_give_mill_stage(), 0);
    game.debug_set_hp(hp);
    let (bx, by) = game.map().spots.mill.map(site_bench).expect("a mill site");
    game.debug_place_player((bx - 1, by));
    hold(&mut game, 0, 2);
    hold(&mut game, Buttons::RIGHT, 2);
    hold(&mut game, 0, 2);
    press(&mut game, Buttons::A);
    for _ in 0..MILL_STAGES[0].needs.len() {
        press(&mut game, Buttons::A);
        press(&mut game, Buttons::A);
        press(&mut game, Buttons::DOWN);
    }
    press(&mut game, Buttons::A);
    press(&mut game, Buttons::A);
    hold(&mut game, 0, 400);
    game
}

#[test]
fn too_weak_to_see_the_work_through_the_castaway_doesnt_set_to_it() {
    let game = build_foundation_with_hp(4);
    assert_eq!(game.mill_stages_built(), 0, "not built");
    assert!(game.vitals().hp > 0, "and not worn out by it");
}

#[test]
fn with_health_enough_the_same_stage_is_built() {
    let game = build_foundation_with_hp(64);
    assert_eq!(game.mill_stages_built(), 1);
}

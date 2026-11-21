//! Scenario: raising the mill's frame, the stage after the foundation:
//! posts stood on the foundation's stones, plates along the north and
//! south walls, and beams across between them.

mod common;

use common::{hold, new_game, press};
use survival_core::common::input::Buttons;
use survival_core::game::Game;
use survival_core::game::mill::MILL_STAGES;
use survival_core::items::ItemKind;
use survival_core::world::Feature;
use survival_core::world::map::{foundation_stones, mill_plot, site_bench};

fn mill_site(game: &Game) -> (i32, i32) {
    game.map().spots.mill.expect("the island keeps ground for a mill")
}

#[test]
fn the_frame_comes_after_the_foundation_and_takes_lauan_pine_and_rope_with_the_axe_and_knife() {
    let frame = &MILL_STAGES[1];
    assert_eq!(frame.name, "FRAME");
    assert!(frame.needs.contains(&(ItemKind::LauanTimber, 8)));
    assert!(frame.needs.contains(&(ItemKind::PineTimber, 3)));
    assert!(frame.needs.contains(&(ItemKind::Rope, 4)));
    assert!(frame.tools.contains(&ItemKind::StoneAxe) && frame.tools.contains(&ItemKind::StoneKnife));
    assert!(frame.hours > MILL_STAGES[0].hours, "more work than the foundation");
}

/// The foundation laid, everything the frame takes handed over, and the
/// castaway at the bench, facing it.
fn ready_for_the_frame() -> Game {
    let mut game = new_game();
    while game.debug_mill_progress() < 4 {
        assert!(game.debug_mill_advance());
    }
    game.debug_give(ItemKind::Handcart);
    assert_eq!(game.debug_give_mill_stage(), 0);
    let (bx, by) = site_bench(mill_site(&game));
    game.debug_place_player((bx - 1, by));
    hold(&mut game, 0, 2);
    hold(&mut game, Buttons::RIGHT, 2);
    hold(&mut game, 0, 2);
    game
}

/// At the BUILD screen: puts every material in (asked, yes), then builds
/// (asked, yes) and waits.
fn put_in_and_build(game: &mut Game) {
    press(game, Buttons::A);
    let lines = MILL_STAGES[game.mill_stages_built()].needs.len();
    for _ in 0..lines {
        press(game, Buttons::A);
        press(game, Buttons::A);
        press(game, Buttons::DOWN);
    }
    press(game, Buttons::A);
    press(game, Buttons::A);
    hold(game, 0, 900);
}

#[test]
fn with_everything_in_the_frame_goes_up_on_the_foundation() {
    let mut game = ready_for_the_frame();
    let before = game.debug_hours_since_start();
    put_in_and_build(&mut game);
    assert_eq!(game.mill_stages_built(), 2);
    assert!(game.debug_hours_since_start() >= before + MILL_STAGES[1].hours);
    let site = mill_site(&game);
    let map = game.map();
    for (x, y) in foundation_stones(site) {
        assert_eq!(map.feature(x, y), Feature::Framed(true), "a post on the stone at {:?}", (x, y));
        assert!(map.props(x, y).solid, "the post stands in the way");
    }
    let (x0, y0, x1, y1) = mill_plot(site);
    let floor = (y0..=y1).flat_map(|y| (x0..=x1).map(move |x| (x, y))).filter(|c| !foundation_stones(site).contains(c));
    for (x, y) in floor {
        assert!(matches!(map.feature(x, y), Feature::Framed(false) | Feature::Lathe(_)), "{:?}", (x, y));
        assert!(!map.props(x, y).solid || matches!(map.feature(x, y), Feature::Lathe(_)), "walked through under the beams");
    }
}

#[test]
fn the_frame_is_still_up_on_carrying_on() {
    let mut game = new_game();
    while game.debug_mill_advance() {}
    let carried_on = Game::debug_continue(&game.debug_save_json()).expect("the save loads");
    assert_eq!(carried_on.mill_stages_built(), MILL_STAGES.len());
    let (x0, y0, _, _) = mill_plot(mill_site(&carried_on));
    assert_eq!(carried_on.map().feature(x0, y0), Feature::Framed(true));
}

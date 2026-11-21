//! Scenario: marking out the mill. Once the wood lathe turns beside the
//! water wheel, marking stakes held near it are driven in at the corners
//! of the ground the mill will stand on, with rope run between them. The
//! lathe stands inside it, for the mill to be built round it. Without the
//! stakes in hand, the ground beside the lathe gives pause.

mod common;

use common::{hold, new_game, press};
use survival_core::common::input::Buttons;
use survival_core::game::Game;
use survival_core::items::ItemKind;
use survival_core::world::Feature;
use survival_core::world::map::{mill_plot, stakes_stand};

fn mill_site(game: &Game) -> (i32, i32) {
    game.map().spots.mill.expect("the island keeps ground for a mill")
}

fn stakes_in(game: &Game) -> usize {
    let map = game.map();
    (0..map.height()).map(|y| (0..map.width()).filter(|&x| matches!(map.feature(x, y), Feature::Marking(m) if m.stake)).count()).sum()
}

/// A game with the wheel turning (and the lathe beside it, if `lathe`),
/// the castaway standing `(dx, dy)` cells from the mill site's bank,
/// facing `facing` (held a moment).
fn near_the_mill(lathe: bool, (dx, dy): (i32, i32), facing: u16) -> Game {
    let mut game = new_game();
    assert!(game.debug_set_wheel());
    if lathe {
        assert!(game.debug_set_lathe());
    }
    let (x, y) = mill_site(&game);
    game.debug_place_player((x + dx, y + dy));
    hold(&mut game, 0, 2);
    hold(&mut game, facing, 2);
    hold(&mut game, 0, 2);
    game
}

#[test]
fn stakes_held_near_the_lathe_mark_out_the_mill_round_it() {
    for (at, facing) in [((-5, 3), Buttons::DOWN), ((-9, -2), Buttons::LEFT), ((-3, 5), Buttons::UP)] {
        let mut game = near_the_mill(true, at, facing);
        game.debug_give(ItemKind::MarkingStakes);
        press(&mut game, Buttons::B);
        hold(&mut game, 0, 300);
        assert_eq!(stakes_in(&game), 4, "a stake at each corner, from {at:?}");
        assert!(game.map().is_staked());
        assert!(!game.carries(ItemKind::MarkingStakes), "they're used, from {at:?}");
        let site = mill_site(&game);
        assert_eq!(game.player_tile(), stakes_stand(site), "the castaway stepped over to drive them in, from {at:?}");
    }
}

#[test]
fn the_ground_marked_out_takes_in_the_lathe() {
    let game = near_the_mill(true, (-5, 3), Buttons::DOWN);
    let (x0, y0, x1, y1) = mill_plot(mill_site(&game));
    let (lx, ly) = game.map().lathe_middle().expect("the lathe stands");
    let (lx, ly) = (lx.div_euclid(16), ly.div_euclid(16));
    assert!((x0..=x1).contains(&lx) && (y0..=y1).contains(&ly));
}

#[test]
fn the_stakes_stand_in_the_way_but_the_rope_between_them_is_stepped_over() {
    let mut game = near_the_mill(true, (-5, 3), Buttons::DOWN);
    assert!(game.debug_set_stakes());
    let map = game.map();
    let (x0, y0, x1, y1) = mill_plot(mill_site(&game));
    for (x, y) in [(x0, y0), (x1, y0), (x0, y1), (x1, y1)] {
        assert!(map.props(x, y).solid, "the stake at {:?}", (x, y));
    }
    let mid = (x0 + x1) / 2;
    assert!(matches!(map.feature(mid, y1), Feature::Marking(m) if !m.stake && m.ropes != 0));
    assert!(!map.props(mid, y1).solid, "the rope along the front");
    hold(&mut game, 0, 1);
}

#[test]
fn stakes_wont_be_driven_in_before_the_lathe_turns() {
    let mut game = near_the_mill(false, (-5, 3), Buttons::DOWN);
    game.debug_give(ItemKind::MarkingStakes);
    press(&mut game, Buttons::B);
    hold(&mut game, 0, 300);
    assert_eq!(stakes_in(&game), 0);
    assert!(game.carries(ItemKind::MarkingStakes));
}

#[test]
fn stakes_held_far_from_the_mill_arent_driven_in() {
    let mut game = near_the_mill(true, (-14, 0), Buttons::RIGHT);
    game.debug_give(ItemKind::MarkingStakes);
    press(&mut game, Buttons::B);
    hold(&mut game, 0, 300);
    assert_eq!(stakes_in(&game), 0);
}

#[test]
fn without_stakes_the_castaway_stops_to_think_beside_the_lathe() {
    let mut game = near_the_mill(true, (-5, 3), Buttons::DOWN);
    press(&mut game, Buttons::A);
    assert!(game.is_pondering());
}

#[test]
fn before_the_lathe_turns_there_is_no_mill_to_think_about() {
    let mut game = near_the_mill(false, (-6, 3), Buttons::DOWN);
    press(&mut game, Buttons::A);
    assert!(!game.is_pondering());
}

#[test]
fn once_the_mill_is_marked_out_there_is_nothing_more_to_think_about() {
    let mut game = near_the_mill(true, (-5, 3), Buttons::DOWN);
    assert!(game.debug_set_stakes());
    hold(&mut game, 0, 2);
    press(&mut game, Buttons::A);
    assert!(!game.is_pondering());
}

#[test]
fn the_marked_out_mill_is_still_there_on_carrying_on() {
    let mut game = new_game();
    assert!(game.debug_set_wheel());
    assert!(game.debug_set_lathe());
    assert!(game.debug_set_stakes());
    let carried_on = Game::debug_continue(&game.debug_save_json()).expect("the save loads");
    assert!(carried_on.map().is_staked());
}

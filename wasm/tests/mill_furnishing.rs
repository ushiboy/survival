//! Scenario: fitting out the built mill. A at its workbench opens FIT OUT,
//! where a shelf and a bed are made from materials at hand, taking hours
//! of work; each then stands in its place in the room. Things left on the
//! shelf (as many as like) go into what's made in the mill; sleeping in
//! the bed mends faster than on the floor, and ailments pass sooner.

mod common;

use common::{hold, new_game, press};
use survival_core::common::input::Buttons;
use survival_core::game::Game;
use survival_core::game::menu::Page;
use survival_core::game::mill::Furnishing;
use survival_core::game::state::Ailment;
use survival_core::items::ItemKind;
use survival_core::world::Feature;

/// The mill built, the castaway in its room in front of the workbench,
/// facing it.
fn at_the_workbench() -> Game {
    let mut game = new_game();
    while game.debug_mill_advance() {}
    let (bx, by) = game.map().mill_room_bench();
    game.debug_place_player((bx, by + 1));
    hold(&mut game, 0, 2);
    hold(&mut game, Buttons::UP, 2);
    hold(&mut game, 0, 2);
    assert!(game.is_in_mill_room());
    game
}

/// Everything `f` takes (and the tools for it), in the bag.
fn give_what_it_takes(game: &mut Game, f: Furnishing) {
    for &(kind, n) in f.needs() {
        assert_eq!(game.debug_give_n(kind, n), n, "{kind:?} fits in the bag");
    }
    for &tool in f.tools() {
        game.debug_give(tool);
    }
    hold(game, 0, 2);
}

/// A at the workbench, down to `f` in the list, A to open it, A to make
/// it and A to say yes.
fn make(game: &mut Game, f: Furnishing) {
    press(game, Buttons::A);
    let place = Furnishing::ALL.iter().position(|&g| g == f).unwrap();
    for _ in 0..place {
        press(game, Buttons::DOWN);
    }
    for _ in 0..3 {
        press(game, Buttons::A);
    }
}

/// Lets the work go on until it's done.
fn work_through(game: &mut Game) {
    hold(game, 0, 400);
}

#[test]
fn a_at_the_workbench_in_the_built_mill_opens_fit_out_on_its_own() {
    let mut game = at_the_workbench();
    press(&mut game, Buttons::A);
    assert_eq!(game.bag_page(), Some(Page::FitOut));
    assert_eq!(game.bag_pages(), vec![Page::FitOut]);
    press(&mut game, Buttons::B);
    assert_eq!(game.bag_page(), None, "B closes it");
}

#[test]
fn a_shelf_made_at_the_workbench_stands_against_the_north_wall() {
    let mut game = at_the_workbench();
    give_what_it_takes(&mut game, Furnishing::Shelf);
    make(&mut game, Furnishing::Shelf);
    assert_eq!(game.bag_page(), None, "the screen closes to set to work");
    work_through(&mut game);
    assert_eq!(game.furnished(), &[Furnishing::Shelf]);
    for (x, y) in game.map().furnishing_cells(Furnishing::Shelf) {
        assert!(matches!(game.map().feature(x, y), Feature::Shelf(_)), "{:?}", (x, y));
    }
    for &(kind, _) in Furnishing::Shelf.needs() {
        assert_eq!(game.carried(kind), 0, "{kind:?} used up");
    }
}

#[test]
fn making_a_furnishing_takes_hours() {
    let mut game = at_the_workbench();
    give_what_it_takes(&mut game, Furnishing::Bed);
    let before = game.clock().total_minutes();
    make(&mut game, Furnishing::Bed);
    work_through(&mut game);
    assert!(game.clock().total_minutes() >= before + Furnishing::Bed.hours() * 60);
}

/// The bed made, the castaway beside it.
fn by_the_bed() -> Game {
    let mut game = at_the_workbench();
    assert!(game.debug_furnish(Furnishing::Bed));
    let (x, y) = game.map().bedside();
    game.debug_place_player((x, y));
    hold(&mut game, 0, 2);
    game
}

/// Lies down in the bed (facing it, A) or (`in_bed` false) on the floor
/// where they stand (L), and sleeps a while.
fn sleep_a_while(game: &mut Game, in_bed: bool) {
    if in_bed {
        hold(game, Buttons::LEFT, 2);
        hold(game, 0, 2);
        press(game, Buttons::A);
    } else {
        press(game, Buttons::L);
    }
    hold(game, 0, 120);
}

#[test]
fn sleeping_in_the_bed_mends_faster_than_on_the_floor() {
    let mended = |in_bed: bool| {
        let mut game = by_the_bed();
        game.debug_set_hp(10);
        sleep_a_while(&mut game, in_bed);
        game.vitals().hp - 10
    };
    assert!(mended(true) > mended(false));
}

#[test]
fn an_upset_stomach_passes_sooner_in_the_bed() {
    let still_upset = |in_bed: bool| {
        let mut game = by_the_bed();
        game.debug_upset_stomach();
        sleep_a_while(&mut game, in_bed);
        game.vitals().has(Ailment::UpsetStomach)
    };
    assert!(!still_upset(true));
    assert!(still_upset(false));
}

#[test]
fn getting_out_of_the_bed_the_castaway_stands_beside_it_at_once() {
    let mut game = by_the_bed();
    sleep_a_while(&mut game, true);
    press(&mut game, Buttons::A);
    assert_eq!(game.player_tile(), game.map().bedside());
    assert!(game.player_is_idle(), "standing, not getting up off the floor");
}

#[test]
fn pulling_the_cart_the_castaway_cant_lie_in_the_bed_and_ponders() {
    let mut game = by_the_bed();
    game.debug_give(ItemKind::Handcart);
    hold(&mut game, 0, 2);
    hold(&mut game, Buttons::LEFT, 2);
    hold(&mut game, 0, 2);
    press(&mut game, Buttons::A);
    assert!(game.is_pondering(), "the cart wants setting down first");
    assert!(game.is_pulling_cart(), "A leaves the cart be");
    hold(&mut game, 0, 120);
    assert_eq!(game.player_tile(), game.map().bedside(), "not in the bed");
}

/// The shelf made, the castaway in front of it, facing it.
fn at_the_shelf() -> Game {
    let mut game = at_the_workbench();
    assert!(game.debug_furnish(Furnishing::Shelf));
    let (x, y) = game.map().furnishing_cells(Furnishing::Shelf)[1];
    game.debug_place_player((x, y + 1));
    hold(&mut game, 0, 2);
    hold(&mut game, Buttons::UP, 2);
    hold(&mut game, 0, 2);
    game
}

#[test]
fn a_at_the_shelf_opens_its_screen() {
    let mut game = at_the_shelf();
    press(&mut game, Buttons::A);
    assert_eq!(game.bag_page(), Some(Page::ShelveBag));
    assert_eq!(game.bag_pages(), vec![Page::ShelveBag, Page::Shelf]);
}

#[test]
fn whats_left_on_the_shelf_is_kept_on_carrying_on_from_a_save() {
    let mut game = at_the_shelf();
    game.debug_give_n(ItemKind::Rope, 5);
    hold(&mut game, 0, 2);
    press(&mut game, Buttons::A);
    press(&mut game, Buttons::Y);
    press(&mut game, Buttons::B);
    assert_eq!(game.shelf().count(ItemKind::Rope), 5);
    let carried_on = Game::debug_continue(&game.debug_save_json()).expect("the save loads");
    assert_eq!(carried_on.shelf().count(ItemKind::Rope), 5);
    assert_eq!(carried_on.furnished(), &[Furnishing::Shelf]);
    let (x, y) = carried_on.map().furnishing_cells(Furnishing::Shelf)[0];
    assert!(matches!(carried_on.map().feature(x, y), Feature::Shelf(_)));
}

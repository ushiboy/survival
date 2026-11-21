//! Scenario: laying the mill's foundation. Marking its ground out sets up
//! a bench beside it; A at the bench opens the BUILD screen, listing what
//! the stage being built (the foundation) takes. Materials are left at the site a few at a time,
//! from the bag (and the cart, when it's at hand); once they're all in and
//! the tools are carried, the castaway builds it, which takes hours.

mod common;

use common::{hold, new_game, press};
use survival_core::common::input::Buttons;
use survival_core::game::Game;
use survival_core::game::menu::Page;
use survival_core::game::mill::MILL_STAGES;
use survival_core::items::ItemKind;
use survival_core::world::Feature;
use survival_core::world::map::{mill_plot, site_bench};

const STONES: u32 = 10;
const SOIL: u32 = 4;

fn mill_site(game: &Game) -> (i32, i32) {
    game.map().spots.mill.expect("the island keeps ground for a mill")
}

#[test]
fn the_foundation_is_the_first_stage_and_takes_stone_and_soil_with_a_digging_stick() {
    let foundation = &MILL_STAGES[0];
    assert_eq!(foundation.name, "FOUNDATION");
    assert!(foundation.needs.contains(&(ItemKind::Stone, STONES)));
    assert!(foundation.needs.contains(&(ItemKind::Soil, SOIL)));
    assert!(foundation.tools.contains(&ItemKind::DiggingStick));
}

/// A game with the mill's ground marked out (if `staked`; otherwise only
/// the lathe set), the castaway standing just west of the site's bench,
/// facing it.
fn on_the_ground(staked: bool) -> Game {
    let mut game = new_game();
    assert!(game.debug_set_wheel());
    assert!(game.debug_set_lathe());
    if staked {
        assert!(game.debug_set_stakes());
    }
    let (bx, by) = site_bench(mill_site(&game));
    game.debug_place_player((bx - 1, by));
    hold(&mut game, 0, 2);
    hold(&mut game, Buttons::RIGHT, 2);
    hold(&mut game, 0, 2);
    game
}

#[test]
fn marking_the_ground_out_sets_up_a_bench_beside_it_that_stands_in_the_way() {
    let game = on_the_ground(true);
    let (x, y) = site_bench(mill_site(&game));
    assert_eq!(game.map().feature(x, y), Feature::SiteBench);
    assert!(game.map().props(x, y).solid);
    let (x0, y0, x1, y1) = mill_plot(mill_site(&game));
    assert!(!((x0..=x1).contains(&x) && (y0..=y1).contains(&y)), "out of the way of the building");
}

#[test]
fn a_at_the_bench_opens_the_build_screen() {
    let mut game = on_the_ground(true);
    press(&mut game, Buttons::A);
    assert_eq!(game.bag_page(), Some(Page::Build));
    assert_eq!(game.bag_pages(), vec![Page::Build], "on its own, not among the bag's pages");
    press(&mut game, Buttons::B);
    assert_eq!(game.bag_page(), None, "B closes it");
}

#[test]
fn the_bag_has_no_build_page() {
    let mut game = on_the_ground(true);
    press(&mut game, Buttons::X);
    assert!(!game.bag_pages().contains(&Page::Build));
}

/// Opens the build screen (A at the bench), presses A on the line of the
/// stage's materials `line` (0 for the first) and A again to the question
/// whether to put them in, then closes it.
fn leave(game: &mut Game, line: usize) {
    press(game, Buttons::A);
    assert_eq!(game.bag_page(), Some(Page::Build));
    for _ in 0..line {
        press(game, Buttons::DOWN);
    }
    press(game, Buttons::A);
    press(game, Buttons::A);
    press(game, Buttons::B);
    hold(game, 0, 2);
}

#[test]
fn materials_go_in_only_once_asked_and_b_puts_none_in() {
    let mut game = on_the_ground(true);
    game.debug_give_n(ItemKind::Stone, 4);
    press(&mut game, Buttons::A);
    press(&mut game, Buttons::A);
    assert_eq!(game.mill_delivered(ItemKind::Stone), 0, "asked first");
    assert_eq!(game.bag_page(), Some(Page::Build));
    press(&mut game, Buttons::B);
    assert_eq!(game.mill_delivered(ItemKind::Stone), 0, "B: none put in");
    assert_eq!(game.bag_page(), Some(Page::Build), "still on the screen");
    press(&mut game, Buttons::A);
    press(&mut game, Buttons::A);
    assert_eq!(game.mill_delivered(ItemKind::Stone), 4);
}

#[test]
fn materials_left_a_few_at_a_time_add_up() {
    let mut game = on_the_ground(true);
    game.debug_give_n(ItemKind::Stone, 4);
    leave(&mut game, 0);
    assert_eq!(game.mill_delivered(ItemKind::Stone), 4);
    assert_eq!(game.carried(ItemKind::Stone), 0, "they're left at the site");
    game.debug_give_n(ItemKind::Stone, 3);
    leave(&mut game, 0);
    assert_eq!(game.mill_delivered(ItemKind::Stone), 7);
}

#[test]
fn no_more_is_left_than_the_stage_takes() {
    let mut game = on_the_ground(true);
    game.debug_give_n(ItemKind::Stone, STONES + 3);
    leave(&mut game, 0);
    assert_eq!(game.mill_delivered(ItemKind::Stone), STONES);
    assert_eq!(game.carried(ItemKind::Stone), 3, "the rest are kept");
}

#[test]
fn with_the_cart_at_hand_what_the_bag_lacks_comes_off_the_cart() {
    let mut game = on_the_ground(true);
    game.debug_give(ItemKind::Handcart);
    hold(&mut game, 0, 2);
    assert_eq!(game.debug_load_cart(ItemKind::Stone, 8), 8);
    game.debug_give_n(ItemKind::Stone, 4);
    leave(&mut game, 0);
    assert_eq!(game.mill_delivered(ItemKind::Stone), STONES);
    assert_eq!(game.carried(ItemKind::Stone), 0, "the bag's first");
    assert_eq!(game.cart_load(ItemKind::Stone), 2);
}

/// Everything the foundation takes left at the site, and the tools carried
/// if `tools`.
fn all_in(tools: bool) -> Game {
    let mut game = on_the_ground(true);
    game.debug_give_n(ItemKind::Stone, STONES);
    game.debug_give_n(ItemKind::Soil, SOIL);
    leave(&mut game, 0);
    leave(&mut game, 1);
    if tools {
        game.debug_give(ItemKind::DiggingStick);
    }
    game
}

/// Opens the build screen, goes down to BUILD and presses A twice (asked
/// whether to build, yes), then waits `frames`.
fn build(game: &mut Game, frames: u32) {
    press(game, Buttons::A);
    for _ in 0..MILL_STAGES[0].needs.len() {
        press(game, Buttons::DOWN);
    }
    press(game, Buttons::A);
    press(game, Buttons::A);
    hold(game, 0, frames);
}

fn foundation_laid(game: &Game) -> bool {
    let map = game.map();
    let (x0, y0, x1, y1) = mill_plot(mill_site(game));
    (y0..=y1).all(|y| (x0..=x1).all(|x| matches!(map.feature(x, y), Feature::Foundation(_) | Feature::Lathe(_))))
}

#[test]
fn with_everything_in_and_the_tools_carried_the_foundation_is_laid() {
    let mut game = all_in(true);
    let before = game.debug_hours_since_start();
    build(&mut game, 600);
    assert_eq!(game.mill_stages_built(), 1);
    assert!(foundation_laid(&game), "the ground is tamped and the stones are down");
    assert!(game.debug_hours_since_start() >= before + MILL_STAGES[0].hours, "it took hours");
    assert!(!game.map().is_staked(), "the stakes and rope are gone");
}

#[test]
fn short_of_materials_nothing_is_built() {
    let mut game = on_the_ground(true);
    game.debug_give_n(ItemKind::Stone, STONES - 1);
    game.debug_give_n(ItemKind::Soil, SOIL);
    game.debug_give(ItemKind::DiggingStick);
    leave(&mut game, 0);
    leave(&mut game, 1);
    build(&mut game, 600);
    assert_eq!(game.mill_stages_built(), 0);
}

#[test]
fn without_the_tools_nothing_is_built() {
    let mut game = all_in(false);
    build(&mut game, 600);
    assert_eq!(game.mill_stages_built(), 0);
}

#[test]
fn building_wears_the_castaway_as_hours_awake_do() {
    let mut game = all_in(true);
    game.debug_set_vitals(None, Some(64), Some(64));
    build(&mut game, 600);
    let (food, water) = (game.vitals().food, game.vitals().water);
    assert!(food < 64 && water < 64, "hungrier and thirstier for the work");
}

#[test]
fn once_the_foundation_is_laid_there_is_nothing_more_to_build_yet() {
    let mut game = all_in(true);
    build(&mut game, 600);
    let (bx, by) = site_bench(mill_site(&game));
    game.debug_place_player((bx - 1, by));
    hold(&mut game, Buttons::RIGHT, 2);
    hold(&mut game, 0, 2);
    press(&mut game, Buttons::A);
    assert_eq!(game.bag_page(), None);
}

#[test]
fn what_was_left_at_the_site_is_still_there_on_carrying_on() {
    let mut game = on_the_ground(true);
    game.debug_give_n(ItemKind::Stone, 6);
    leave(&mut game, 0);
    let carried_on = Game::debug_continue(&game.debug_save_json()).expect("the save loads");
    assert_eq!(carried_on.mill_delivered(ItemKind::Stone), 6);
}

#[test]
fn the_foundation_is_still_there_on_carrying_on() {
    let mut game = all_in(true);
    build(&mut game, 600);
    let carried_on = Game::debug_continue(&game.debug_save_json()).expect("the save loads");
    assert_eq!(carried_on.mill_stages_built(), 1);
    assert!(foundation_laid(&carried_on));
}

#[test]
fn pulling_the_cart_up_to_the_bench_a_opens_the_build_screen_and_takes_from_the_cart() {
    let mut game = on_the_ground(true);
    game.debug_give(ItemKind::Handcart);
    hold(&mut game, 0, 2);
    assert_eq!(game.debug_load_cart(ItemKind::Stone, 5), 5);
    leave(&mut game, 0);
    assert!(game.is_pulling_cart(), "still pulled");
    assert_eq!(game.mill_delivered(ItemKind::Stone), 5);
}

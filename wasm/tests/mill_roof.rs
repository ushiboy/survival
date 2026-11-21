//! Scenario: roofing the mill, its last stage: thatch laid over bamboo
//! rafters and lashed down, a sliding plank door hung in the doorway, and
//! the bench cleared away.

mod common;

use common::{hold, new_game, press};
use survival_core::common::input::Buttons;
use survival_core::game::Game;
use survival_core::game::mill::MILL_STAGES;
use survival_core::items::ItemKind;
use survival_core::world::Feature;
use survival_core::world::map::{MILL_DOOR_OPEN, mill_doorway, mill_plot, site_bench};

fn mill_site(game: &Game) -> (i32, i32) {
    game.map().spots.mill.expect("the island keeps ground for a mill")
}

#[test]
fn the_roof_is_the_last_stage_and_takes_thatch_bamboo_rope_and_lauan_with_the_knife() {
    let roof = MILL_STAGES.last().expect("stages");
    assert_eq!(MILL_STAGES.len(), 4);
    assert_eq!(roof.name, "ROOF");
    for need in [(ItemKind::Thatch, 6), (ItemKind::Bamboo, 8), (ItemKind::Rope, 4), (ItemKind::LauanTimber, 2)] {
        assert!(roof.needs.contains(&need), "{need:?}");
    }
    assert_eq!(roof.tools, &[ItemKind::StoneKnife]);
}

/// The walls up, everything the roof takes handed over, and the roof's
/// building just begun at the bench.
fn roofing() -> Game {
    let mut game = new_game();
    while game.debug_mill_progress() < 6 {
        assert!(game.debug_mill_advance());
    }
    game.debug_give(ItemKind::Handcart);
    assert_eq!(game.debug_give_mill_stage(), 0);
    let (bx, by) = site_bench(mill_site(&game));
    game.debug_place_player((bx - 1, by));
    hold(&mut game, 0, 2);
    hold(&mut game, Buttons::RIGHT, 2);
    hold(&mut game, 0, 2);
    press(&mut game, Buttons::A);
    for _ in 0..MILL_STAGES[3].needs.len() {
        press(&mut game, Buttons::A);
        press(&mut game, Buttons::A);
        press(&mut game, Buttons::DOWN);
    }
    press(&mut game, Buttons::A);
    press(&mut game, Buttons::A);
    hold(&mut game, 0, 2);
    game
}

/// The same, the roof built.
fn roofed() -> Game {
    let mut game = roofing();
    hold(&mut game, 0, 1500);
    game
}

#[test]
fn while_roofing_the_castaway_works_up_on_the_roof_the_door_not_yet_hung() {
    let game = roofing();
    let site = mill_site(&game);
    let map = game.map();
    assert!(map.shows_mill_roof(), "the roof's drawn from the start of the work");
    assert!(!map.has_mill_roof(), "but it isn't done: no door yet");
    let (x0, y0, x1, y1) = mill_plot(site);
    let (x, y) = game.player_tile();
    assert!((x0..=x1).contains(&x) && (y0..y1).contains(&y), "up over the ground: {:?}", (x, y));
}

#[test]
fn with_everything_in_the_roof_goes_on_a_door_is_hung_and_the_bench_is_cleared() {
    let game = roofed();
    assert_eq!(game.mill_stages_built(), 4);
    let site = mill_site(&game);
    let map = game.map();
    assert!(map.has_mill_roof());
    for (x, y) in mill_doorway(site) {
        assert_eq!(map.feature(x, y), Feature::MillDoor, "{:?}", (x, y));
    }
    let (bx, by) = site_bench(site);
    assert_eq!(map.feature(bx, by), Feature::None, "the bench is cleared away");
}

#[test]
fn the_door_opens_as_the_castaway_comes_up_to_it_and_shuts_after_them() {
    let mut game = roofed();
    let (dx, dy) = mill_doorway(mill_site(&game))[0];
    game.debug_place_player((dx, dy + 5));
    hold(&mut game, 0, 30);
    assert_eq!(game.map().mill_door(), 0, "shut with no one by it");
    game.debug_place_player((dx, dy + 1));
    hold(&mut game, 0, 30);
    assert_eq!(game.map().mill_door(), MILL_DOOR_OPEN, "open in front of it");
    game.debug_place_player((dx + 1, dy - 1));
    hold(&mut game, 0, 30);
    assert_eq!(game.map().mill_door(), MILL_DOOR_OPEN, "open just inside it");
    game.debug_place_player((dx, dy + 5));
    hold(&mut game, 0, 30);
    assert_eq!(game.map().mill_door(), 0, "shut again once they're away");
}

#[test]
fn once_the_roof_is_on_there_is_nothing_more_to_build() {
    let mut game = roofed();
    assert_eq!(game.debug_give_mill_stage(), 0);
    assert!(!game.debug_mill_advance(), "every step done");
}

#[test]
fn the_roof_and_door_are_still_there_on_carrying_on_the_door_shut() {
    let mut game = roofed();
    let (dx, dy) = mill_doorway(mill_site(&game))[0];
    game.debug_place_player((dx, dy + 1));
    hold(&mut game, 0, 30);
    let carried_on = Game::debug_continue(&game.debug_save_json()).expect("the save loads");
    assert_eq!(carried_on.mill_stages_built(), 4);
    assert!(carried_on.map().has_mill_roof());
    assert_eq!(carried_on.map().feature(dx, dy), Feature::MillDoor);
    assert_eq!(carried_on.map().mill_door(), 0, "how far it stood open isn't kept");
}

#[test]
fn once_the_roof_is_on_the_castaway_is_back_down_where_they_stood_by_the_bench() {
    let game = roofed();
    let (bx, by) = site_bench(mill_site(&game));
    assert_eq!(game.player_tile(), (bx - 1, by), "back where they stood");
}

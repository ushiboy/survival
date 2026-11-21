//! Scenario: walling the mill, the stage after the frame: bamboo lath
//! between the posts, plastered with clay and soil, all round but a
//! doorway in the south wall.

mod common;

use common::{hold, new_game, press};
use survival_core::common::input::Buttons;
use survival_core::game::Game;
use survival_core::game::mill::MILL_STAGES;
use survival_core::items::ItemKind;
use survival_core::world::Feature;
use survival_core::world::map::{mill_doorway, mill_plot, site_bench};

fn mill_site(game: &Game) -> (i32, i32) {
    game.map().spots.mill.expect("the island keeps ground for a mill")
}

#[test]
fn the_walls_come_after_the_frame_and_take_bamboo_vine_clay_and_soil_with_the_knife_and_spade() {
    let walls = &MILL_STAGES[2];
    assert_eq!(walls.name, "WALLS");
    for need in [(ItemKind::Bamboo, 8), (ItemKind::Vine, 4), (ItemKind::Clay, 6), (ItemKind::Soil, 6)] {
        assert!(walls.needs.contains(&need), "{need:?}");
    }
    assert!(walls.tools.contains(&ItemKind::StoneKnife) && walls.tools.contains(&ItemKind::BambooSpade));
}

/// The frame up, everything the walls take handed over, and the walls
/// built at the bench.
fn walled() -> Game {
    let mut game = new_game();
    while game.debug_mill_progress() < 5 {
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
    for _ in 0..MILL_STAGES[2].needs.len() {
        press(&mut game, Buttons::A);
        press(&mut game, Buttons::A);
        press(&mut game, Buttons::DOWN);
    }
    press(&mut game, Buttons::A);
    press(&mut game, Buttons::A);
    hold(&mut game, 0, 900);
    game
}

#[test]
fn with_everything_in_the_walls_go_up_round_the_frame_but_the_doorway() {
    let game = walled();
    assert_eq!(game.mill_stages_built(), 3);
    let site = mill_site(&game);
    let (x0, y0, x1, y1) = mill_plot(site);
    let doorway = mill_doorway(site);
    let map = game.map();
    for y in y0..=y1 {
        for x in x0..=x1 {
            let edge = x == x0 || x == x1 || y == y0 || y == y1;
            match map.feature(x, y) {
                Feature::MillWall => assert!(edge && !doorway.contains(&(x, y)), "a wall at {:?}", (x, y)),
                Feature::Framed(true) | Feature::Lathe(_) => {}
                Feature::Framed(false) => assert!(!edge || doorway.contains(&(x, y)), "nothing at {:?}", (x, y)),
                f => panic!("{f:?} at {:?}", (x, y)),
            }
            let solid = map.props(x, y).solid;
            assert_eq!(solid, edge && !doorway.contains(&(x, y)) || matches!(map.feature(x, y), Feature::Lathe(_)), "{:?}", (x, y));
        }
    }
}

#[test]
fn the_doorway_is_in_the_south_wall_wide_enough_for_the_cart() {
    let game = new_game();
    let site = mill_site(&game);
    let (_, _, _, y1) = mill_plot(site);
    let doorway = mill_doorway(site);
    assert_eq!(doorway.len(), 2);
    assert!(doorway.iter().all(|&(_, y)| y == y1));
    assert_eq!(doorway[1].0, doorway[0].0 + 1, "side by side");
}

#[test]
fn the_castaway_walks_in_through_the_doorway() {
    let mut game = walled();
    let site = mill_site(&game);
    let (dx, dy) = mill_doorway(site)[0];
    game.debug_place_player((dx, dy + 2));
    hold(&mut game, 0, 2);
    hold(&mut game, Buttons::UP, 60);
    assert!(game.player_tile().1 < dy, "inside: {:?}", game.player_tile());
}

#[test]
fn the_walls_are_still_up_on_carrying_on() {
    let game = walled();
    let carried_on = Game::debug_continue(&game.debug_save_json()).expect("the save loads");
    assert_eq!(carried_on.mill_stages_built(), 3);
    let (x0, y0, _, _) = mill_plot(mill_site(&carried_on));
    assert_eq!(carried_on.map().feature(x0, y0 + 1), Feature::MillWall);
}

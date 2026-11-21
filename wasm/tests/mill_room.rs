//! Scenario: inside the mill. Once its roof is on, walking in through the
//! door takes the castaway (the screen fading out and in) into its room,
//! kept off the island's south edge, out of sight of it: a floor of tamped
//! earth roomier than the hut looks from outside, the workbench (the bench
//! it was built from) against the north wall and the lathe by the east
//! wall. Out through its door, they're back outside the hut.

mod common;

use common::{hold, new_game, press};
use survival_core::common::input::Buttons;
use survival_core::game::Game;
use survival_core::items::ItemKind;
use survival_core::world::Feature;
use survival_core::world::map::{lathe_cells, mill_doorway};

fn mill_site(game: &Game) -> (i32, i32) {
    game.map().spots.mill.expect("the island keeps ground for a mill")
}

/// The mill built, roof and all.
fn roofed() -> Game {
    let mut game = new_game();
    while game.debug_mill_advance() {}
    assert!(game.map().has_mill_roof());
    game
}

/// Walks up into the mill's door from just outside it, and on in.
fn walk_in(game: &mut Game) {
    let (dx, dy) = mill_doorway(mill_site(game))[0];
    game.debug_place_player((dx, dy + 2));
    hold(game, 0, 2);
    hold(game, Buttons::UP, 40);
    hold(game, 0, 40);
}

/// Inside, walks down out through the room's door.
fn walk_out(game: &mut Game) {
    hold(game, Buttons::DOWN, 40);
    hold(game, 0, 40);
}

#[test]
fn walking_in_through_the_door_takes_the_castaway_into_the_room() {
    let mut game = roofed();
    walk_in(&mut game);
    assert!(game.is_in_mill_room());
    let (x0, y0, x1, y1) = game.map().mill_room();
    let (x, y) = game.player_tile();
    assert!((x0 < x && x < x1) && (y0 < y && y < y1), "on the floor: {:?}", (x, y));
}

#[test]
fn the_room_is_kept_off_the_islands_south_edge() {
    let game = roofed();
    let (_, island_h) = game.map().island_size();
    let (x0, y0, x1, y1) = game.map().mill_room();
    assert!(y0 >= island_h, "south of the island");
    let (rw, rh) = (x1 - x0 - 1, y1 - y0 - 3);
    assert!(rw * rh > 5 * 3, "roomier than the hut looks: {rw}x{rh}");
}

#[test]
fn walking_out_through_the_rooms_door_takes_the_castaway_back_outside_the_hut() {
    let mut game = roofed();
    walk_in(&mut game);
    walk_out(&mut game);
    assert!(!game.is_in_mill_room());
    let (dx, dy) = mill_doorway(mill_site(&game))[0];
    let (x, y) = game.player_tile();
    assert!(y > dy && (dx - 1..=dx + 2).contains(&x), "just outside the door: {:?}", (x, y));
}

#[test]
fn the_screen_fades_out_and_in_on_the_way_through() {
    let mut game = roofed();
    let (dx, dy) = mill_doorway(mill_site(&game))[0];
    game.debug_place_player((dx, dy + 2));
    hold(&mut game, 0, 2);
    let mut darkest = 15;
    for _ in 0..80 {
        game.step(Buttons::UP);
        darkest = darkest.min(game.brightness());
    }
    assert_eq!(darkest, 0, "dark at the turn");
    hold(&mut game, 0, 40);
    assert_eq!(game.brightness(), 15, "light again");
}

#[test]
fn before_the_roof_is_on_the_doorway_is_just_walked_through() {
    let mut game = new_game();
    while game.debug_mill_progress() < 6 {
        assert!(game.debug_mill_advance());
    }
    let (dx, dy) = mill_doorway(mill_site(&game))[0];
    game.debug_place_player((dx, dy + 2));
    hold(&mut game, 0, 2);
    hold(&mut game, Buttons::UP, 60);
    assert!(!game.is_in_mill_room());
    assert!(game.player_tile().1 < dy, "inside the walls: {:?}", game.player_tile());
}

#[test]
fn inside_the_workbench_stands_by_the_north_wall_and_the_lathe_by_the_east_wall() {
    let game = roofed();
    let map = game.map();
    let (x0, y0, x1, _) = map.mill_room();
    let (bx, by) = map.mill_room_bench();
    assert_eq!(map.feature(bx, by), Feature::SiteBench);
    assert_eq!(by, y0 + 2, "against the north wall");
    assert!(bx < (x0 + x1) / 2, "toward the west");
    for (x, y) in lathe_cells(map.mill_room_lathe()) {
        assert!(matches!(map.feature(x, y), Feature::Lathe(_)), "{:?}", (x, y));
        assert!(x >= x1 - 2, "by the east wall");
    }
}

#[test]
fn the_lathe_is_worked_inside() {
    let mut game = roofed();
    let (lx, ly) = game.map().mill_room_lathe();
    game.debug_place_player((lx - 2, ly));
    hold(&mut game, 0, 2);
    assert!(game.is_near_lathe());
}

#[test]
fn sleeping_inside_mends_faster_than_out_in_the_open() {
    /// Health mended in a while of sleep, inside or outside.
    fn mended(inside: bool) -> u32 {
        let mut game = roofed();
        if inside {
            walk_in(&mut game);
        } else {
            let (dx, dy) = mill_doorway(mill_site(&game))[0];
            game.debug_place_player((dx, dy + 4));
            hold(&mut game, 0, 2);
        }
        game.debug_set_hp(10);
        press(&mut game, Buttons::L);
        hold(&mut game, 0, 300);
        game.vitals().hp - 10
    }
    assert!(mended(true) > mended(false));
}

#[test]
fn the_cart_pulled_comes_in_too() {
    let mut game = roofed();
    game.debug_give(ItemKind::Handcart);
    hold(&mut game, 0, 2);
    walk_in(&mut game);
    assert!(game.is_in_mill_room());
    let (cx, cy) = game.cart_at().expect("a cart");
    let (x0, y0, x1, y1) = game.map().mill_room();
    assert!((x0 * 16..=x1 * 16).contains(&cx) && (y0 * 16..=y1 * 16).contains(&cy), "in the room: {:?}", (cx, cy));
}

#[test]
fn carrying_on_from_a_save_inside_starts_inside() {
    let mut game = roofed();
    walk_in(&mut game);
    let carried_on = Game::debug_continue(&game.debug_save_json()).expect("the save loads");
    assert!(carried_on.is_in_mill_room());
    assert_eq!(carried_on.map().feature(carried_on.map().mill_room_bench().0, carried_on.map().mill_room_bench().1), Feature::SiteBench);
}

//! Scenario: the hearth and the oil lamps made at the mill's workbench.
//! The hearth is sunk in the floor between the bed and the south wall.
//! Embers are kept banked in its ash, so fuel laid on it catches without
//! the fire drill, and it takes a week's fuel at once, burning a day on
//! each; while it burns it's a fire to cook by. The lamps hang on the west
//! wall and, once made, light the room at night as by day with no more
//! fuel.

mod common;

use common::{hold, new_game, press};
use survival_core::common::input::Buttons;
use survival_core::game::Game;
use survival_core::game::mill::{Furnishing, HEARTH_FUEL_DAYS};
use survival_core::items::ItemKind;
use survival_core::world::Feature;

/// Frames laying a piece of fuel on the hearth takes (with a little to
/// spare).
const STOKING: u32 = 60;

/// The mill built and fitted out with `f`, the castaway in its room.
fn furnished_with(f: Furnishing) -> Game {
    let mut game = new_game();
    while game.debug_mill_advance() {}
    assert!(game.debug_furnish(f));
    game
}

/// The hearth made, the castaway just south of it, facing it.
fn at_the_hearth() -> Game {
    let mut game = furnished_with(Furnishing::Hearth);
    let cells = game.map().furnishing_cells(Furnishing::Hearth);
    let (x, y) = *cells.iter().max_by_key(|c| (c.1, -c.0)).unwrap();
    game.debug_place_player((x, y + 1));
    hold(&mut game, 0, 2);
    hold(&mut game, Buttons::UP, 2);
    hold(&mut game, 0, 2);
    assert!(game.is_in_mill_room());
    game
}

/// A at the hearth, and the castaway lays on what's to burn.
fn stoke(game: &mut Game) {
    press(game, Buttons::A);
    hold(game, 0, STOKING);
}

#[test]
fn the_hearth_is_sunk_in_the_floor_midway_between_the_bed_and_the_south_wall() {
    let game = furnished_with(Furnishing::Hearth);
    let map = game.map();
    let hearth = map.furnishing_cells(Furnishing::Hearth);
    for &(x, y) in &hearth {
        assert!(matches!(map.feature(x, y), Feature::Hearth(_)), "{:?}", (x, y));
    }
    let bed = map.furnishing_cells(Furnishing::Bed);
    let columns = |cells: &[(i32, i32)]| {
        let mut xs: Vec<i32> = cells.iter().map(|c| c.0).collect();
        xs.sort();
        xs.dedup();
        xs
    };
    assert_eq!(columns(&hearth), columns(&bed), "under the bed");
    let (_, _, _, y1) = map.mill_room();
    let south_wall = y1 - 1;
    let bed_foot = bed.iter().map(|c| c.1).max().unwrap();
    let (top, bottom) = (hearth.iter().map(|c| c.1).min().unwrap(), hearth.iter().map(|c| c.1).max().unwrap());
    let (above, below) = (top - bed_foot - 1, south_wall - bottom - 1);
    assert!(above > 0 && below > 0);
    assert!((above - below).abs() <= 1, "midway: {above} rows above, {below} below");
}

#[test]
fn the_hearth_is_in_the_way() {
    let game = furnished_with(Furnishing::Hearth);
    let (x, y) = game.map().furnishing_cells(Furnishing::Hearth)[0];
    assert!(game.map().props(x, y).solid);
}

#[test]
fn fuel_laid_on_the_hearth_catches_without_the_fire_drill() {
    let mut game = at_the_hearth();
    game.debug_give_n(ItemKind::Branch, 1);
    assert!(!game.carries(ItemKind::FireDrill));
    stoke(&mut game);
    assert_eq!(game.carried(ItemKind::Branch), 0, "burnt");
    assert!(game.is_by_a_fire());
}

#[test]
fn a_pine_cone_goes_on_the_hearth_before_a_branch() {
    let mut game = at_the_hearth();
    game.debug_give_n(ItemKind::Branch, 1);
    game.debug_give_n(ItemKind::PineCone, 1);
    stoke(&mut game);
    assert_eq!(game.carried(ItemKind::PineCone), 0);
    assert_eq!(game.carried(ItemKind::Branch), 1);
}

#[test]
fn with_nothing_to_burn_the_castaway_stops_to_think_at_the_hearth() {
    let mut game = at_the_hearth();
    game.debug_give(ItemKind::FireDrill);
    press(&mut game, Buttons::A);
    assert!(game.is_pondering());
    hold(&mut game, 0, STOKING);
    assert!(!game.is_by_a_fire());
}

#[test]
fn each_piece_of_fuel_keeps_the_hearth_burning_a_day_more() {
    let mut game = at_the_hearth();
    game.debug_give_n(ItemKind::Branch, 3);
    for _ in 0..3 {
        stoke(&mut game);
    }
    game.debug_advance_hours(2 * 24 + 20);
    assert!(game.is_by_a_fire(), "still burning on the third day");
    game.debug_advance_hours(6);
    assert!(!game.is_by_a_fire(), "gone out after three days");
}

#[test]
fn the_hearth_takes_no_more_than_a_weeks_fuel_at_once() {
    let mut game = at_the_hearth();
    let given = HEARTH_FUEL_DAYS + 3;
    game.debug_give_n(ItemKind::Branch, given);
    for _ in 0..given {
        stoke(&mut game);
    }
    assert_eq!(game.carried(ItemKind::Branch), 3, "the rest is kept");
}

#[test]
fn gone_out_the_hearth_catches_again_from_its_embers() {
    let mut game = at_the_hearth();
    game.debug_give_n(ItemKind::Branch, 2);
    stoke(&mut game);
    game.debug_advance_hours(25);
    assert!(!game.is_by_a_fire());
    stoke(&mut game);
    assert!(game.is_by_a_fire());
}

#[test]
fn fish_is_grilled_at_the_burning_hearth() {
    let mut game = at_the_hearth();
    game.debug_give_n(ItemKind::PineCone, 1);
    stoke(&mut game);
    game.debug_give(ItemKind::Fish);
    game.debug_give(ItemKind::StoneKnife);
    hold(&mut game, 0, 2);
    // The bag's CRAFT page, on its only recipe (grilled fish): make it.
    press(&mut game, Buttons::X);
    press(&mut game, Buttons::UP);
    press(&mut game, Buttons::RIGHT);
    press(&mut game, Buttons::A);
    press(&mut game, Buttons::A);
    press(&mut game, Buttons::A);
    press(&mut game, Buttons::A);
    hold(&mut game, 0, 300);
    assert_eq!(game.carried(ItemKind::Fish), 0);
    assert_eq!(game.carried(ItemKind::GrilledFish), 1);
}

#[test]
fn the_hearths_fire_is_kept_on_carrying_on_from_a_save() {
    let mut game = at_the_hearth();
    game.debug_give_n(ItemKind::Branch, 2);
    stoke(&mut game);
    stoke(&mut game);
    let mut carried_on = Game::debug_continue(&game.debug_save_json()).expect("the save loads");
    hold(&mut carried_on, 0, 2);
    assert!(carried_on.is_by_a_fire());
    carried_on.debug_advance_hours(30);
    assert!(carried_on.is_by_a_fire(), "the second day's fuel is kept too");
}

/// How bright the frame drawn is, on average (0-255 a channel).
fn brightness(game: &mut Game) -> f32 {
    game.render();
    let px = game.rgba();
    px.chunks(4).map(|p| (p[0] as u32 + p[1] as u32 + p[2] as u32) as f32 / 3.0).sum::<f32>() / (px.len() / 4) as f32
}

/// The castaway in the middle of the mill's room, with `lamps` made or
/// not, at noon or at night.
fn in_the_room(lamps: bool, night: bool) -> Game {
    let mut game = new_game();
    while game.debug_mill_advance() {}
    if lamps {
        assert!(game.debug_furnish(Furnishing::Lamps));
    }
    let (x0, y0, x1, y1) = game.map().mill_room();
    game.debug_place_player(((x0 + x1) / 2, (y0 + y1) / 2));
    let hours_to = |hour: u32| (hour + 24 - game.clock().hour_minute().0) % 24;
    game.debug_advance_hours(hours_to(if night { 22 } else { 12 }));
    hold(&mut game, 0, 2);
    game
}

#[test]
fn the_lamps_light_the_room_at_night_as_by_day() {
    let by_day = brightness(&mut in_the_room(false, false));
    let dark = brightness(&mut in_the_room(false, true));
    let lamplit = brightness(&mut in_the_room(true, true));
    assert!(dark < by_day * 0.5, "without the lamps it's dark at night");
    assert!(lamplit > by_day * 0.9, "lamplit {lamplit} by day {by_day}");
}

#[test]
fn the_lamps_stand_out_from_the_west_wall_in_the_way() {
    let game = furnished_with(Furnishing::Lamps);
    let map = game.map();
    let (x0, _, _, _) = map.mill_room();
    let lamps = map.furnishing_cells(Furnishing::Lamps);
    assert!(!lamps.is_empty());
    for (x, y) in lamps {
        assert_eq!(x, x0 + 1, "along the west wall");
        assert!(matches!(map.feature(x, y), Feature::Lamp(_)), "{:?}", (x, y));
        assert!(map.props(x, y).solid, "no walking under them");
    }
}

#[test]
fn the_lamps_take_fish_oil_and_sea_shells() {
    let needs = Furnishing::Lamps.needs();
    assert!(needs.iter().any(|&(k, _)| k == ItemKind::FishOil));
    assert!(needs.iter().any(|&(k, _)| k == ItemKind::SeaShell));
}

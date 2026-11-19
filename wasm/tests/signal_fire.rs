//! Scenario: the signal fire. Set on open ground, it's lit with the fire
//! drill, burning a branch and a palm frond, and sends up smoke for a day.
//! Once a day the smoke may be seen by a passing plane or ship, and the
//! castaway is rescued.

mod common;

use common::{hold, new_game, press, save_op_within};
use survival_core::common::input::Buttons;
use survival_core::game::{Game, SaveOp};
use survival_core::items::ItemKind;
use survival_core::world::{Feature, Terrain};

/// Frames lighting takes, from A to the fire catching and the castaway
/// getting up again (with a little to spare).
const LIGHTING: u32 = 150;

/// A cell of open, low ground near where the castaway washed up, with
/// open ground round it (two cells to the east, a cell above and below).
fn open_ground(game: &Game) -> (i32, i32) {
    let map = game.map();
    let start = game.player_tile();
    let open = |x: i32, y: i32| {
        map.in_bounds(x, y) && map.terrain(x, y).is_land() && map.terrain(x, y) != Terrain::Cliff && !map.is_high(x, y) && map.feature(x, y) == Feature::None
    };
    (1..40)
        .flat_map(|r: i32| (-r..=r).flat_map(move |dy| (-r..=r).map(move |dx| (start.0 + dx, start.1 + dy))))
        .find(|&(x, y)| (-1..=2).all(|dx| (-1..=1).all(|dy| open(x + dx, y + dy))))
        .expect("open ground near the beach")
}

/// A game with the castaway on open ground, facing east across more of it.
fn on_open_ground() -> Game {
    let mut game = new_game();
    let at = open_ground(&game);
    game.debug_place_player(at);
    hold(&mut game, Buttons::RIGHT, 1);
    hold(&mut game, 0, 2);
    game
}

/// Where a signal fire stands on the island, if one does.
fn signal_fire(game: &Game) -> Option<(i32, i32)> {
    let map = game.map();
    (0..map.height()).flat_map(|y| (0..map.width()).map(move |x| (x, y))).find(|&(x, y)| matches!(map.feature(x, y), Feature::SignalFire(_)))
}

/// Sets a signal fire held in hand on the ground ahead.
fn set_one(game: &mut Game) {
    game.debug_give(ItemKind::SignalFire);
    press(game, Buttons::B);
    hold(game, 0, 60);
}

/// What it takes to light one: the drill, a branch and a frond.
fn give_kindling(game: &mut Game) {
    game.debug_give(ItemKind::FireDrill);
    game.debug_give_n(ItemKind::Branch, 1);
    game.debug_give_n(ItemKind::PalmFrond, 1);
}

/// Lights the signal fire ahead (A), and waits for it to catch.
fn light(game: &mut Game) {
    press(game, Buttons::A);
    hold(game, 0, LIGHTING);
}

/// A game with a signal fire set on open ground just ahead, and what it
/// takes to light it carried.
fn ready_to_light() -> Game {
    let mut game = on_open_ground();
    set_one(&mut game);
    give_kindling(&mut game);
    game
}

/// Lets a day go by (time hurried on, as in sleep), a frame at a time
/// an hour apart, so what's due in it comes about.
fn a_day_goes_by(game: &mut Game) {
    for _ in 0..25 {
        game.debug_advance_hours(1);
        hold(game, 0, 1);
    }
}

#[test]
fn a_signal_fire_is_made_from_stones_timber_and_soil() {
    let recipe = survival_core::crafting::RECIPES.iter().find(|r| r.makes == ItemKind::SignalFire).expect("a recipe for it");
    let mut needs: Vec<ItemKind> = recipe.needs.iter().map(|&(k, _)| k).collect();
    needs.sort_by_key(|k| format!("{k:?}"));
    assert_eq!(needs, [ItemKind::Soil, ItemKind::Stone, ItemKind::Timber]);
    assert!(recipe.tools.is_empty() && !recipe.at_fire && !recipe.at_lathe, "made anywhere, with bare hands");
}

#[test]
fn a_signal_fire_held_in_hand_is_set_on_the_ground_ahead() {
    let mut game = on_open_ground();
    let at = game.player_tile();
    set_one(&mut game);
    assert_eq!(signal_fire(&game), Some((at.0 + 1, at.1)), "it stands just ahead");
    assert!(!game.carries(ItemKind::SignalFire), "it is out of the bag");
}

#[test]
fn a_signal_fire_set_down_can_be_walked_through_like_a_campfire() {
    let mut game = on_open_ground();
    set_one(&mut game);
    let fire = signal_fire(&game).expect("set down");
    let mut stood_in_it = false;
    for _ in 0..60 {
        hold(&mut game, Buttons::RIGHT, 1);
        stood_in_it |= game.player_tile() == fire;
    }
    assert!(stood_in_it, "nothing stops the castaway walking over it");
}

#[test]
fn a_signal_fire_is_not_set_in_the_water() {
    let mut game = new_game();
    let map = game.map();
    // A cell of beach with the sea just east of it.
    let shore = (0..map.height())
        .flat_map(|y| (0..map.width()).map(move |x| (x, y)))
        .find(|&(x, y)| map.terrain(x, y) == Terrain::Sand && map.feature(x, y) == Feature::None && map.terrain(x + 1, y).is_water() && !map.is_high(x, y))
        .expect("a beach by the sea");
    game.debug_place_player(shore);
    hold(&mut game, Buttons::RIGHT, 1);
    hold(&mut game, 0, 2);
    set_one(&mut game);
    assert_eq!(signal_fire(&game), None);
    assert!(game.carries(ItemKind::SignalFire));
}

#[test]
fn lighting_the_signal_fire_burns_a_branch_and_a_frond_and_keeps_the_drill() {
    let mut game = ready_to_light();
    light(&mut game);
    assert_eq!(game.burning_signal_fires(), 1, "smoke rises from it");
    assert_eq!(game.carried(ItemKind::Branch), 0);
    assert_eq!(game.carried(ItemKind::PalmFrond), 0);
    assert!(game.carries(ItemKind::FireDrill));
}

#[test]
fn without_the_drill_a_branch_or_a_frond_the_castaway_stops_to_think() {
    for missing in [ItemKind::FireDrill, ItemKind::Branch, ItemKind::PalmFrond] {
        let mut game = on_open_ground();
        set_one(&mut game);
        for kind in [ItemKind::FireDrill, ItemKind::Branch, ItemKind::PalmFrond].into_iter().filter(|&k| k != missing) {
            game.debug_give_n(kind, 1);
        }
        press(&mut game, Buttons::A);
        assert!(game.is_pondering(), "without {missing:?}");
        hold(&mut game, 0, LIGHTING);
        assert_eq!(game.burning_signal_fires(), 0, "without {missing:?}");
    }
}

#[test]
fn a_signal_fire_already_burning_takes_no_more() {
    let mut game = ready_to_light();
    light(&mut game);
    give_kindling(&mut game);
    light(&mut game);
    assert_eq!(game.carried(ItemKind::Branch), 1, "nothing more burnt");
    assert_eq!(game.carried(ItemKind::PalmFrond), 1);
    assert!(!game.is_pondering());
}

#[test]
fn a_signal_fire_burns_for_a_day_and_can_be_lit_again() {
    let mut game = ready_to_light();
    game.debug_rescue_luck(Some(false));
    light(&mut game);
    a_day_goes_by(&mut game);
    assert_eq!(game.burning_signal_fires(), 0, "burnt out");
    assert!(signal_fire(&game).is_some(), "the hearth stays");
    give_kindling(&mut game);
    light(&mut game);
    assert_eq!(game.burning_signal_fires(), 1);
}

#[test]
fn a_burning_signal_fire_is_no_fire_to_cook_by() {
    let mut game = ready_to_light();
    light(&mut game);
    assert!(!game.is_by_a_fire(), "too smoky to cook over");
}

#[test]
fn smoke_seen_by_luck_brings_rescue_while_it_rises() {
    let mut game = ready_to_light();
    game.debug_rescue_luck(Some(true));
    light(&mut game);
    assert!(!game.is_rescued(), "not the moment it's lit");
    a_day_goes_by(&mut game);
    assert!(game.is_rescued());
}

#[test]
fn smoke_not_seen_brings_no_rescue() {
    let mut game = ready_to_light();
    game.debug_rescue_luck(Some(false));
    light(&mut game);
    a_day_goes_by(&mut game);
    assert!(!game.is_rescued());
}

#[test]
fn the_smoke_has_one_chance_a_day_however_many_fires_burn() {
    let mut game = ready_to_light();
    game.debug_rescue_luck(Some(false));
    light(&mut game);
    // A second signal fire, a row down, lit the same day.
    let at = game.player_tile();
    game.debug_place_player((at.0, at.1 + 1));
    hold(&mut game, Buttons::RIGHT, 1);
    hold(&mut game, 0, 2);
    set_one(&mut game);
    give_kindling(&mut game);
    game.debug_rescue_luck(Some(true));
    light(&mut game);
    assert_eq!(game.burning_signal_fires(), 2);
    a_day_goes_by(&mut game);
    assert!(!game.is_rescued(), "the day's chance was already had");
}

#[test]
fn the_next_day_brings_another_chance() {
    let mut game = ready_to_light();
    game.debug_rescue_luck(Some(false));
    light(&mut game);
    a_day_goes_by(&mut game);
    give_kindling(&mut game);
    game.debug_rescue_luck(Some(true));
    light(&mut game);
    a_day_goes_by(&mut game);
    assert!(game.is_rescued());
}

#[test]
fn being_rescued_erases_the_saved_game() {
    let mut game = ready_to_light();
    game.debug_rescue_luck(Some(true));
    light(&mut game);
    for _ in 0..25 {
        game.debug_advance_hours(1);
        if let Some(op) = save_op_within(&mut game, 1)
            && game.is_rescued()
        {
            assert_eq!(op, SaveOp::Erase);
            return;
        }
    }
    panic!("rescued, the save erased");
}

#[test]
fn a_burning_signal_fire_is_saved_and_its_luck_with_it() {
    for lucky in [true, false] {
        let mut game = ready_to_light();
        game.debug_rescue_luck(Some(lucky));
        light(&mut game);
        let mut carried_on = Game::debug_continue(&game.debug_save_json()).expect("the save loads");
        assert_eq!(carried_on.burning_signal_fires(), 1, "still smoking");
        // The luck was settled as it was lit: loading again won't change it.
        carried_on.debug_rescue_luck(Some(!lucky));
        a_day_goes_by(&mut carried_on);
        assert_eq!(carried_on.is_rescued(), lucky);
    }
}

#[test]
fn the_end_shows_the_days_it_took_and_what_was_found_and_made() {
    let mut game = ready_to_light();
    game.debug_rescue_luck(Some(true));
    light(&mut game);
    a_day_goes_by(&mut game);
    let r = game.records();
    assert_eq!(r.day, game.clock().day);
    assert!(r.recipes_known <= r.recipes && r.made <= r.makeable);
}

/// Lets `hours` go by, an hour at a time.
fn hours_go_by(game: &mut Game, hours: u32) {
    for _ in 0..hours {
        game.debug_advance_hours(1);
        hold(game, 0, 1);
    }
}

#[test]
fn with_rescue_made_sure_a_signal_fire_lit_brings_rescue_within_hours() {
    let mut game = ready_to_light();
    game.debug_sure_rescue();
    light(&mut game);
    assert!(!game.is_rescued());
    hours_go_by(&mut game, 2);
    assert!(game.is_rescued());
}

#[test]
fn making_rescue_sure_counts_for_a_signal_fire_already_burning_unseen() {
    let mut game = ready_to_light();
    game.debug_rescue_luck(Some(false));
    light(&mut game);
    game.debug_sure_rescue();
    hours_go_by(&mut game, 2);
    assert!(game.is_rescued());
}

/// The middle of a signal fire's cell (world px).
fn middle((x, y): (i32, i32)) -> (i32, i32) {
    (x * 16 + 8, y * 16 + 8)
}

/// A signal fire set on open ground, the castaway moved below it, a
/// little off to the left, facing it: by the edge of its stones.
fn by_the_edge_of_it() -> Game {
    let mut game = ready_to_light();
    let fire = signal_fire(&game).expect("set down");
    game.debug_place_player((fire.0, fire.1 + 1));
    let (mx, _) = middle(fire);
    while game.player_feet().0 > mx - 10 {
        hold(&mut game, Buttons::LEFT, 1);
    }
    hold(&mut game, Buttons::UP, 1);
    hold(&mut game, 0, 2);
    game
}

#[test]
fn a_signal_fire_is_lit_from_by_the_edge_of_its_stones() {
    let mut game = by_the_edge_of_it();
    let fire = signal_fire(&game).unwrap();
    let off = game.player_feet().0 - middle(fire).0;
    assert!((-14..=-8).contains(&off), "off to one side ({off} px)");
    light(&mut game);
    assert_eq!(game.burning_signal_fires(), 1);
}

#[test]
fn a_signal_fire_is_lit_standing_over_it() {
    let mut game = ready_to_light();
    let fire = signal_fire(&game).unwrap();
    game.debug_place_player(fire);
    hold(&mut game, 0, 2);
    light(&mut game);
    assert_eq!(game.burning_signal_fires(), 1);
}

#[test]
fn a_signal_fire_a_few_cells_off_is_out_of_reach() {
    let mut game = ready_to_light();
    let at = game.player_tile();
    game.debug_place_player((at.0 - 2, at.1));
    hold(&mut game, Buttons::RIGHT, 1);
    hold(&mut game, 0, 2);
    press(&mut game, Buttons::A);
    assert!(!game.is_pondering());
    hold(&mut game, 0, LIGHTING);
    assert_eq!(game.burning_signal_fires(), 0);
}

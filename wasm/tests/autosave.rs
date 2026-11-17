//! Scenario: the game saves itself when something worth keeping has
//! happened and the castaway is calm (docs/save.md).

mod common;

use common::{hold, new_game, press};
use survival_core::common::input::Buttons;
use survival_core::game::{Game, SaveOp};
use survival_core::items::ItemKind;

/// The saves written over `frames` frames with `buttons` held.
fn saves_while(game: &mut Game, buttons: u16, frames: u32) -> Vec<serde_json::Value> {
    let mut saves = Vec::new();
    for _ in 0..frames {
        game.step(buttons);
        match game.take_save_op() {
            Some(SaveOp::Write(json)) => saves.push(serde_json::from_str(&json).unwrap()),
            Some(SaveOp::Erase) => panic!("erased"),
            None => {}
        }
    }
    saves
}

/// A game in which nothing waits to be saved.
fn settled() -> Game {
    let mut game = new_game();
    hold(&mut game, 0, 600);
    game.take_save_op();
    game
}

#[test]
fn standing_about_is_not_saved() {
    let mut game = settled();
    assert!(saves_while(&mut game, 0, 600).is_empty());
}

#[test]
fn walking_is_not_saved_while_walking() {
    let mut game = settled();
    assert!(saves_while(&mut game, Buttons::RIGHT, 300).is_empty());
}

#[test]
fn walking_is_saved_once_the_castaway_stands_still() {
    let mut game = settled();
    hold(&mut game, Buttons::RIGHT, 60);
    let saves = saves_while(&mut game, 0, 300);
    assert_eq!(saves.len(), 1);
    let at = game.debug_save_json();
    let at: serde_json::Value = serde_json::from_str(&at).unwrap();
    assert_eq!(saves[0]["castaway"]["at"], at["castaway"]["at"], "where they stopped");
}

#[test]
fn something_eaten_is_saved_once_it_is_eaten() {
    let mut game = settled();
    game.debug_set_vitals(None, Some(10), None);
    game.debug_give(ItemKind::OpenedCoconut);
    press(&mut game, Buttons::X);
    press(&mut game, Buttons::A);
    press(&mut game, Buttons::A);
    let saves = saves_while(&mut game, 0, 300);
    assert_eq!(saves.len(), 1);
    assert!(saves[0]["vitals"]["food"].as_u64().unwrap() > 10, "saved having eaten it, not halfway");
    assert!(!saves[0]["inventory"]["bag"]["items"].as_array().unwrap().iter().any(|p| p["kind"] == "OpenedCoconut"));
}

#[test]
fn something_made_in_the_bag_is_saved_when_the_bag_is_closed() {
    let mut game = settled();
    game.debug_give_n(ItemKind::Husk, 2);
    // Open the first recipe of the CRAFT page and make it.
    press(&mut game, Buttons::X);
    press(&mut game, Buttons::UP);
    press(&mut game, Buttons::RIGHT);
    for _ in 0..4 {
        press(&mut game, Buttons::A);
    }
    assert!(saves_while(&mut game, 0, 300).is_empty(), "not while the bag is open");
    press(&mut game, Buttons::X);
    let saves = saves_while(&mut game, 0, 10);
    assert_eq!(saves.len(), 1);
    assert!(saves[0]["inventory"]["bag"]["items"].as_array().unwrap().iter().any(|p| p["kind"] == "Rope"));
}

#[test]
fn opening_and_closing_the_bag_without_changing_anything_is_not_saved() {
    let mut game = settled();
    press(&mut game, Buttons::X);
    press(&mut game, Buttons::X);
    assert!(saves_while(&mut game, 0, 300).is_empty());
}

#[test]
fn a_long_sleep_is_saved_along_the_way_and_on_waking() {
    let mut game = settled();
    press(&mut game, Buttons::L);
    let asleep = saves_while(&mut game, 0, 240);
    assert!(asleep.len() >= 2, "saved {} times in eight hours of sleep", asleep.len());
    press(&mut game, Buttons::L);
    assert_eq!(saves_while(&mut game, 0, 300).len(), 1, "and on waking");
}

#[test]
fn flushing_saves_what_waits_to_be_saved() {
    let mut game = settled();
    hold(&mut game, Buttons::RIGHT, 60);
    game.flush_save();
    assert!(matches!(game.take_save_op(), Some(SaveOp::Write(_))));
    game.flush_save();
    assert_eq!(game.take_save_op(), None, "nothing more waits");
}

#[test]
fn nothing_is_saved_after_collapsing() {
    let mut game = settled();
    hold(&mut game, Buttons::RIGHT, 60);
    game.debug_set_vitals(Some(0), None, None);
    let mut erased = false;
    for _ in 0..600 {
        game.step(0);
        game.flush_save();
        match game.take_save_op() {
            Some(SaveOp::Erase) => erased = true,
            Some(SaveOp::Write(_)) => panic!("saved after collapsing"),
            None => {}
        }
    }
    assert!(erased);
}

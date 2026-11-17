//! Lying down to sleep and waking up, by the buttons that do it.

mod common;

use common::{hold, new_game, press, save_op_within};
use survival_core::common::input::Buttons;
use survival_core::game::SaveOp;

/// Game minutes that pass in the second after pressing `button` (0: nothing).
fn minutes_after_pressing(button: u16) -> u32 {
    let mut game = new_game();
    let start = game.clock().total_minutes();
    press(&mut game, button);
    hold(&mut game, 0, 60);
    game.clock().total_minutes() - start
}

#[test]
fn l_lies_down_to_sleep() {
    // Asleep, time runs far faster than awake.
    assert!(minutes_after_pressing(Buttons::L) > 4 * minutes_after_pressing(0));
}

#[test]
fn y_does_not_lie_down_to_sleep() {
    assert_eq!(minutes_after_pressing(Buttons::Y), minutes_after_pressing(0));
}

/// A game with the castaway settled into sleep, nothing yet to save.
fn asleep() -> survival_core::game::Game {
    let mut game = new_game();
    press(&mut game, Buttons::L);
    hold(&mut game, 0, 60);
    game.take_save_op();
    game
}

#[test]
fn l_wakes_up_and_saves() {
    let mut game = asleep();
    press(&mut game, Buttons::L);
    assert!(matches!(save_op_within(&mut game, 300), Some(SaveOp::Write(_))));
}

#[test]
fn a_and_b_still_wake_up() {
    for button in [Buttons::A, Buttons::B] {
        let mut game = asleep();
        press(&mut game, button);
        assert!(matches!(save_op_within(&mut game, 300), Some(SaveOp::Write(_))));
    }
}

#[test]
fn y_does_not_wake_up() {
    let mut game = asleep();
    press(&mut game, Buttons::Y);
    // Still asleep, time runs far faster than awake.
    let start = game.clock().total_minutes();
    hold(&mut game, 0, 60);
    assert!(game.clock().total_minutes() - start > 4 * minutes_after_pressing(0));
}

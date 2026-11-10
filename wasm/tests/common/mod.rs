//! Helpers for the scenario tests: games set up past the title, and the
//! joypad pressed for them.

#![allow(dead_code)]

use survival_core::common::input::Buttons;
use survival_core::game::{Game, SaveOp};

/// The seed the island is made from in the tests.
pub const SEED: u32 = 1;

/// A new game, past the title, the castaway already up on their feet
/// where they washed up (a new game opens with them lying there, waking).
pub fn new_game() -> Game {
    let mut game = Game::new(SEED);
    let washed_up = game.player_tile();
    game.debug_place_player(washed_up);
    game
}

/// Holds `buttons` down for `frames` frames.
pub fn hold(game: &mut Game, buttons: u16, frames: u32) {
    for _ in 0..frames {
        game.step(buttons);
    }
}

/// Presses `buttons` and lets go (a press counts on the frame it starts).
pub fn press(game: &mut Game, buttons: u16) {
    game.step(buttons);
    game.step(0);
}

/// Some walking about: right, then down, then left, with pauses.
pub fn walk_about(game: &mut Game) {
    hold(game, Buttons::RIGHT, 120);
    hold(game, 0, 20);
    hold(game, Buttons::DOWN, 60);
    hold(game, Buttons::LEFT, 90);
    hold(game, 0, 30);
}

/// What the game asked to do with the saved game over `frames` frames, the
/// last of them if several.
pub fn save_op_within(game: &mut Game, frames: u32) -> Option<SaveOp> {
    let mut last = None;
    for _ in 0..frames {
        game.step(0);
        if let Some(op) = game.take_save_op() {
            last = Some(op);
        }
    }
    last
}

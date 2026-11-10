//! Scenario: when health runs out the castaway collapses, and the saved
//! game goes with them.

mod common;

use common::{new_game, save_op_within};
use survival_core::game::SaveOp;

#[test]
fn collapsing_erases_the_saved_game() {
    let mut game = new_game();
    game.debug_set_vitals(Some(0), None, None);
    assert!(matches!(save_op_within(&mut game, 60), Some(SaveOp::Erase)));
}

#[test]
fn nothing_is_saved_or_erased_while_just_walking_about() {
    let mut game = new_game();
    assert!(save_op_within(&mut game, 120).is_none());
}

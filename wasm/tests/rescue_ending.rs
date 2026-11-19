//! Scenario: the rescue's ending. The world fades out; three cuts follow,
//! one after another (the island from afar with its smoke, the view from
//! a passing plane's cockpit, the plane turning toward the island); then
//! the records. START skips the cuts; on the records it begins again.

mod common;

use common::{hold, new_game, press};
use survival_core::common::input::Buttons;
use survival_core::game::{EndingView, Game};

/// A game whose castaway has just been rescued.
fn rescued() -> Game {
    let mut game = new_game();
    game.debug_rescue_now();
    game
}

/// Steps on until the ending shows `view` (or gives up), returning
/// whether it did.
fn until(game: &mut Game, view: EndingView, frames: u32) -> bool {
    for _ in 0..frames {
        if game.ending_view() == Some(view) {
            return true;
        }
        hold(game, 0, 1);
    }
    game.ending_view() == Some(view)
}

#[test]
fn the_three_cuts_play_in_order_then_the_records() {
    let mut game = rescued();
    let mut seen = Vec::new();
    for _ in 0..3000 {
        let view = game.ending_view().expect("the ending");
        if seen.last() != Some(&view) {
            seen.push(view);
        }
        hold(&mut game, 0, 1);
    }
    assert_eq!(seen, [EndingView::FadingOut, EndingView::Cut(0), EndingView::Cut(1), EndingView::Cut(2), EndingView::Records]);
}

#[test]
fn start_skips_the_cuts_to_the_records() {
    let mut game = rescued();
    assert!(until(&mut game, EndingView::Cut(0), 600));
    press(&mut game, Buttons::START);
    assert_eq!(game.ending_view(), Some(EndingView::Records));
}

#[test]
fn start_on_the_records_begins_again_at_the_title() {
    let mut game = rescued();
    assert!(until(&mut game, EndingView::Cut(0), 600));
    press(&mut game, Buttons::START);
    hold(&mut game, 0, 120);
    press(&mut game, Buttons::START);
    assert_eq!(game.ending_view(), None);
    assert!(game.is_at_title());
}

#[test]
fn collapsing_shows_no_cuts() {
    let mut game = new_game();
    game.debug_set_hp(0);
    hold(&mut game, 0, 2);
    assert_eq!(game.ending_view(), None, "only escapes have the cuts and records");
}

//! Scenario: how the castaway spends the day tells on their health.

mod common;

use common::{hold, new_game};
use survival_core::common::input::Buttons;

#[test]
fn running_about_wears_health_faster_than_walking_about() {
    // Long enough for walking to wear a little health away.
    let frames = 4000;
    let mut walker = new_game();
    let mut runner = new_game();
    hold(&mut walker, Buttons::UP, frames);
    hold(&mut runner, Buttons::UP | Buttons::R, frames);
    assert!(walker.vitals().hp < walker.vitals().max_hp, "walking wears some health away");
    assert!(runner.vitals().hp < walker.vitals().hp);
}

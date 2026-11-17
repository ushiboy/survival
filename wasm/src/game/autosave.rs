//! When the game saves itself (see docs/save.md).
//!
//! The game tells `AutoSave` what has happened (`note`) and, once a frame
//! of the world, whether the castaway is calm (`tick`); `tick` answers
//! whether to save now. Every save goes through here, so where saves come
//! from stays in one place: what a deed means for saving is in
//! `cue_of`, and a new deed can't be added without deciding it.

use crate::entity::player::Deed;

/// Frames the castaway stands still before a walk is saved.
const STILL_FRAMES: u32 = 60;
/// Frames that must pass between saves (things happening close together
/// are saved once). Waking, sleeping and closing the bag don't wait.
const MIN_GAP_FRAMES: u32 = 3 * 60;
/// Frames anything is left unsaved before it is saved at the next calm
/// moment, however it came about.
const FALLBACK_FRAMES: u32 = 60 * 60;
/// Game minutes of sleep between saves while asleep.
pub const SLEEP_SAVE_MINUTES: u32 = 2 * 60;

/// Something that happened, as it bears on saving.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cue {
    /// Something to keep changed (an item got or used, the island
    /// changed): saved at the next calm moment, not too soon after the
    /// last save.
    Changed,
    /// The castaway moved: saved once they have stood still a while.
    Moved,
    /// The bag was closed with something in it changed: saved at the next
    /// calm moment, without waiting.
    BagClosed,
    /// Saved straight away (waking, a stretch of sleep).
    Now,
}

/// What a deed means for saving.
pub fn cue_of(deed: &Deed) -> Option<Cue> {
    match deed {
        Deed::Chop(_)
        | Deed::Dig(_)
        | Deed::Thrust(..)
        | Deed::Shake(_)
        | Deed::PickUp
        | Deed::Pull(_)
        | Deed::DrawWater
        | Deed::Drink(_)
        | Deed::Eat(_)
        | Deed::LightFire(_)
        | Deed::Cooked(_)
        | Deed::Strike
        | Deed::StandLadder(..)
        | Deed::LayBridge(_)
        | Deed::SetWheel(_)
        | Deed::SetLathe(_) => Some(Cue::Changed),
        // Passing: nothing is kept of them.
        Deed::Smoulder(_) | Deed::KickDust(_) | Deed::Cast(_) | Deed::ReelIn => None,
    }
}

#[derive(Clone, Debug, Default)]
pub struct AutoSave {
    changed: bool,
    moved: bool,
    bag_closed: bool,
    now: bool,
    /// Frames since the last save (counted only while something waits).
    since_save: u32,
    /// Frames since something first went unsaved.
    unsaved_for: u32,
    /// Frames the castaway has been calm.
    calm_for: u32,
    /// Game minutes asleep since the last save in sleep.
    slept: u32,
}

impl AutoSave {
    /// A game just saved (or carried on): nothing waits.
    pub fn new() -> Self {
        Self { since_save: MIN_GAP_FRAMES, ..Self::default() }
    }

    pub fn note(&mut self, cue: Cue) {
        match cue {
            Cue::Changed => self.changed = true,
            Cue::Moved => self.moved = true,
            Cue::BagClosed => self.bag_closed = true,
            Cue::Now => self.now = true,
        }
    }

    /// `minutes` of sleep went by: every `SLEEP_SAVE_MINUTES` is saved.
    pub fn slept(&mut self, minutes: u32) {
        self.slept += minutes;
        if self.slept >= SLEEP_SAVE_MINUTES {
            self.slept -= SLEEP_SAVE_MINUTES;
            self.now = true;
        }
    }

    /// Woke up: saved straight away.
    pub fn woke(&mut self) {
        self.slept = 0;
        self.now = true;
    }

    /// Something waits to be saved.
    pub fn pending(&self) -> bool {
        self.changed || self.moved || self.bag_closed || self.now
    }

    /// One frame of the world, the castaway `calm` (standing, not in the
    /// middle of anything) or not: whether to save now. Saying yes counts
    /// as saving.
    pub fn tick(&mut self, calm: bool) -> bool {
        self.since_save = self.since_save.saturating_add(1);
        self.calm_for = if calm { self.calm_for.saturating_add(1) } else { 0 };
        if self.pending() {
            self.unsaved_for = self.unsaved_for.saturating_add(1);
        }
        let gap = self.since_save >= MIN_GAP_FRAMES;
        let due = self.now
            || (calm
                && (self.bag_closed
                    || (gap && self.changed)
                    || (gap && self.moved && self.calm_for >= STILL_FRAMES)
                    || (self.pending() && self.unsaved_for >= FALLBACK_FRAMES)));
        if due {
            self.saved();
        }
        due
    }

    /// The game was saved (by `tick`, or otherwise): nothing waits.
    pub fn saved(&mut self) {
        let slept = self.slept;
        *self = Self { slept, ..Self::default() };
    }

    /// Nothing is to be saved any more (the castaway has collapsed).
    pub fn forget(&mut self) {
        *self = Self::new();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Frames until `tick` says to save, the castaway calm or not
    /// throughout (none within `limit`).
    fn frames_to_save(auto: &mut AutoSave, calm: bool, limit: u32) -> Option<u32> {
        (1..=limit).find(|_| auto.tick(calm))
    }

    #[test]
    fn nothing_changed_is_not_saved() {
        let mut auto = AutoSave::new();
        assert_eq!(frames_to_save(&mut auto, true, 2 * FALLBACK_FRAMES), None);
    }

    #[test]
    fn an_item_got_is_saved_at_the_next_calm_moment() {
        let mut auto = AutoSave::new();
        auto.note(Cue::Changed);
        assert_eq!(frames_to_save(&mut auto, true, 10), Some(1));
    }

    #[test]
    fn nothing_is_saved_in_the_middle_of_doing_something() {
        let mut auto = AutoSave::new();
        auto.note(Cue::Changed);
        assert_eq!(frames_to_save(&mut auto, false, 100), None);
        assert!(auto.tick(true), "saved once it's over");
    }

    #[test]
    fn walking_is_not_saved_while_walking() {
        let mut auto = AutoSave::new();
        auto.note(Cue::Moved);
        assert_eq!(frames_to_save(&mut auto, false, 600), None);
    }

    #[test]
    fn walking_is_saved_after_standing_still_a_while() {
        let mut auto = AutoSave::new();
        auto.note(Cue::Moved);
        let frames = frames_to_save(&mut auto, true, 600).expect("saved");
        assert!(frames > 1, "not the moment they stop");
    }

    #[test]
    fn standing_still_for_a_moment_between_steps_is_not_saved() {
        let mut auto = AutoSave::new();
        auto.note(Cue::Moved);
        for _ in 0..20 {
            assert!(!auto.tick(true));
            auto.note(Cue::Moved);
            assert!(!auto.tick(false));
        }
    }

    #[test]
    fn things_happening_close_together_are_saved_once() {
        let mut auto = AutoSave::new();
        auto.note(Cue::Changed);
        assert!(auto.tick(true));
        auto.note(Cue::Changed);
        let next = frames_to_save(&mut auto, true, 600).expect("saved in the end");
        assert!(next > 1, "not straight after the last save");
    }

    #[test]
    fn something_left_unsaved_long_is_saved_at_the_next_calm_moment() {
        let mut auto = AutoSave::new();
        auto.note(Cue::Moved);
        // Calm only for moments, never long enough for a walk to be saved.
        let mut saved = false;
        for i in 0..2 * FALLBACK_FRAMES {
            if auto.tick(i % 10 == 0) {
                saved = true;
                break;
            }
        }
        assert!(saved);
    }

    #[test]
    fn closing_the_bag_is_saved_without_waiting() {
        let mut auto = AutoSave::new();
        auto.note(Cue::Changed);
        assert!(auto.tick(true));
        auto.note(Cue::BagClosed);
        assert_eq!(frames_to_save(&mut auto, true, 10), Some(1));
    }

    #[test]
    fn closing_the_bag_waits_for_what_was_taken_out_to_be_used() {
        let mut auto = AutoSave::new();
        auto.note(Cue::BagClosed);
        assert_eq!(frames_to_save(&mut auto, false, 100), None);
        assert!(auto.tick(true));
    }

    #[test]
    fn waking_is_saved_straight_away() {
        let mut auto = AutoSave::new();
        auto.note(Cue::Changed);
        assert!(auto.tick(true));
        auto.woke();
        assert!(auto.tick(false));
    }

    #[test]
    fn sleep_is_saved_every_so_often() {
        let mut auto = AutoSave::new();
        let mut saves = 0;
        for _ in 0..4 * SLEEP_SAVE_MINUTES / 2 {
            auto.slept(2);
            if auto.tick(false) {
                saves += 1;
            }
        }
        assert_eq!(saves, 4);
    }

    #[test]
    fn a_short_nap_is_saved_only_on_waking() {
        let mut auto = AutoSave::new();
        auto.slept(SLEEP_SAVE_MINUTES / 2);
        assert!(!auto.tick(false));
        auto.woke();
        assert!(auto.tick(false));
        auto.slept(SLEEP_SAVE_MINUTES / 2);
        assert!(!auto.tick(false), "the count of sleep starts again on waking");
    }

    #[test]
    fn nothing_is_saved_once_forgotten() {
        let mut auto = AutoSave::new();
        auto.note(Cue::Changed);
        auto.note(Cue::Now);
        auto.forget();
        assert_eq!(frames_to_save(&mut auto, true, 2 * FALLBACK_FRAMES), None);
    }

    #[test]
    fn passing_deeds_are_not_saved() {
        assert_eq!(cue_of(&Deed::KickDust((0, 0))), None);
        assert_eq!(cue_of(&Deed::PickUp), Some(Cue::Changed));
    }
}

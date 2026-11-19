//! How the rescue's ending runs, frame by frame: the world fades out,
//! three cuts follow one after another (each fading in from black, held,
//! and fading out again), then the records.

/// Frames the world takes to fade out once the castaway is rescued.
pub const FADE_OUT: u32 = 120;
/// Frames each cut takes to fade in (and out again), and is held between.
pub const CUT_FADE: u32 = 30;
pub const CUT_HOLD: u32 = 240;
pub const CUT_FRAMES: u32 = CUT_FADE + CUT_HOLD + CUT_FADE;
/// The cuts: the island from afar with its smoke; the view out of a
/// passing plane's cockpit, the island far off; the plane turning toward
/// the island.
pub const CUTS: u32 = 3;
/// When the records come up.
pub const RECORDS_AT: u32 = FADE_OUT + CUTS * CUT_FRAMES;
/// Frames the records show before START begins again.
pub const RECORDS_HOLD: u32 = 60;

/// What the rescue's ending shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EndingView {
    /// The world fading out.
    FadingOut,
    /// One of the cuts (from 0).
    Cut(usize),
    /// The records.
    Records,
}

/// What shows `t` frames after the rescue.
pub fn view_at(t: u32) -> EndingView {
    if t < FADE_OUT {
        EndingView::FadingOut
    } else if t < RECORDS_AT {
        EndingView::Cut(((t - FADE_OUT) / CUT_FRAMES) as usize)
    } else {
        EndingView::Records
    }
}

/// How far into its cut `t` is (frames), during a cut.
pub fn cut_frame(t: u32) -> Option<u32> {
    matches!(view_at(t), EndingView::Cut(_)).then(|| (t - FADE_OUT) % CUT_FRAMES)
}

/// The cut's brightness (0..=15) `f` frames into it: fading in from
/// black, held, fading out again.
pub fn cut_brightness(f: u32) -> u8 {
    let up = f.min(CUT_FADE);
    let down = CUT_FRAMES.saturating_sub(f).min(CUT_FADE);
    (15 * up.min(down) / CUT_FADE) as u8
}

/// The frame START skips to from the cuts: the records.
pub fn skip(t: u32) -> u32 {
    t.max(RECORDS_AT)
}

/// Whether START now begins again (the records have shown long enough).
pub fn can_begin_again(t: u32) -> bool {
    t >= RECORDS_AT + RECORDS_HOLD
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_world_fades_then_each_cut_in_turn_then_the_records() {
        assert_eq!(view_at(0), EndingView::FadingOut);
        assert_eq!(view_at(FADE_OUT), EndingView::Cut(0));
        assert_eq!(view_at(FADE_OUT + CUT_FRAMES), EndingView::Cut(1));
        assert_eq!(view_at(FADE_OUT + 3 * CUT_FRAMES - 1), EndingView::Cut(2));
        assert_eq!(view_at(RECORDS_AT), EndingView::Records);
    }

    #[test]
    fn each_cut_fades_in_from_black_and_out_again() {
        assert_eq!(cut_brightness(0), 0);
        assert_eq!(cut_brightness(CUT_FADE), 15);
        assert_eq!(cut_brightness(CUT_FADE + CUT_HOLD), 15);
        assert!(cut_brightness(CUT_FRAMES - 1) < 15);
        assert_eq!(cut_brightness(CUT_FRAMES), 0);
    }

    #[test]
    fn skipping_goes_to_the_records_and_not_back() {
        assert_eq!(view_at(skip(FADE_OUT + 10)), EndingView::Records);
        assert_eq!(skip(RECORDS_AT + 5), RECORDS_AT + 5);
        assert!(!can_begin_again(skip(0)));
    }
}

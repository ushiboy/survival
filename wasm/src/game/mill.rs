//! Building the water mill: the stages it goes up in, and how far it has
//! got. Each stage takes materials, left at the site a few at a time (from
//! the bag, or the cart), and tools carried; once they're all in, the
//! castaway builds it, which takes hours. The ground has to be marked out
//! first (`WorldMap::is_staked`).

use crate::items::ItemKind;

/// A stage the mill goes up in.
#[derive(Debug)]
pub struct MillStage {
    /// As the BUILD page heads it.
    pub name: &'static str,
    /// Materials used up, left at the site beforehand.
    pub needs: &'static [(ItemKind, u32)],
    /// Tools carried to build it (not used up).
    pub tools: &'static [ItemKind],
    /// How long building it takes (game hours).
    pub hours: u32,
}

/// The stages, in the order they go up.
pub const MILL_STAGES: &[MillStage] = &[
    // Holes dug with the stick where the posts will stand, the earth
    // tamped down, flat stones bedded in it for the posts to stand on.
    MillStage { name: "FOUNDATION", needs: &[(ItemKind::Stone, 10), (ItemKind::Soil, 4)], tools: &[ItemKind::DiggingStick], hours: 3 },
];

/// How far the mill has got: the stages built, and what has been left at
/// the site for the next (by its line in `needs`).
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MillWorks {
    pub built: usize,
    pub delivered: Vec<u32>,
}

impl MillWorks {
    /// The stage to build next, if any is left.
    pub fn next(&self) -> Option<&'static MillStage> {
        MILL_STAGES.get(self.built)
    }

    /// How many of `kind` have been left at the site for the next stage.
    pub fn delivered(&self, kind: ItemKind) -> u32 {
        let Some(stage) = self.next() else { return 0 };
        stage.needs.iter().position(|&(k, _)| k == kind).and_then(|i| self.delivered.get(i).copied()).unwrap_or(0)
    }

    /// How many more of `kind` the next stage takes.
    pub fn still_needs(&self, kind: ItemKind) -> u32 {
        let Some(stage) = self.next() else { return 0 };
        stage.needs.iter().find(|&&(k, _)| k == kind).map_or(0, |&(_, n)| n.saturating_sub(self.delivered(kind)))
    }

    /// Leaves `n` of `kind` at the site (no more than the next stage
    /// takes); returns how many were taken.
    pub fn deliver(&mut self, kind: ItemKind, n: u32) -> u32 {
        let Some(stage) = self.next() else { return 0 };
        let Some(i) = stage.needs.iter().position(|&(k, _)| k == kind) else { return 0 };
        let n = n.min(self.still_needs(kind));
        if self.delivered.len() < stage.needs.len() {
            self.delivered.resize(stage.needs.len(), 0);
        }
        self.delivered[i] += n;
        n
    }

    /// Whether everything the next stage takes has been left at the site.
    pub fn all_in(&self) -> bool {
        self.next().is_some_and(|s| s.needs.iter().all(|&(k, _)| self.still_needs(k) == 0))
    }

    /// The next stage is built: on to the one after, nothing left for it
    /// yet.
    pub fn finish(&mut self) {
        self.built += 1;
        self.delivered.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn materials_left_add_up_to_what_the_stage_takes_and_no_more() {
        let mut w = MillWorks::default();
        assert_eq!(w.deliver(ItemKind::Stone, 4), 4);
        assert_eq!(w.deliver(ItemKind::Stone, 100), 6);
        assert_eq!(w.delivered(ItemKind::Stone), 10);
        assert_eq!(w.still_needs(ItemKind::Stone), 0);
    }

    #[test]
    fn what_the_stage_doesnt_take_isnt_left() {
        let mut w = MillWorks::default();
        assert_eq!(w.deliver(ItemKind::Coconut, 3), 0);
    }

    #[test]
    fn a_stage_is_ready_only_with_every_material_in() {
        let mut w = MillWorks::default();
        w.deliver(ItemKind::Stone, 10);
        assert!(!w.all_in());
        w.deliver(ItemKind::Soil, 4);
        assert!(w.all_in());
    }

    #[test]
    fn once_built_the_next_stage_starts_with_nothing_left_for_it() {
        let mut w = MillWorks::default();
        w.deliver(ItemKind::Stone, 10);
        w.finish();
        assert_eq!(w.built, 1);
        assert_eq!(w.delivered(ItemKind::Stone), 0);
    }

    #[test]
    fn every_stage_takes_something_and_some_time() {
        for s in MILL_STAGES {
            assert!(!s.needs.is_empty() && s.hours > 0, "{}", s.name);
        }
    }
}

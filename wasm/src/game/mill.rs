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
    // Posts of lauan (hard and strong) stood on the stones, and plates of
    // it along the north and south walls on their heads; beams of pine
    // (long and straight) across between the plates. Shaped with the axe,
    // the joints cut with the knife and lashed with rope.
    MillStage {
        name: "FRAME",
        needs: &[(ItemKind::LauanTimber, 8), (ItemKind::PineTimber, 3), (ItemKind::Rope, 4)],
        tools: &[ItemKind::StoneAxe, ItemKind::StoneKnife],
        hours: 6,
    },
    // Bamboo split with the knife into lath, woven between the posts and
    // tied with vine; clay and soil worked together and plastered on with
    // the spade. All round but a doorway in the south wall.
    MillStage {
        name: "WALLS",
        needs: &[(ItemKind::Bamboo, 8), (ItemKind::Vine, 4), (ItemKind::Clay, 6), (ItemKind::Soil, 6)],
        tools: &[ItemKind::StoneKnife, ItemKind::BambooSpade],
        hours: 8,
    },
    // Rafters of bamboo (and a scaffold of it to work from) lashed to the
    // frame with rope; thatch laid over them in courses from the eaves up
    // and tied down, the ridge capped; a sliding door of lauan planks hung
    // in the doorway, running on a lauan sill. The last stage: the hut is
    // done.
    MillStage {
        name: "ROOF",
        needs: &[(ItemKind::Thatch, 6), (ItemKind::Bamboo, 8), (ItemKind::Rope, 4), (ItemKind::LauanTimber, 2)],
        tools: &[ItemKind::StoneKnife],
        hours: 10,
    },
];

/// How far a post of the mill's frame rises above the middle of its
/// stone's cell (px), and how far into the cell it stands on (on the
/// stone). The front (south) posts are taller, so their plate clears the
/// head of someone standing by the south wall; the back (north) ones
/// shorter, so their plate stays clear of the river's water drawn on BG2
/// north-east of the ground. The beams across between the plates come out
/// shorter.
pub const POST_TOP: i32 = 20;
pub const POST_TOP_FRONT: i32 = 32;
pub const POST_FOOT: i32 = 2;

/// The height (world px) of the heads of the posts on the row `cy` of the
/// mill's ground, whose south edge is the row `y1`.
pub fn post_head(cy: i32, y1: i32) -> i32 {
    cy * 16 + 8 - post_rise(cy, y1)
}

/// How far the posts on the row `cy` rise (`POST_TOP` or, along the south
/// edge `y1`, `POST_TOP_FRONT`).
pub fn post_rise(cy: i32, y1: i32) -> i32 {
    if cy == y1 { POST_TOP_FRONT } else { POST_TOP }
}

/// How long the beams across the mill's frame are on screen (px): from the
/// top of the north plate down to the bottom of the south one.
pub fn beam_len() -> i32 {
    let (_, y0, _, y1) = crate::world::map::mill_plot((0, 0));
    post_head(y1, y1) + 4 - post_head(y0, y1)
}

/// Where a post stands on its stone in the cell `(cx, cy)`: its foot (the
/// middle of its bottom, world px), what it's sorted among sprites by.
pub fn post_foot((cx, cy): (i32, i32)) -> (i32, i32) {
    (cx * 16 + 8, cy * 16 + 8 + POST_FOOT)
}

/// Something made at the workbench in the mill once it's built, to fit
/// it out: made as the castaway likes, in any order, one of each. It
/// stands in its own place in the room.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Furnishing {
    /// Things are left on it, as many as like.
    Shelf,
    /// Slept in, health mends faster, and ailments pass sooner.
    Bed,
}

impl Furnishing {
    /// In the order listed at the workbench.
    pub const ALL: [Furnishing; 2] = [Furnishing::Bed, Furnishing::Shelf];

    /// As the workbench lists it.
    pub fn name(self) -> &'static str {
        match self {
            Furnishing::Shelf => "SHELF",
            Furnishing::Bed => "BED",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Furnishing::Shelf => "A SHELF BY THE WALL. LEAVE ON IT AS MUCH AS YOU LIKE.",
            Furnishing::Bed => "A BED UP OFF THE FLOOR. SLEEP IN IT TO HEAL FASTER AND GET WELL SOONER.",
        }
    }

    /// Materials used up.
    pub fn needs(self) -> &'static [(ItemKind, u32)] {
        match self {
            // Posts and feet of lauan (hard and strong), boards of pine
            // (long and straight), lashed with rope.
            Furnishing::Shelf => &[(ItemKind::LauanTimber, 2), (ItemKind::PineTimber, 2), (ItemKind::Rope, 2)],
            // A frame and legs of lauan, bamboo split into slats across
            // it, lashed with rope; leaves stuffed in for a mattress, and
            // palm fronds woven into a mat over it.
            Furnishing::Bed => &[(ItemKind::LauanTimber, 2), (ItemKind::Bamboo, 4), (ItemKind::Rope, 2), (ItemKind::Leaf, 10), (ItemKind::PalmFrond, 4)],
        }
    }

    /// Tools carried to make it (not used up).
    pub fn tools(self) -> &'static [ItemKind] {
        &[ItemKind::StoneAxe, ItemKind::StoneKnife]
    }

    /// How long making it takes (game hours).
    pub fn hours(self) -> u32 {
        match self {
            Furnishing::Shelf => 4,
            Furnishing::Bed => 6,
        }
    }

    /// Whether it comes to mind: every material it takes has been had
    /// (as with recipes; the tools don't count).
    pub fn thought_of(self, had: impl Fn(ItemKind) -> bool) -> bool {
        self.needs().iter().all(|&(k, _)| had(k))
    }
}

/// How far the mill has got: the stages built, and what has been left at
/// the site for the next (by its line in `needs`); then, once it's built,
/// what it has been fitted out with.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MillWorks {
    pub built: usize,
    pub delivered: Vec<u32>,
    pub furnished: Vec<Furnishing>,
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

    /// Whether the mill has been fitted out with `f`.
    pub fn has(&self, f: Furnishing) -> bool {
        self.furnished.contains(&f)
    }

    /// The mill is fitted out with `f` (once only).
    pub fn furnish(&mut self, f: Furnishing) {
        if !self.has(f) {
            self.furnished.push(f);
        }
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
    fn every_furnishing_takes_something_and_some_time() {
        for f in Furnishing::ALL {
            assert!(!f.needs().is_empty() && f.hours() > 0, "{}", f.name());
        }
    }

    #[test]
    fn the_mill_is_fitted_out_with_each_furnishing_once() {
        let mut w = MillWorks::default();
        w.furnish(Furnishing::Shelf);
        w.furnish(Furnishing::Shelf);
        assert_eq!(w.furnished, vec![Furnishing::Shelf]);
        assert!(w.has(Furnishing::Shelf) && !w.has(Furnishing::Bed));
    }

    #[test]
    fn a_furnishing_comes_to_mind_once_all_its_materials_have_been_had() {
        let needs = Furnishing::Shelf.needs();
        let (last, rest) = needs.split_last().unwrap();
        assert!(!Furnishing::Shelf.thought_of(|k| rest.iter().any(|&(r, _)| r == k)));
        assert!(Furnishing::Shelf.thought_of(|k| k == last.0 || rest.iter().any(|&(r, _)| r == k)));
    }

    #[test]
    fn every_stage_takes_something_and_some_time() {
        for s in MILL_STAGES {
            assert!(!s.needs.is_empty() && s.hours > 0, "{}", s.name);
        }
    }
}

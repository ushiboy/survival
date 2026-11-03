//! Kinds of things the castaway can carry, and how much room each takes in
//! the bag (in grid cells).

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ItemKind {
    Coconut,
    /// A squared length of palm wood.
    Timber,
    /// River water, not yet boiled. Carried in cupped leaves; it can't be
    /// put down, only poured away.
    RawWater,
    Axe,
}

/// What using an item does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Use {
    Eat,
    Drink,
}

impl ItemKind {
    /// Footprint in bag cells (width, height), unturned.
    pub fn size(self) -> (i32, i32) {
        match self {
            ItemKind::Coconut | ItemKind::RawWater => (1, 1),
            ItemKind::Timber => (2, 2),
            ItemKind::Axe => (1, 3),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            ItemKind::Coconut => "COCONUT",
            ItemKind::Timber => "TIMBER",
            ItemKind::RawWater => "RAW WATER",
            ItemKind::Axe => "AXE",
        }
    }

    pub fn usage(self) -> Option<Use> {
        match self {
            ItemKind::Coconut => Some(Use::Eat),
            ItemKind::RawWater => Some(Use::Drink),
            ItemKind::Timber | ItemKind::Axe => None,
        }
    }

    /// Whether it can lie on the ground (water can only be poured away).
    pub fn can_put_down(self) -> bool {
        self != ItemKind::RawWater
    }
}

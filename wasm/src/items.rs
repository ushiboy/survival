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
    /// A sharp stone lashed to a branch with vine.
    StoneAxe,
    /// A dead branch picked up off the ground.
    Branch,
    Stone,
    /// A length of vine pulled from a climbing plant, coiled up.
    Vine,
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
            ItemKind::Coconut | ItemKind::RawWater | ItemKind::Stone | ItemKind::Vine => (1, 1),
            ItemKind::Branch | ItemKind::StoneAxe => (1, 2),
            ItemKind::Timber => (2, 2),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            ItemKind::Coconut => "COCONUT",
            ItemKind::Timber => "TIMBER",
            ItemKind::RawWater => "RAW WATER",
            ItemKind::StoneAxe => "STONE AXE",
            ItemKind::Branch => "BRANCH",
            ItemKind::Stone => "STONE",
            ItemKind::Vine => "VINE",
        }
    }

    pub fn usage(self) -> Option<Use> {
        match self {
            ItemKind::Coconut => Some(Use::Eat),
            ItemKind::RawWater => Some(Use::Drink),
            _ => None,
        }
    }

    /// Whether it can lie on the ground (water can only be poured away).
    pub fn can_put_down(self) -> bool {
        self != ItemKind::RawWater
    }
}

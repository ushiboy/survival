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
    /// A bow strung with vine and a spindle for drilling a fire.
    FireDrill,
    /// A dead branch picked up off the ground.
    Branch,
    Stone,
    /// A length of vine pulled from a climbing plant, coiled up.
    Vine,
    /// A branch whittled to a point against a stone.
    WoodSpear,
    /// A crab caught with the spear.
    Crab,
    /// A broad leaf picked from a bush; folded into a cup it holds water.
    Leaf,
}

/// What using an item does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Use {
    Eat,
    Drink,
    /// Drill a fire into being (the drill is kept; a branch is burnt).
    Light,
}

impl Use {
    /// Whether using it uses it up.
    pub fn uses_up(self) -> bool {
        self != Use::Light
    }
}

impl ItemKind {
    /// Footprint in bag cells (width, height), unturned.
    pub fn size(self) -> (i32, i32) {
        match self {
            ItemKind::Coconut | ItemKind::RawWater | ItemKind::Stone | ItemKind::Vine | ItemKind::FireDrill | ItemKind::Crab | ItemKind::Leaf => (1, 1),
            ItemKind::Branch | ItemKind::StoneAxe => (1, 2),
            ItemKind::WoodSpear => (1, 3),
            ItemKind::Timber => (2, 2),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            ItemKind::Coconut => "COCONUT",
            ItemKind::Timber => "TIMBER",
            ItemKind::RawWater => "RAW WATER",
            ItemKind::StoneAxe => "STONE AXE",
            ItemKind::FireDrill => "FIRE DRILL",
            ItemKind::Branch => "BRANCH",
            ItemKind::Stone => "STONE",
            ItemKind::Vine => "VINE",
            ItemKind::WoodSpear => "WOOD SPEAR",
            ItemKind::Crab => "CRAB",
            ItemKind::Leaf => "LEAF",
        }
    }

    pub fn usage(self) -> Option<Use> {
        match self {
            ItemKind::Coconut | ItemKind::Crab => Some(Use::Eat),
            ItemKind::RawWater => Some(Use::Drink),
            ItemKind::FireDrill => Some(Use::Light),
            _ => None,
        }
    }

    /// How much eating it fills the food and water gauges: a coconut's
    /// flesh about a third and its milk a little thirst; a crab more food
    /// but hardly any water.
    pub fn nourishment(self) -> (u32, u32) {
        match self {
            ItemKind::Coconut => (22, 6),
            ItemKind::Crab => (30, 2),
            _ => (0, 0),
        }
    }

    /// How many fit in one stack in the bag (1 for things that don't stack).
    pub fn max_stack(self) -> u32 {
        match self {
            ItemKind::Leaf => 20,
            _ => 1,
        }
    }

    /// Tools are held in hand and used with B.
    pub fn is_tool(self) -> bool {
        matches!(self, ItemKind::StoneAxe | ItemKind::WoodSpear)
    }

    /// Whether it can lie on the ground (water can only be poured away).
    pub fn can_put_down(self) -> bool {
        self != ItemKind::RawWater
    }
}

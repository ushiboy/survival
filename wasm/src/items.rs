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
    /// A sharp stone lashed to a branch with rope.
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
    /// A crab cooked over a campfire.
    GrilledCrab,
    /// A strip of bark peeled off a broadleaf tree.
    Bark,
    /// A squared length of lauan, the reddish hardwood of broadleaf trees.
    LauanTimber,
    /// A pot of bark stitched together with vine.
    BarkContainer,
    /// River water boiled over a fire in a bark pot: safe to drink.
    BoiledWater,
    /// A carrying frame of branches lashed with rope, with a bark back
    /// board: worn on the back, it makes room for more in the bag.
    PackFrame,
    /// A broad leaf picked from a bush; folded into a cup it holds water.
    Leaf,
    /// The fibrous husk left over once a coconut's been eaten.
    Husk,
    /// Cord twisted from coconut husk fibre (coir): stronger than vine.
    Rope,
    /// A long palm leaf, shaken loose or brought down with a felled palm.
    PalmFrond,
    /// A panel of palm fronds woven together with vine: a piece of roof.
    Thatch,
    /// A lean-to of branch poles and thatch, rolled up to carry; pitched
    /// over the castaway when they lie down to sleep.
    Shelter,
}

/// Where a piece of gear goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Slot {
    /// Held in hand and used with B (a tool).
    Hand,
    /// Worn on the back (a pack frame).
    Back,
}

impl Slot {
    pub const ALL: [Slot; 2] = [Slot::Hand, Slot::Back];

    pub fn label(self) -> &'static str {
        match self {
            Slot::Hand => "HAND",
            Slot::Back => "BACK",
        }
    }
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
            ItemKind::Coconut | ItemKind::RawWater | ItemKind::Stone | ItemKind::Vine | ItemKind::FireDrill | ItemKind::Crab | ItemKind::GrilledCrab | ItemKind::Leaf | ItemKind::Bark | ItemKind::BarkContainer | ItemKind::BoiledWater | ItemKind::Husk | ItemKind::Rope => (1, 1),
            ItemKind::Branch | ItemKind::StoneAxe | ItemKind::PackFrame | ItemKind::PalmFrond => (1, 2),
            ItemKind::Thatch => (2, 1),
            ItemKind::WoodSpear => (1, 3),
            ItemKind::Timber | ItemKind::LauanTimber | ItemKind::Shelter => (2, 2),
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
            ItemKind::GrilledCrab => "GRILLED CRAB",
            ItemKind::Leaf => "LEAF",
            ItemKind::Bark => "BARK",
            ItemKind::LauanTimber => "LAUAN TIMBER",
            ItemKind::BarkContainer => "BARK POT",
            ItemKind::BoiledWater => "BOILED WATER",
            ItemKind::PackFrame => "PACK FRAME",
            ItemKind::Husk => "HUSK",
            ItemKind::Rope => "ROPE",
            ItemKind::PalmFrond => "PALM FROND",
            ItemKind::Thatch => "THATCH",
            ItemKind::Shelter => "SHELTER",
        }
    }

    /// A line or two about what it is and what it's good for, for the
    /// crafting screen.
    pub fn description(self) -> &'static str {
        match self {
            ItemKind::Coconut => "FLESH TO EAT AND MILK TO DRINK.",
            ItemKind::Timber => "A SQUARED LENGTH OF PALM WOOD.",
            ItemKind::RawWater => "RIVER WATER IN A LEAF. EASES THIRST.",
            ItemKind::StoneAxe => "A SHARP STONE LASHED TO A BRANCH. FELLS TREES.",
            ItemKind::FireDrill => "A BOW AND SPINDLE. BURNS A BRANCH TO LIGHT A FIRE.",
            ItemKind::Branch => "A DEAD BRANCH, GOOD FOR TOOLS AND FIRES.",
            ItemKind::Stone => "A STONE WITH A SHARP EDGE.",
            ItemKind::Vine => "A LENGTH OF VINE FOR LIGHT LASHING.",
            ItemKind::WoodSpear => "A SHARPENED BRANCH. THRUST IT TO CATCH CRABS.",
            ItemKind::Crab => "A RAW CRAB. FILLING, BUT BETTER COOKED.",
            ItemKind::GrilledCrab => "COOKED OVER A FIRE. VERY FILLING, AND HEALS.",
            ItemKind::Leaf => "A BROAD LEAF. FOLDED, IT HOLDS WATER.",
            ItemKind::Bark => "A STRIP OF BARK FROM A BROADLEAF TREE.",
            ItemKind::LauanTimber => "A SQUARED LENGTH OF LAUAN, A STRONG HARDWOOD.",
            ItemKind::BarkContainer => "A POT OF BARK. WATER CAN BE BOILED IN IT.",
            ItemKind::BoiledWater => "SAFE TO DRINK. EASES THIRST, AND HEALS.",
            ItemKind::PackFrame => "WORN ON THE BACK, IT GIVES THE BAG 12 MORE ROOM.",
            ItemKind::Husk => "LEFT FROM A COCONUT. ITS FIBRE MAKES ROPE.",
            ItemKind::Rope => "TWISTED COCONUT FIBRE. STRONGER THAN VINE.",
            ItemKind::PalmFrond => "A LONG PALM LEAF. WOVEN TOGETHER, FRONDS KEEP OFF RAIN.",
            ItemKind::Thatch => "PALM FRONDS WOVEN WITH VINE INTO A PIECE OF ROOF.",
            ItemKind::Shelter => "A LEAN-TO, ROLLED UP. SLEEP UNDER IT TO HEAL FASTER AND TIRE LESS.",
        }
    }

    pub fn usage(self) -> Option<Use> {
        match self {
            ItemKind::Coconut | ItemKind::Crab | ItemKind::GrilledCrab => Some(Use::Eat),
            ItemKind::RawWater | ItemKind::BoiledWater => Some(Use::Drink),
            ItemKind::FireDrill => Some(Use::Light),
            _ => None,
        }
    }

    /// How much eating or drinking it fills the food and water gauges: a
    /// coconut's flesh about a third and its milk a little thirst; a crab
    /// more food but hardly any water, and grilled it does much more; a
    /// draw of water a quarter of thirst, and a pot of boiled water more.
    pub fn nourishment(self) -> (u32, u32) {
        match self {
            ItemKind::RawWater => (0, 16),
            ItemKind::BoiledWater => (0, 24),
            ItemKind::Coconut => (22, 6),
            ItemKind::Crab => (30, 2),
            ItemKind::GrilledCrab => (48, 4),
            _ => (0, 0),
        }
    }

    /// How many fit in one stack in the bag (1 for things that don't stack).
    pub fn max_stack(self) -> u32 {
        match self {
            ItemKind::Leaf => 20,
            ItemKind::Husk | ItemKind::Rope | ItemKind::PalmFrond => 5,
            _ => 1,
        }
    }

    /// Hearts of health eating or drinking it restores (cooked food and
    /// boiled water are what heal).
    pub fn heals(self) -> u8 {
        match self {
            ItemKind::GrilledCrab | ItemKind::BoiledWater => 1,
            _ => 0,
        }
    }

    /// What's left in hand once it's been eaten or drunk (the pot boiled
    /// water was carried in, a coconut's husk).
    pub fn leaves_behind(self) -> Option<ItemKind> {
        match self {
            ItemKind::Coconut => Some(ItemKind::Husk),
            ItemKind::BoiledWater => Some(ItemKind::BarkContainer),
            _ => None,
        }
    }

    /// Where it's held or worn, if it's gear.
    pub fn slot(self) -> Option<Slot> {
        match self {
            ItemKind::StoneAxe | ItemKind::WoodSpear => Some(Slot::Hand),
            ItemKind::PackFrame => Some(Slot::Back),
            _ => None,
        }
    }

    /// Rows a pack worn on the back adds to the bag.
    pub fn extra_rows(self) -> i32 {
        match self {
            ItemKind::PackFrame => 2,
            _ => 0,
        }
    }

    /// Whether it can lie on the ground (water can only be poured away).
    pub fn can_put_down(self) -> bool {
        self != ItemKind::RawWater
    }
}

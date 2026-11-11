//! Kinds of things the castaway can carry, and how much room each takes in
//! the bag (in grid cells).

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ItemKind {
    /// A whole coconut in its husk: it has to be cut open to eat.
    Coconut,
    /// A coconut cut open with a knife: flesh to eat and milk to drink.
    OpenedCoconut,
    /// A squared length of palm wood.
    Timber,
    /// River water, not yet boiled. Carried in cupped leaves; it can't be
    /// put down, only poured away.
    RawWater,
    /// A sharp stone lashed to a branch with rope.
    StoneAxe,
    /// A flake struck off one stone with another: an edge for cutting bark.
    /// Carrying it is enough to use it.
    StoneKnife,
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
    /// A squared length of lauan, the light reddish hardwood of broadleaf
    /// trees.
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
    /// A branch with husk fibre wound round its head, lit at a campfire.
    /// Unlike everything else it never goes in the bag: once lit it's
    /// carried (beside whatever tool is in hand), lighting the dark, until
    /// it burns out half a day later.
    Torch,
    /// A scallop shell picked up off the beach (more wash up every day).
    SeaShell,
    /// A fish hook ground out of shell with the knife.
    ShellHook,
    /// A branch rod with a rope line and a shell hook: cast into the water
    /// with B and struck with B when the float goes under.
    FishingRod,
    /// A fish caught on the line.
    Fish,
    /// A fish cleaned and cooked over a fire.
    GrilledFish,
    /// Two lauan poles with branch rungs lashed across them with rope:
    /// held in hand and stood against a cliff low enough for it to reach
    /// the top (B), where it stays.
    Ladder,
    /// A pine cone, fallen from a pine: resinous, so it burns well, and
    /// small, so it's handier fuel to carry than a branch.
    PineCone,
    /// A long, straight length of pine (the conifer of the high ground):
    /// light for its size.
    PineTimber,
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
    /// Drill a fire into being (the drill is kept; a pine cone or a branch
    /// is burnt).
    Light,
}

impl Use {
    /// Whether using it uses it up.
    pub fn uses_up(self) -> bool {
        self != Use::Light
    }
}

impl ItemKind {
    /// Every kind, in the order the debug tools list them (add new kinds
    /// here too).
    pub const ALL: [ItemKind; 33] = [
        ItemKind::Coconut,
        ItemKind::OpenedCoconut,
        ItemKind::Timber,
        ItemKind::RawWater,
        ItemKind::StoneAxe,
        ItemKind::StoneKnife,
        ItemKind::FireDrill,
        ItemKind::Branch,
        ItemKind::Stone,
        ItemKind::Vine,
        ItemKind::WoodSpear,
        ItemKind::Crab,
        ItemKind::GrilledCrab,
        ItemKind::Bark,
        ItemKind::LauanTimber,
        ItemKind::BarkContainer,
        ItemKind::BoiledWater,
        ItemKind::PackFrame,
        ItemKind::Leaf,
        ItemKind::Husk,
        ItemKind::Rope,
        ItemKind::PalmFrond,
        ItemKind::Thatch,
        ItemKind::Shelter,
        ItemKind::Torch,
        ItemKind::SeaShell,
        ItemKind::ShellHook,
        ItemKind::FishingRod,
        ItemKind::Fish,
        ItemKind::GrilledFish,
        ItemKind::Ladder,
        ItemKind::PineCone,
        ItemKind::PineTimber,
    ];

    /// Footprint in bag cells (width, height), unturned.
    pub fn size(self) -> (i32, i32) {
        match self {
            ItemKind::Coconut | ItemKind::OpenedCoconut | ItemKind::RawWater | ItemKind::Stone | ItemKind::StoneKnife | ItemKind::Vine | ItemKind::FireDrill | ItemKind::Crab | ItemKind::GrilledCrab | ItemKind::Leaf | ItemKind::Bark | ItemKind::BarkContainer | ItemKind::BoiledWater | ItemKind::Husk | ItemKind::Rope | ItemKind::SeaShell | ItemKind::ShellHook | ItemKind::Fish | ItemKind::GrilledFish | ItemKind::PineCone => (1, 1),
            ItemKind::Branch | ItemKind::StoneAxe | ItemKind::PackFrame | ItemKind::PalmFrond | ItemKind::Torch => (1, 2),
            ItemKind::Thatch => (2, 1),
            ItemKind::WoodSpear | ItemKind::FishingRod => (1, 3),
            ItemKind::Ladder => (1, 4),
            ItemKind::Timber | ItemKind::LauanTimber | ItemKind::PineTimber | ItemKind::Shelter => (2, 2),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            ItemKind::Coconut => "COCONUT",
            ItemKind::OpenedCoconut => "OPENED COCONUT",
            ItemKind::Timber => "TIMBER",
            ItemKind::RawWater => "RAW WATER",
            ItemKind::StoneAxe => "STONE AXE",
            ItemKind::StoneKnife => "STONE KNIFE",
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
            ItemKind::Torch => "TORCH",
            ItemKind::SeaShell => "SEA SHELL",
            ItemKind::ShellHook => "SHELL HOOK",
            ItemKind::FishingRod => "FISHING ROD",
            ItemKind::Fish => "FISH",
            ItemKind::GrilledFish => "GRILLED FISH",
            ItemKind::Ladder => "LADDER",
            ItemKind::PineCone => "PINE CONE",
            ItemKind::PineTimber => "PINE TIMBER",
        }
    }

    /// A line or two about what it is and what it's good for, for the
    /// crafting screen.
    pub fn description(self) -> &'static str {
        match self {
            ItemKind::Coconut => "A WHOLE COCONUT IN ITS HUSK. HARD TO GET INTO.",
            ItemKind::OpenedCoconut => "CUT OPEN. FLESH TO EAT AND MILK TO DRINK.",
            ItemKind::Timber => "A SQUARED LENGTH OF PALM WOOD.",
            ItemKind::RawWater => "RIVER WATER IN A LEAF. EASES THIRST.",
            ItemKind::StoneAxe => "A SHARP STONE LASHED TO A BRANCH. FELLS TREES.",
            ItemKind::StoneKnife => "A SHARP FLAKE OF STONE WITH A CUTTING EDGE.",
            ItemKind::FireDrill => "A BOW AND SPINDLE. BURNS FUEL TO LIGHT A FIRE.",
            ItemKind::Branch => "A DEAD BRANCH, GOOD FOR TOOLS AND FIRES.",
            ItemKind::Stone => "A STONE WITH A SHARP EDGE.",
            ItemKind::Vine => "A LENGTH OF VINE FOR LIGHT LASHING.",
            ItemKind::WoodSpear => "A SHARPENED BRANCH. THRUST IT TO CATCH CRABS.",
            ItemKind::Crab => "A RAW CRAB. FILLING, BUT BETTER COOKED.",
            ItemKind::GrilledCrab => "CLEANED AND COOKED OVER A FIRE. VERY FILLING, AND HEALS.",
            ItemKind::Leaf => "A BROAD LEAF. FOLDED, IT HOLDS WATER.",
            ItemKind::Bark => "A STRIP OF BARK FROM A BROADLEAF TREE.",
            ItemKind::LauanTimber => "A SQUARED LENGTH OF LAUAN, A LIGHT TROPICAL HARDWOOD.",
            ItemKind::BarkContainer => "A POT OF BARK. WATER CAN BE BOILED IN IT.",
            ItemKind::BoiledWater => "SAFE TO DRINK. EASES THIRST, AND HEALS.",
            ItemKind::PackFrame => "WORN ON THE BACK, IT GIVES THE BAG 12 MORE ROOM.",
            ItemKind::Husk => "LEFT FROM A COCONUT. ITS FIBRE MAKES ROPE.",
            ItemKind::Rope => "TWISTED COCONUT FIBRE. STRONGER THAN VINE.",
            ItemKind::PalmFrond => "A LONG PALM LEAF. WOVEN TOGETHER, FRONDS KEEP OFF RAIN.",
            ItemKind::Thatch => "PALM FRONDS WOVEN WITH VINE INTO A PIECE OF ROOF.",
            ItemKind::Shelter => "A LEAN-TO, ROLLED UP. SLEEP UNDER IT TO HEAL FASTER AND TIRE LESS.",
            ItemKind::Torch => "HUSK FIBRE ON A BRANCH, LIT AT A FIRE. CARRIED, IT LIGHTS THE DARK UNTIL IT BURNS OUT.",
            ItemKind::SeaShell => "A SCALLOP SHELL WASHED UP ON THE BEACH. HARD AND SMOOTH.",
            ItemKind::ShellHook => "A HOOK GROUND OUT OF SHELL. SMALL BUT SHARP.",
            ItemKind::FishingRod => "A ROD WITH A ROPE LINE AND A SHELL HOOK. CAST IT INTO WATER WHERE FISH SWIM.",
            ItemKind::Fish => "A FRESH FISH. FILLING, BUT BETTER COOKED.",
            ItemKind::GrilledFish => "CLEANED AND COOKED OVER A FIRE. FILLING, AND HEALS.",
            ItemKind::Ladder => "A LADDER OF LAUAN POLES AND BRANCHES, LASHED WITH ROPE.",
            ItemKind::PineCone => "A RESINOUS PINE CONE. IT BURNS WELL.",
            ItemKind::PineTimber => "A LONG, STRAIGHT LENGTH OF PINE. LIGHT FOR ITS SIZE.",
        }
    }

    pub fn usage(self) -> Option<Use> {
        match self {
            ItemKind::OpenedCoconut | ItemKind::Crab | ItemKind::GrilledCrab | ItemKind::Fish | ItemKind::GrilledFish => Some(Use::Eat),
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
            ItemKind::OpenedCoconut => (22, 6),
            ItemKind::Crab => (30, 2),
            ItemKind::GrilledCrab => (48, 4),
            ItemKind::Fish => (18, 4),
            ItemKind::GrilledFish => (36, 6),
            _ => (0, 0),
        }
    }

    /// How many fit in one stack in the bag (1 for things that don't stack).
    pub fn max_stack(self) -> u32 {
        match self {
            ItemKind::Leaf => 20,
            ItemKind::Husk | ItemKind::Rope | ItemKind::PalmFrond | ItemKind::SeaShell | ItemKind::PineCone => 5,
            _ => 1,
        }
    }

    /// Health eating or drinking it restores, of a 64-point gauge (cooked
    /// food and boiled water are what heal, about a fifth of it).
    pub fn heals(self) -> u32 {
        match self {
            ItemKind::GrilledCrab | ItemKind::GrilledFish | ItemKind::BoiledWater => 13,
            _ => 0,
        }
    }

    /// What's left in hand once it's been eaten or drunk (the pot boiled
    /// water was carried in, an opened coconut's husk).
    pub fn leaves_behind(self) -> Option<ItemKind> {
        match self {
            ItemKind::OpenedCoconut => Some(ItemKind::Husk),
            ItemKind::BoiledWater => Some(ItemKind::BarkContainer),
            _ => None,
        }
    }

    /// Where it's held or worn, if it's gear.
    pub fn slot(self) -> Option<Slot> {
        match self {
            ItemKind::StoneAxe | ItemKind::WoodSpear | ItemKind::FishingRod | ItemKind::Ladder => Some(Slot::Hand),
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

    /// Whether it goes in the bag once made or picked up (a lit torch is
    /// carried instead).
    pub fn goes_in_bag(self) -> bool {
        self != ItemKind::Torch
    }

    /// How getting rid of it from the bag reads: the command, and what's
    /// said once it's done.
    pub fn discard_words(self) -> (&'static str, &'static str) {
        match self {
            ItemKind::RawWater => ("POUR OUT", "POURED IT OUT"),
            _ => ("PUT DOWN", "PUT IT DOWN"),
        }
    }
}

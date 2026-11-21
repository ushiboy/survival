//! Kinds of things the castaway can carry, and how much room each takes in
//! the bag (in grid cells).

use crate::game::state::Ailment;

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
    /// Two pine logs lashed side by side with rope: held in hand and laid
    /// across the river where it runs narrow (B), where it stays and can be
    /// walked over.
    LogBridge,
    /// A pine cone, fallen from a pine: resinous, so it burns well, and
    /// small, so it's handier fuel to carry than a branch.
    PineCone,
    /// A long, straight length of pine (the conifer of the high ground):
    /// light for its size.
    PineTimber,
    /// A length of bamboo cut from a culm in the groves behind the west
    /// coast: long, hollow and light.
    Bamboo,
    /// A wheel of lauan (axle, arms and rim, strong enough to drive work)
    /// with paddles of split bamboo lashed on with rope, and two lauan posts
    /// with stone bearings to hold its axle: held in hand and set in the
    /// river at the mill site (B), where the current turns it.
    WaterWheel,
    /// A branch whittled to a point with the knife: held in hand and
    /// driven into the ground (B) to loosen earth and lift it out.
    DiggingStick,
    /// A length of bamboo split down the middle, its node left as the
    /// blade, bound with rope where the split stops: held in hand to lift
    /// soft, sticky clay (B). Hard ground would crack it.
    BambooSpade,
    /// A lump of wet clay dug from the lake's shore: it can be shaped, and
    /// fired.
    Clay,
    /// A handful of earth dug from bare ground.
    Soil,
    /// A belt of plaited bark strips with rope loops, cut to shape with the
    /// knife: worn at the waist, it holds two tools out of the bag.
    ToolBelt,
    /// A lauan bed with a headstock and a tailstock holding work between
    /// them on stone bearings, and a rope drive: held in hand and set on
    /// the bank beside the turning water wheel (B), which drives it to
    /// turn wood round.
    WoodLathe,
    /// A cartwheel of lauan (hub and rim) with spokes of branch, all
    /// turned round on the lathe.
    SpokedWheel,
    /// A cart of a lauan bed on two spoked wheels, pulled by two bamboo
    /// poles lashed on with rope. Like a torch it never goes in the bag:
    /// once made it follows the castaway, pulled behind, carrying a load of
    /// its own.
    Handcart,
    /// A hearth of stones bedded in earth with timber stacked on it: held
    /// in hand and set on open ground (B), where it stays. Lit with the
    /// fire drill, a branch and a palm frond burnt on it send up smoke for
    /// a day, for a passing plane to see.
    SignalFire,
    /// Stakes split from bamboo, and rope for a line between them: held in
    /// hand and driven in round the turning wood lathe (B), they mark out
    /// the ground the mill is to stand on.
    MarkingStakes,
}

/// Where a piece of gear goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Slot {
    /// Held in hand and used with B (a tool).
    Hand,
    /// Worn on the back (a pack frame).
    Back,
    /// Worn at the waist (a tool belt).
    Waist,
}

impl Slot {
    pub const ALL: [Slot; 3] = [Slot::Hand, Slot::Back, Slot::Waist];

    /// Whether `kind` can go here.
    pub fn holds(self, kind: ItemKind) -> bool {
        kind.slot() == Some(self)
    }

    /// Whether what's here is still in the bag (taking room there): a
    /// tool in hand is; anything worn is out of it.
    pub fn keeps_in_bag(self) -> bool {
        self == Slot::Hand
    }

    /// What's said once something is put here.
    pub fn done_words(self) -> &'static str {
        if self == Slot::Hand { "TOOK IT IN HAND" } else { "PUT IT ON" }
    }

    pub fn label(self) -> &'static str {
        match self {
            Slot::Hand => "HAND",
            Slot::Back => "BACK",
            Slot::Waist => "WAIST",
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
    /// Every kind, numbered as the debug tools give them (add new kinds
    /// here too; the page lists them by its own names for them).
    pub const ALL: [ItemKind; 46] = [
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
        ItemKind::LogBridge,
        ItemKind::PineCone,
        ItemKind::PineTimber,
        ItemKind::Bamboo,
        ItemKind::WaterWheel,
        ItemKind::DiggingStick,
        ItemKind::BambooSpade,
        ItemKind::Clay,
        ItemKind::Soil,
        ItemKind::ToolBelt,
        ItemKind::WoodLathe,
        ItemKind::SpokedWheel,
        ItemKind::Handcart,
        ItemKind::SignalFire,
        ItemKind::MarkingStakes,
    ];

    /// Footprint in bag cells (width, height), unturned.
    pub fn size(self) -> (i32, i32) {
        match self {
            ItemKind::Coconut
            | ItemKind::OpenedCoconut
            | ItemKind::RawWater
            | ItemKind::Stone
            | ItemKind::StoneKnife
            | ItemKind::Vine
            | ItemKind::FireDrill
            | ItemKind::Crab
            | ItemKind::GrilledCrab
            | ItemKind::Leaf
            | ItemKind::Bark
            | ItemKind::BarkContainer
            | ItemKind::BoiledWater
            | ItemKind::Husk
            | ItemKind::Rope
            | ItemKind::SeaShell
            | ItemKind::ShellHook
            | ItemKind::Fish
            | ItemKind::GrilledFish
            | ItemKind::PineCone
            | ItemKind::Clay
            | ItemKind::Soil => (1, 1),
            ItemKind::Branch
            | ItemKind::DiggingStick
            | ItemKind::StoneAxe
            | ItemKind::PackFrame
            | ItemKind::PalmFrond
            | ItemKind::Torch
            | ItemKind::ToolBelt => (1, 2),
            ItemKind::Thatch => (2, 1),
            ItemKind::WaterWheel => (3, 3),
            ItemKind::WoodSpear | ItemKind::FishingRod | ItemKind::Bamboo | ItemKind::BambooSpade | ItemKind::MarkingStakes => (1, 3),
            ItemKind::Ladder | ItemKind::LogBridge => (1, 4),
            ItemKind::Timber
            | ItemKind::LauanTimber
            | ItemKind::PineTimber
            | ItemKind::Shelter
            | ItemKind::SpokedWheel
            | ItemKind::Handcart
            | ItemKind::SignalFire => (2, 2),
            ItemKind::WoodLathe => (2, 3),
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
            ItemKind::LogBridge => "LOG BRIDGE",
            ItemKind::PineCone => "PINE CONE",
            ItemKind::PineTimber => "PINE TIMBER",
            ItemKind::Bamboo => "BAMBOO",
            ItemKind::WaterWheel => "WATER WHEEL",
            ItemKind::DiggingStick => "DIGGING STICK",
            ItemKind::BambooSpade => "BAMBOO SPADE",
            ItemKind::Clay => "CLAY",
            ItemKind::Soil => "SOIL",
            ItemKind::ToolBelt => "TOOL BELT",
            ItemKind::WoodLathe => "WOOD LATHE",
            ItemKind::SpokedWheel => "SPOKED WHEEL",
            ItemKind::Handcart => "HANDCART",
            ItemKind::SignalFire => "SIGNAL FIRE",
            ItemKind::MarkingStakes => "MARKING STAKES",
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
            ItemKind::Rope => "TWISTED PLANT FIBRE. STRONGER THAN VINE.",
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
            ItemKind::LogBridge => "TWO LONG PINE LOGS, LASHED SIDE BY SIDE WITH ROPE.",
            ItemKind::PineCone => "A RESINOUS PINE CONE. IT BURNS WELL.",
            ItemKind::PineTimber => "A LONG, STRAIGHT LENGTH OF PINE. LIGHT FOR ITS SIZE.",
            ItemKind::Bamboo => "A LENGTH OF BAMBOO. HOLLOW AND LIGHT.",
            ItemKind::WaterWheel => "A STOUT LAUAN WHEEL WITH PADDLES OF SPLIT BAMBOO. A RIVER COULD TURN IT.",
            ItemKind::DiggingStick => "A BRANCH WHITTLED TO A POINT. IT LOOSENS HARD EARTH.",
            ItemKind::BambooSpade => "SPLIT BAMBOO BOUND WITH ROPE. IT LIFTS SOFT, STICKY GROUND.",
            ItemKind::Clay => "WET, STICKY CLAY. IT CAN BE SHAPED, AND HARDENS IN A FIRE.",
            ItemKind::Soil => "A HANDFUL OF EARTH.",
            ItemKind::ToolBelt => "WORN AT THE WAIST, IT HOLDS TWO TOOLS OUT OF THE BAG.",
            ItemKind::WoodLathe => "A LAUAN BED AND SPINDLE WITH A ROPE DRIVE. SOMETHING TURNING COULD SPIN IT.",
            ItemKind::SpokedWheel => "A HUB, SPOKES AND RIM, ALL TURNED ROUND. IT ROLLS UNDER A LOAD.",
            ItemKind::Handcart => "A BED ON TWO WHEELS WITH BAMBOO POLES TO PULL IT BY. IT CARRIES FAR MORE THAN A BAG.",
            ItemKind::SignalFire => "STONES IN EARTH, WOOD STACKED ON. LIGHT A BRANCH AND A PALM FROND ON IT TO SEND UP SMOKE.",
            ItemKind::MarkingStakes => "STAKES AND A LINE TO MARK OUT GROUND FOR SOMETHING BIG.",
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

    /// What eating or drinking it raw may bring on, and the chance it does
    /// (the riskier the rawer: water from the river least, crab most).
    /// Cooked food, boiled water and coconut are safe.
    pub fn risk(self) -> Option<(Ailment, f32)> {
        match self {
            ItemKind::RawWater => Some((Ailment::UpsetStomach, 0.2)),
            ItemKind::Fish => Some((Ailment::UpsetStomach, 0.3)),
            ItemKind::Crab => Some((Ailment::UpsetStomach, 0.5)),
            _ => None,
        }
    }

    /// How many fit in one stack in the bag (1 for things that don't stack).
    pub fn max_stack(self) -> u32 {
        match self {
            ItemKind::Leaf => 20,
            ItemKind::Husk | ItemKind::Rope | ItemKind::PalmFrond | ItemKind::SeaShell | ItemKind::PineCone | ItemKind::Bamboo => 5,
            // Heavy for their size.
            ItemKind::Clay | ItemKind::Soil => 3,
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
            ItemKind::StoneAxe
            | ItemKind::WoodSpear
            | ItemKind::FishingRod
            | ItemKind::Ladder
            | ItemKind::LogBridge
            | ItemKind::WaterWheel
            | ItemKind::DiggingStick
            | ItemKind::BambooSpade
            | ItemKind::WoodLathe
            | ItemKind::SignalFire
            | ItemKind::MarkingStakes => Some(Slot::Hand),
            ItemKind::PackFrame => Some(Slot::Back),
            ItemKind::ToolBelt => Some(Slot::Waist),
            _ => None,
        }
    }

    /// Tools a belt worn at the waist holds.
    pub fn hooks(self) -> usize {
        match self {
            ItemKind::ToolBelt => 2,
            _ => 0,
        }
    }

    /// Whether it can hang on a tool belt: hand tools short and light
    /// enough to carry at the waist (not a ladder, a bridge or a wheel).
    pub fn hangs_on_belt(self) -> bool {
        matches!(self, ItemKind::StoneAxe | ItemKind::WoodSpear | ItemKind::FishingRod | ItemKind::DiggingStick | ItemKind::BambooSpade)
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
        !matches!(self, ItemKind::Torch | ItemKind::Handcart)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_log_bridge_is_carried_in_hand_like_the_ladder() {
        assert_eq!(ItemKind::LogBridge.slot(), ItemKind::Ladder.slot());
        assert_eq!(ItemKind::LogBridge.max_stack(), 1);
    }

    #[test]
    fn the_tool_belt_is_worn_at_the_waist() {
        assert_eq!(ItemKind::ToolBelt.slot(), Some(Slot::Waist));
        assert!(ItemKind::ToolBelt.hooks() > 0);
    }

    #[test]
    fn only_tools_held_in_hand_hang_on_the_belt() {
        for kind in ItemKind::ALL {
            if kind.hangs_on_belt() {
                assert_eq!(kind.slot(), Some(Slot::Hand), "{kind:?}");
            }
        }
    }

    #[test]
    fn digging_tools_are_held_in_hand() {
        assert_eq!(ItemKind::DiggingStick.slot(), Some(Slot::Hand));
        assert_eq!(ItemKind::BambooSpade.slot(), Some(Slot::Hand));
    }

    #[test]
    fn clay_and_soil_stack_but_fewer_than_bamboo_for_their_weight() {
        for heavy in [ItemKind::Clay, ItemKind::Soil] {
            assert!(heavy.max_stack() > 1);
            assert!(heavy.max_stack() < ItemKind::Bamboo.max_stack());
        }
    }

    #[test]
    fn bamboo_is_light_enough_to_stack_unlike_timber() {
        assert!(ItemKind::Bamboo.max_stack() > 1);
        for timber in [ItemKind::Timber, ItemKind::LauanTimber, ItemKind::PineTimber] {
            assert_eq!(timber.max_stack(), 1);
        }
    }
}

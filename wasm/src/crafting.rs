//! Recipes for making things out of gathered materials. Crafting happens in
//! the bag screen and uses up materials carried in the bag; some recipes
//! can only be made beside a campfire.

use crate::items::ItemKind;

/// Something that can be made, and what goes into it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Recipe {
    /// What a save calls it. The first way of making an item is named after
    /// the item (that was how recipes were saved before an item could have
    /// more than one).
    pub name: &'static str,
    pub makes: ItemKind,
    /// Materials used up, with how many of each.
    pub needs: &'static [(ItemKind, u32)],
    /// Tools that must be carried to make it; they aren't used up.
    pub tools: &'static [ItemKind],
    /// Only made beside a burning campfire, by holding the first material
    /// over the flames.
    pub at_fire: bool,
    /// Only made beside the wood lathe (set by the water wheel), which
    /// turns it round.
    pub at_lathe: bool,
}

pub const RECIPES: &[Recipe] = &[
    Recipe { name: "StoneKnife", makes: ItemKind::StoneKnife, needs: &[(ItemKind::Stone, 2)], tools: &[], at_fire: false, at_lathe: false },
    Recipe {
        name: "OpenedCoconut",
        makes: ItemKind::OpenedCoconut,
        needs: &[(ItemKind::Coconut, 1)],
        tools: &[ItemKind::StoneKnife],
        at_fire: false,
        at_lathe: false,
    },
    Recipe {
        name: "StoneAxe",
        makes: ItemKind::StoneAxe,
        needs: &[(ItemKind::Stone, 1), (ItemKind::Rope, 1), (ItemKind::Branch, 1)],
        tools: &[],
        at_fire: false,
        at_lathe: false,
    },
    Recipe {
        name: "WoodSpear",
        makes: ItemKind::WoodSpear,
        needs: &[(ItemKind::Branch, 1), (ItemKind::Stone, 1)],
        tools: &[],
        at_fire: false,
        at_lathe: false,
    },
    Recipe {
        name: "ShellHook",
        makes: ItemKind::ShellHook,
        needs: &[(ItemKind::SeaShell, 1)],
        tools: &[ItemKind::StoneKnife],
        at_fire: false,
        at_lathe: false,
    },
    Recipe {
        name: "FishingRod",
        makes: ItemKind::FishingRod,
        needs: &[(ItemKind::Branch, 1), (ItemKind::Rope, 1), (ItemKind::ShellHook, 1)],
        tools: &[],
        at_fire: false,
        at_lathe: false,
    },
    Recipe { name: "GrilledCrab", makes: ItemKind::GrilledCrab, needs: &[(ItemKind::Crab, 1)], tools: &[ItemKind::StoneKnife], at_fire: true, at_lathe: false },
    Recipe { name: "GrilledFish", makes: ItemKind::GrilledFish, needs: &[(ItemKind::Fish, 1)], tools: &[ItemKind::StoneKnife], at_fire: true, at_lathe: false },
    Recipe {
        name: "BarkContainer",
        makes: ItemKind::BarkContainer,
        needs: &[(ItemKind::Bark, 2), (ItemKind::Vine, 1)],
        tools: &[],
        at_fire: false,
        at_lathe: false,
    },
    Recipe {
        name: "BoiledWater",
        makes: ItemKind::BoiledWater,
        needs: &[(ItemKind::BarkContainer, 1), (ItemKind::RawWater, 1)],
        tools: &[],
        at_fire: true,
        at_lathe: false,
    },
    Recipe { name: "Torch", makes: ItemKind::Torch, needs: &[(ItemKind::Branch, 1), (ItemKind::Husk, 1)], tools: &[], at_fire: true, at_lathe: false },
    Recipe { name: "Rope", makes: ItemKind::Rope, needs: &[(ItemKind::Husk, 2)], tools: &[], at_fire: false, at_lathe: false },
    Recipe { name: "RopeFromBark", makes: ItemKind::Rope, needs: &[(ItemKind::Bark, 3)], tools: &[ItemKind::StoneKnife], at_fire: false, at_lathe: false },
    Recipe { name: "Thatch", makes: ItemKind::Thatch, needs: &[(ItemKind::PalmFrond, 4), (ItemKind::Vine, 1)], tools: &[], at_fire: false, at_lathe: false },
    Recipe {
        name: "Shelter",
        makes: ItemKind::Shelter,
        needs: &[(ItemKind::Branch, 4), (ItemKind::Rope, 2), (ItemKind::Thatch, 2)],
        tools: &[],
        at_fire: false,
        at_lathe: false,
    },
    Recipe {
        name: "PackFrame",
        makes: ItemKind::PackFrame,
        needs: &[(ItemKind::Branch, 4), (ItemKind::Rope, 2), (ItemKind::Bark, 2)],
        tools: &[],
        at_fire: false,
        at_lathe: false,
    },
    Recipe { name: "FireDrill", makes: ItemKind::FireDrill, needs: &[(ItemKind::Branch, 2), (ItemKind::Vine, 1)], tools: &[], at_fire: false, at_lathe: false },
    Recipe {
        name: "Ladder",
        makes: ItemKind::Ladder,
        needs: &[(ItemKind::LauanTimber, 2), (ItemKind::Branch, 3), (ItemKind::Rope, 2)],
        tools: &[ItemKind::StoneAxe],
        at_fire: false,
        at_lathe: false,
    },
    Recipe {
        name: "LogBridge",
        makes: ItemKind::LogBridge,
        needs: &[(ItemKind::PineTimber, 2), (ItemKind::Rope, 2)],
        tools: &[ItemKind::StoneAxe],
        at_fire: false,
        at_lathe: false,
    },
    Recipe {
        name: "WaterWheel",
        makes: ItemKind::WaterWheel,
        needs: &[(ItemKind::LauanTimber, 4), (ItemKind::Bamboo, 4), (ItemKind::Stone, 2), (ItemKind::Rope, 2)],
        tools: &[ItemKind::StoneAxe, ItemKind::StoneKnife],
        at_fire: false,
        at_lathe: false,
    },
    Recipe {
        name: "DiggingStick",
        makes: ItemKind::DiggingStick,
        needs: &[(ItemKind::Branch, 1)],
        tools: &[ItemKind::StoneKnife],
        at_fire: false,
        at_lathe: false,
    },
    Recipe {
        name: "BambooSpade",
        makes: ItemKind::BambooSpade,
        needs: &[(ItemKind::Bamboo, 1), (ItemKind::Rope, 1)],
        tools: &[ItemKind::StoneAxe, ItemKind::StoneKnife],
        at_fire: false,
        at_lathe: false,
    },
    Recipe {
        name: "ToolBelt",
        makes: ItemKind::ToolBelt,
        needs: &[(ItemKind::Bark, 2), (ItemKind::Rope, 1)],
        tools: &[ItemKind::StoneKnife],
        at_fire: false,
        at_lathe: false,
    },
    Recipe {
        name: "WoodLathe",
        makes: ItemKind::WoodLathe,
        needs: &[(ItemKind::LauanTimber, 2), (ItemKind::Branch, 2), (ItemKind::Stone, 2), (ItemKind::Rope, 3)],
        tools: &[ItemKind::StoneAxe, ItemKind::StoneKnife],
        at_fire: false,
        at_lathe: false,
    },
    Recipe {
        name: "SpokedWheel",
        makes: ItemKind::SpokedWheel,
        needs: &[(ItemKind::LauanTimber, 1), (ItemKind::Branch, 4)],
        tools: &[ItemKind::StoneKnife],
        at_fire: false,
        at_lathe: true,
    },
    Recipe {
        name: "Handcart",
        makes: ItemKind::Handcart,
        needs: &[(ItemKind::SpokedWheel, 2), (ItemKind::LauanTimber, 2), (ItemKind::Bamboo, 2), (ItemKind::Rope, 2)],
        tools: &[ItemKind::StoneAxe],
        at_fire: false,
        at_lathe: false,
    },
    Recipe {
        name: "SignalFire",
        makes: ItemKind::SignalFire,
        needs: &[(ItemKind::Stone, 4), (ItemKind::Timber, 2), (ItemKind::Soil, 2)],
        tools: &[],
        at_fire: false,
        at_lathe: false,
    },
];

/// The recipes the castaway has thought of. A recipe comes to mind the
/// first time one of each of its materials is carried at once (tools and
/// the fire aren't needed for that: finding out they're missing is a hint
/// in itself), and stays known. One not yet looked at is fresh.
///
/// Some items can be made more than one way (rope from husks or from
/// bark). Such an item is listed once, showing the way last made with it
/// (or the first known); the others are a step away on its page.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(into = "KnownRecipes", from = "KnownRecipes")]
pub struct RecipeBook {
    known: Vec<bool>,
    fresh: Vec<bool>,
    /// The way last made, for items made more than one way.
    chosen: Vec<bool>,
    /// The ways that have made something at least once (what the end
    /// counts as kinds of things made).
    used: Vec<bool>,
    /// How the CRAFT list is sorted (the last order chosen).
    order: RecipeOrder,
}

/// The orders the CRAFT list can be sorted in. Each but A to Z puts some
/// recipes first; both parts are A to Z by the name of what's made.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum RecipeOrder {
    #[default]
    AToZ,
    /// What can be made here and now first (the lit rows).
    CanMake,
    /// What hasn't been looked at yet first (the rows marked "!").
    New,
    /// What has never been made first.
    NotMade,
}

impl RecipeOrder {
    /// In the order the sort menu offers them.
    pub const ALL: [Self; 4] = [Self::AToZ, Self::CanMake, Self::New, Self::NotMade];

    pub fn label(self) -> &'static str {
        match self {
            Self::AToZ => "A TO Z",
            Self::CanMake => "CAN MAKE",
            Self::New => "NEW",
            Self::NotMade => "NOT MADE",
        }
    }
}

impl Default for RecipeBook {
    fn default() -> Self {
        let none = vec![false; RECIPES.len()];
        Self { known: none.clone(), fresh: none.clone(), chosen: none.clone(), used: none, order: RecipeOrder::AToZ }
    }
}

/// How many kinds of thing there are to make (an item made more than one
/// way is one kind).
pub fn makeable_kinds() -> usize {
    RECIPES.iter().enumerate().filter(|&(i, r)| RECIPES[..i].iter().all(|q| q.makes != r.makes)).count()
}

/// How a recipe book is saved: the recipes known, those not yet looked at,
/// the ways last made and the ways that have made something, by name (so
/// adding recipes or reordering them leaves a save as it was), and the
/// order the list is sorted in.
#[derive(serde::Serialize, serde::Deserialize)]
struct KnownRecipes {
    known: Vec<String>,
    fresh: Vec<String>,
    #[serde(default)]
    chosen: Vec<String>,
    made: Vec<String>,
    sort: RecipeOrder,
}

impl From<RecipeBook> for KnownRecipes {
    fn from(b: RecipeBook) -> Self {
        let pick = |flags: &[bool]| RECIPES.iter().zip(flags).filter(|&(_, &on)| on).map(|(r, _)| r.name.to_string()).collect();
        Self { known: pick(&b.known), fresh: pick(&b.fresh), chosen: pick(&b.chosen), made: pick(&b.used), sort: b.order }
    }
}

impl From<KnownRecipes> for RecipeBook {
    fn from(k: KnownRecipes) -> Self {
        let flags = |names: &[String]| RECIPES.iter().map(|r| names.iter().any(|n| n == r.name)).collect();
        Self { known: flags(&k.known), fresh: flags(&k.fresh), chosen: flags(&k.chosen), used: flags(&k.made), order: k.sort }
    }
}

impl RecipeBook {
    /// Learns every recipe whose materials `has` all says are carried;
    /// whether any was new.
    pub fn notice(&mut self, has: impl Fn(ItemKind) -> bool) -> bool {
        let mut found = false;
        for (i, r) in RECIPES.iter().enumerate() {
            if !self.known[i] && r.needs.iter().all(|&(kind, _)| has(kind)) {
                self.known[i] = true;
                self.fresh[i] = true;
                found = true;
            }
        }
        found
    }

    /// The list of things that can be made: for each item with a known
    /// recipe, in recipe order, the index into `RECIPES` of the way shown.
    pub fn listed(&self) -> Vec<usize> {
        let mut out: Vec<usize> = Vec::new();
        for i in (0..RECIPES.len()).filter(|&i| self.known[i]) {
            match out.iter_mut().find(|j| RECIPES[**j].makes == RECIPES[i].makes) {
                Some(j) if self.chosen[i] => *j = i,
                Some(_) => {}
                None => out.push(i),
            }
        }
        out
    }

    /// The list as the CRAFT page shows it: `listed`, sorted in the book's
    /// order. `ready` says whether a recipe (a way) can be made now; an
    /// item can if any of its ways can.
    pub fn sorted(&self, ready: impl Fn(usize) -> bool) -> Vec<usize> {
        let mut out = self.listed();
        let first = |i: usize| match self.order {
            RecipeOrder::AToZ => true,
            RecipeOrder::CanMake => self.ways(i).into_iter().any(&ready),
            RecipeOrder::New => self.item_fresh(i),
            RecipeOrder::NotMade => !self.ways(i).iter().any(|&j| self.used[j]),
        };
        out.sort_by_cached_key(|&i| (!first(i), RECIPES[i].makes.name()));
        out
    }

    pub fn order(&self) -> RecipeOrder {
        self.order
    }

    pub fn set_order(&mut self, order: RecipeOrder) {
        self.order = order;
    }

    /// The known ways of making what recipe `i` makes, in recipe order.
    pub fn ways(&self, i: usize) -> Vec<usize> {
        let makes = RECIPES[i].makes;
        (0..RECIPES.len()).filter(|&j| self.known[j] && RECIPES[j].makes == makes).collect()
    }

    /// Whether the recipe hasn't been looked at since it came to mind.
    pub fn is_fresh(&self, i: usize) -> bool {
        self.fresh[i]
    }

    /// Whether any way of making what recipe `i` makes is fresh.
    pub fn item_fresh(&self, i: usize) -> bool {
        self.ways(i).iter().any(|&j| self.fresh[j])
    }

    /// Whether any recipe hasn't been looked at yet (the CRAFT tab is
    /// marked).
    pub fn any_fresh(&self) -> bool {
        self.fresh.contains(&true)
    }

    pub fn looked_at(&mut self, i: usize) {
        self.fresh[i] = false;
    }

    /// How many kinds of thing have been made (an item made more than one
    /// way is one kind).
    pub fn kinds_made(&self) -> usize {
        RECIPES.iter().enumerate().filter(|&(i, r)| self.used[i] && (0..i).all(|j| !self.used[j] || RECIPES[j].makes != r.makes)).count()
    }

    /// How many recipes have come to mind.
    pub fn known_count(&self) -> usize {
        self.known.iter().filter(|&&k| k).count()
    }

    /// Something was made by recipe `i`: that way is shown for the item
    /// from now on, and the item counts as made.
    pub fn made(&mut self, i: usize) {
        self.used[i] = true;
        let makes = RECIPES[i].makes;
        for (j, r) in RECIPES.iter().enumerate() {
            if r.makes == makes {
                self.chosen[j] = j == i;
            }
        }
    }

    /// Debug: knows every recipe (none of them fresh).
    pub fn learn_all(&mut self) {
        self.known.fill(true);
        self.fresh.fill(false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn index(makes: ItemKind) -> usize {
        RECIPES.iter().position(|r| r.makes == makes).unwrap()
    }

    #[test]
    fn nothing_is_made_at_first() {
        assert_eq!(RecipeBook::default().kinds_made(), 0);
    }

    #[test]
    fn making_the_same_thing_again_counts_as_one_kind() {
        let mut book = RecipeBook::default();
        book.made(index(ItemKind::StoneKnife));
        book.made(index(ItemKind::StoneKnife));
        assert_eq!(book.kinds_made(), 1);
        book.made(index(ItemKind::StoneAxe));
        assert_eq!(book.kinds_made(), 2);
    }

    #[test]
    fn rope_made_both_ways_is_one_kind() {
        let mut book = RecipeBook::default();
        for (i, r) in RECIPES.iter().enumerate().filter(|(_, r)| r.makes == ItemKind::Rope) {
            book.made(i);
            assert_eq!(book.kinds_made(), 1, "{}", r.name);
        }
    }

    #[test]
    fn the_kinds_there_are_to_make_count_each_item_once() {
        assert!(makeable_kinds() < RECIPES.len(), "rope is made two ways");
        let mut book = RecipeBook::default();
        for i in 0..RECIPES.len() {
            book.made(i);
        }
        assert_eq!(book.kinds_made(), makeable_kinds());
    }

    #[test]
    fn the_recipes_known_are_counted() {
        let mut book = RecipeBook::default();
        assert_eq!(book.known_count(), 0);
        book.notice(carrying(&[ItemKind::Stone]));
        assert!(book.known_count() >= 1);
        book.learn_all();
        assert_eq!(book.known_count(), RECIPES.len());
    }

    #[test]
    fn what_was_made_is_kept_in_a_save_by_recipe_name() {
        let mut book = RecipeBook::default();
        book.made(index(ItemKind::StoneKnife));
        let json = serde_json::to_value(&book).unwrap();
        assert_eq!(json["made"], serde_json::json!(["StoneKnife"]));
        let again: RecipeBook = serde_json::from_value(json).unwrap();
        assert_eq!(again.kinds_made(), 1);
    }

    fn carrying(kinds: &[ItemKind]) -> impl Fn(ItemKind) -> bool + '_ {
        move |k| kinds.contains(&k)
    }

    #[test]
    fn nothing_comes_to_mind_at_first() {
        assert!(RecipeBook::default().listed().is_empty());
    }

    #[test]
    fn a_recipe_comes_to_mind_when_one_of_each_material_is_carried() {
        // A stone knife takes two stones, but one is enough to think of it.
        let mut book = RecipeBook::default();
        assert!(book.notice(carrying(&[ItemKind::Stone])));
        assert!(book.listed().contains(&index(ItemKind::StoneKnife)));
    }

    #[test]
    fn a_recipe_does_not_come_to_mind_with_some_materials_missing() {
        // A stone axe takes a stone, a rope and a branch.
        let mut book = RecipeBook::default();
        book.notice(carrying(&[ItemKind::Stone, ItemKind::Branch]));
        assert!(!book.listed().contains(&index(ItemKind::StoneAxe)));
        book.notice(carrying(&[ItemKind::Rope]));
        assert!(!book.listed().contains(&index(ItemKind::StoneAxe)), "the materials must be carried at once");
        book.notice(carrying(&[ItemKind::Stone, ItemKind::Rope, ItemKind::Branch]));
        assert!(book.listed().contains(&index(ItemKind::StoneAxe)));
    }

    #[test]
    fn tools_and_the_fire_are_not_needed_to_think_of_a_recipe() {
        // Grilled crab needs a knife and a fire; the crab alone brings it to mind.
        let grilled = &RECIPES[index(ItemKind::GrilledCrab)];
        assert!(!grilled.tools.is_empty() && grilled.at_fire);
        let mut book = RecipeBook::default();
        book.notice(carrying(&[ItemKind::Crab]));
        assert!(book.listed().contains(&index(ItemKind::GrilledCrab)));
    }

    #[test]
    fn a_recipe_once_thought_of_stays_known() {
        let mut book = RecipeBook::default();
        book.notice(carrying(&[ItemKind::Stone]));
        book.notice(carrying(&[]));
        assert!(book.listed().contains(&index(ItemKind::StoneKnife)));
    }

    #[test]
    fn noticing_says_whether_anything_new_came_to_mind() {
        let mut book = RecipeBook::default();
        assert!(book.notice(carrying(&[ItemKind::Stone])));
        assert!(!book.notice(carrying(&[ItemKind::Stone])));
    }

    #[test]
    fn a_new_recipe_is_fresh_until_looked_at() {
        let mut book = RecipeBook::default();
        book.notice(carrying(&[ItemKind::Stone]));
        let i = index(ItemKind::StoneKnife);
        assert!(book.is_fresh(i));
        book.looked_at(i);
        assert!(!book.is_fresh(i));
        book.notice(carrying(&[ItemKind::Stone]));
        assert!(!book.is_fresh(i), "thinking of it again doesn't make it fresh");
    }

    #[test]
    fn the_craft_tab_is_marked_while_any_recipe_is_fresh() {
        let mut book = RecipeBook::default();
        assert!(!book.any_fresh(), "nothing thought of yet");
        book.notice(carrying(&[ItemKind::Stone]));
        assert!(book.any_fresh());
        book.looked_at(index(ItemKind::StoneKnife));
        assert!(!book.any_fresh());
    }

    #[test]
    fn a_log_bridge_comes_to_mind_with_pine_timber_and_rope() {
        let mut book = RecipeBook::default();
        book.notice(carrying(&[ItemKind::PineTimber, ItemKind::Rope]));
        assert!(book.listed().contains(&index(ItemKind::LogBridge)));
    }

    #[test]
    fn a_log_bridge_is_shaped_with_the_stone_axe() {
        assert!(RECIPES[index(ItemKind::LogBridge)].tools.contains(&ItemKind::StoneAxe));
    }

    #[test]
    fn a_water_wheel_comes_to_mind_with_lauan_timber_bamboo_stone_and_rope() {
        let mut book = RecipeBook::default();
        book.notice(carrying(&[ItemKind::Bamboo, ItemKind::LauanTimber, ItemKind::Stone, ItemKind::Rope]));
        assert!(book.listed().contains(&index(ItemKind::WaterWheel)));
    }

    #[test]
    fn a_water_wheel_comes_with_stone_bearings_on_posts_to_hold_its_axle() {
        let needs = RECIPES[index(ItemKind::WaterWheel)].needs;
        let count = |kind| needs.iter().find(|&&(k, _)| k == kind).map_or(0, |&(_, n)| n);
        assert!(count(ItemKind::Stone) >= 2, "a bearing at each end of the axle");
        assert!(count(ItemKind::LauanTimber) > 3, "lauan for the posts besides the wheel");
    }

    #[test]
    fn a_water_wheel_is_cut_with_the_axe_and_split_with_the_knife() {
        let tools = RECIPES[index(ItemKind::WaterWheel)].tools;
        assert!(tools.contains(&ItemKind::StoneAxe) && tools.contains(&ItemKind::StoneKnife));
    }

    #[test]
    fn a_wood_lathe_is_lauan_with_branches_stones_and_rope_for_its_drive() {
        let recipe = &RECIPES[index(ItemKind::WoodLathe)];
        let count = |kind| recipe.needs.iter().find(|&&(k, _)| k == kind).map_or(0, |&(_, n)| n);
        assert!(count(ItemKind::LauanTimber) > 0 && count(ItemKind::Branch) > 0 && count(ItemKind::Stone) > 0);
        assert!(count(ItemKind::Rope) > count(ItemKind::LauanTimber), "most of the rope drives it off the wheel");
        assert!(recipe.tools.contains(&ItemKind::StoneAxe) && recipe.tools.contains(&ItemKind::StoneKnife));
        assert!(!recipe.at_lathe, "it is made before there is a lathe");
    }

    #[test]
    fn a_spoked_wheel_is_turned_on_the_lathe() {
        let recipe = &RECIPES[index(ItemKind::SpokedWheel)];
        assert!(recipe.at_lathe && !recipe.at_fire);
        assert!(recipe.needs.iter().any(|&(k, _)| k == ItemKind::LauanTimber));
        assert!(recipe.needs.iter().any(|&(k, n)| k == ItemKind::Branch && n > 1), "branches turned into spokes");
    }

    #[test]
    fn the_lathe_is_not_needed_to_think_of_a_recipe_made_at_it() {
        let mut book = RecipeBook::default();
        book.notice(carrying(&[ItemKind::LauanTimber, ItemKind::Branch]));
        assert!(book.listed().contains(&index(ItemKind::SpokedWheel)));
    }

    #[test]
    fn a_handcart_rolls_on_two_spoked_wheels_and_is_shaped_with_the_axe() {
        let recipe = &RECIPES[index(ItemKind::Handcart)];
        assert!(recipe.needs.contains(&(ItemKind::SpokedWheel, 2)));
        assert!(recipe.needs.iter().any(|&(k, _)| k == ItemKind::Bamboo), "bamboo poles to pull it by");
        assert!(recipe.tools.contains(&ItemKind::StoneAxe));
        assert!(!ItemKind::Handcart.goes_in_bag());
    }

    #[test]
    fn a_digging_stick_is_a_branch_whittled_to_a_point_with_the_knife() {
        let recipe = RECIPES[index(ItemKind::DiggingStick)];
        assert_eq!(recipe.needs, &[(ItemKind::Branch, 1)]);
        assert!(recipe.tools.contains(&ItemKind::StoneKnife));
    }

    #[test]
    fn a_bamboo_spade_comes_to_mind_with_bamboo_and_rope() {
        let mut book = RecipeBook::default();
        book.notice(carrying(&[ItemKind::Bamboo, ItemKind::Rope]));
        assert!(book.listed().contains(&index(ItemKind::BambooSpade)));
    }

    #[test]
    fn a_bamboo_spade_is_cut_with_the_axe_and_split_with_the_knife() {
        let tools = RECIPES[index(ItemKind::BambooSpade)].tools;
        assert!(tools.contains(&ItemKind::StoneAxe) && tools.contains(&ItemKind::StoneKnife));
    }

    #[test]
    fn known_recipes_are_listed_in_recipe_order() {
        let mut book = RecipeBook::default();
        book.learn_all();
        let firsts: Vec<usize> = (0..RECIPES.len()).filter(|&i| index(RECIPES[i].makes) == i).collect();
        assert_eq!(book.listed(), firsts, "one entry per item, at its first way");
        assert!((0..RECIPES.len()).all(|i| !book.is_fresh(i)));
    }

    #[test]
    fn a_saved_book_keeps_recipes_by_name() {
        let mut book = RecipeBook::default();
        book.notice(carrying(&[ItemKind::Stone]));
        book.looked_at(index(ItemKind::StoneKnife));
        let json: serde_json::Value = serde_json::to_value(&book).unwrap();
        assert!(json["known"].as_array().unwrap().contains(&serde_json::json!("StoneKnife")));
        assert!(!json["fresh"].as_array().unwrap().contains(&serde_json::json!("StoneKnife")));
        assert_eq!(serde_json::from_value::<RecipeBook>(json).unwrap(), book);
    }

    fn named(name: &str) -> usize {
        RECIPES.iter().position(|r| r.name == name).unwrap()
    }

    #[test]
    fn rope_can_be_twisted_from_bark_split_with_the_stone_knife() {
        let bark = &RECIPES[named("RopeFromBark")];
        assert_eq!(bark.makes, ItemKind::Rope);
        assert!(bark.needs.iter().all(|&(k, _)| k == ItemKind::Bark));
        assert!(bark.tools.contains(&ItemKind::StoneKnife));
        assert!(!bark.at_fire);
    }

    #[test]
    fn rope_takes_more_bark_than_husks() {
        let need = |name: &str| RECIPES[named(name)].needs[0].1;
        assert!(need("RopeFromBark") > need("Rope"));
    }

    #[test]
    fn carrying_bark_brings_the_bark_rope_to_mind() {
        let mut book = RecipeBook::default();
        book.notice(carrying(&[ItemKind::Bark]));
        assert_eq!(book.ways(book.listed()[0]), vec![named("RopeFromBark")]);
    }

    #[test]
    fn an_item_with_two_ways_of_making_it_is_listed_once() {
        let mut book = RecipeBook::default();
        book.notice(carrying(&[ItemKind::Husk, ItemKind::Bark]));
        let ropes = book.listed().into_iter().filter(|&i| RECIPES[i].makes == ItemKind::Rope).count();
        assert_eq!(ropes, 1);
    }

    #[test]
    fn the_ways_of_making_an_item_are_the_known_ones_in_recipe_order() {
        let mut book = RecipeBook::default();
        book.notice(carrying(&[ItemKind::Bark]));
        book.notice(carrying(&[ItemKind::Husk]));
        let husk = named("Rope");
        let bark = named("RopeFromBark");
        assert_eq!(book.ways(husk), vec![husk, bark]);
        assert_eq!(book.ways(bark), vec![husk, bark]);
    }

    #[test]
    fn the_first_known_way_is_shown_until_another_is_made() {
        let mut book = RecipeBook::default();
        book.notice(carrying(&[ItemKind::Husk, ItemKind::Bark]));
        let shown = |b: &RecipeBook| b.listed().into_iter().find(|&i| RECIPES[i].makes == ItemKind::Rope).unwrap();
        assert_eq!(shown(&book), named("Rope"));
        book.made(named("RopeFromBark"));
        assert_eq!(shown(&book), named("RopeFromBark"));
        book.made(named("Rope"));
        assert_eq!(shown(&book), named("Rope"));
    }

    #[test]
    fn an_item_is_fresh_in_the_list_while_any_of_its_ways_is() {
        let mut book = RecipeBook::default();
        book.notice(carrying(&[ItemKind::Husk]));
        book.looked_at(named("Rope"));
        assert!(!book.item_fresh(named("Rope")));
        book.notice(carrying(&[ItemKind::Bark]));
        assert!(book.item_fresh(named("Rope")), "a new way to make it is news");
        assert!(!book.is_fresh(named("Rope")));
        book.looked_at(named("RopeFromBark"));
        assert!(!book.item_fresh(named("Rope")));
    }

    #[test]
    fn a_saved_book_keeps_the_way_last_made() {
        let mut book = RecipeBook::default();
        book.notice(carrying(&[ItemKind::Husk, ItemKind::Bark]));
        book.made(named("RopeFromBark"));
        let json = serde_json::to_value(&book).unwrap();
        assert!(json["known"].as_array().unwrap().contains(&serde_json::json!("RopeFromBark")));
        assert_eq!(serde_json::from_value::<RecipeBook>(json).unwrap(), book);
    }

    #[test]
    fn every_recipe_has_its_own_name() {
        for (i, r) in RECIPES.iter().enumerate() {
            assert_eq!(named(r.name), i, "{} names two recipes", r.name);
        }
    }

    /// The names on the list, top to bottom, in the book's order, with
    /// `ready` saying which recipes can be made now.
    fn shown(book: &RecipeBook, ready: impl Fn(usize) -> bool) -> Vec<&'static str> {
        book.sorted(ready).iter().map(|&i| RECIPES[i].makes.name()).collect()
    }

    fn a_to_z(names: &[&'static str]) -> bool {
        names.is_sorted()
    }

    #[test]
    fn the_list_is_a_to_z_at_first() {
        let mut book = RecipeBook::default();
        book.learn_all();
        assert_eq!(book.order(), RecipeOrder::AToZ);
        let names = shown(&book, |_| false);
        assert!(a_to_z(&names), "{names:?}");
        assert_eq!(names.len(), book.listed().len(), "one row per item");
    }

    #[test]
    fn can_make_lists_what_can_be_made_now_first_each_part_a_to_z() {
        let mut book = RecipeBook::default();
        book.learn_all();
        book.set_order(RecipeOrder::CanMake);
        let ready = |i: usize| matches!(RECIPES[i].makes, ItemKind::WoodSpear | ItemKind::Torch);
        let names = shown(&book, ready);
        assert_eq!(names[..2], ["TORCH", "WOOD SPEAR"]);
        assert!(a_to_z(&names[2..]), "{names:?}");
    }

    #[test]
    fn can_make_puts_an_item_first_if_any_of_its_ways_can_be_made() {
        let mut book = RecipeBook::default();
        book.learn_all();
        book.set_order(RecipeOrder::CanMake);
        assert_eq!(shown(&book, |i| RECIPES[i].name == "RopeFromBark")[0], "ROPE");
    }

    #[test]
    fn new_lists_recipes_not_yet_looked_at_first_each_part_a_to_z() {
        let mut book = RecipeBook::default();
        book.notice(|_| true);
        for (i, r) in RECIPES.iter().enumerate() {
            if !matches!(r.makes, ItemKind::Thatch | ItemKind::FireDrill) {
                book.looked_at(i);
            }
        }
        book.set_order(RecipeOrder::New);
        let names = shown(&book, |_| false);
        assert_eq!(names[..2], ["FIRE DRILL", "THATCH"]);
        assert!(a_to_z(&names[2..]), "{names:?}");
    }

    #[test]
    fn not_made_lists_what_has_never_been_made_first_each_part_a_to_z() {
        let mut book = RecipeBook::default();
        book.learn_all();
        book.made(index(ItemKind::StoneKnife));
        book.made(named("RopeFromBark"));
        book.set_order(RecipeOrder::NotMade);
        let names = shown(&book, |_| false);
        let n = names.len();
        assert_eq!(names[n - 2..], ["ROPE", "STONE KNIFE"], "rope made either way counts as made");
        assert!(a_to_z(&names[..n - 2]), "{names:?}");
    }

    #[test]
    fn a_saved_book_keeps_its_order() {
        let mut book = RecipeBook::default();
        book.set_order(RecipeOrder::NotMade);
        let json = serde_json::to_value(&book).unwrap();
        assert_eq!(json["sort"], serde_json::json!("NotMade"));
        assert_eq!(serde_json::from_value::<RecipeBook>(json).unwrap(), book);
    }

    #[test]
    fn no_recipe_needs_what_it_makes() {
        for r in RECIPES {
            assert!(r.needs.iter().all(|&(k, n)| k != r.makes && n > 0), "{:?}", r.makes);
            assert!(!r.tools.contains(&r.makes), "{:?}", r.makes);
        }
    }
}

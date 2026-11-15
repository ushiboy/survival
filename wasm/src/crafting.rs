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
}

pub const RECIPES: &[Recipe] = &[
    Recipe { name: "StoneKnife", makes: ItemKind::StoneKnife, needs: &[(ItemKind::Stone, 2)], tools: &[], at_fire: false },
    Recipe { name: "OpenedCoconut", makes: ItemKind::OpenedCoconut, needs: &[(ItemKind::Coconut, 1)], tools: &[ItemKind::StoneKnife], at_fire: false },
    Recipe {
        name: "StoneAxe",
        makes: ItemKind::StoneAxe,
        needs: &[(ItemKind::Stone, 1), (ItemKind::Rope, 1), (ItemKind::Branch, 1)],
        tools: &[],
        at_fire: false,
    },
    Recipe { name: "WoodSpear", makes: ItemKind::WoodSpear, needs: &[(ItemKind::Branch, 1), (ItemKind::Stone, 1)], tools: &[], at_fire: false },
    Recipe { name: "ShellHook", makes: ItemKind::ShellHook, needs: &[(ItemKind::SeaShell, 1)], tools: &[ItemKind::StoneKnife], at_fire: false },
    Recipe {
        name: "FishingRod",
        makes: ItemKind::FishingRod,
        needs: &[(ItemKind::Branch, 1), (ItemKind::Rope, 1), (ItemKind::ShellHook, 1)],
        tools: &[],
        at_fire: false,
    },
    Recipe { name: "GrilledCrab", makes: ItemKind::GrilledCrab, needs: &[(ItemKind::Crab, 1)], tools: &[ItemKind::StoneKnife], at_fire: true },
    Recipe { name: "GrilledFish", makes: ItemKind::GrilledFish, needs: &[(ItemKind::Fish, 1)], tools: &[ItemKind::StoneKnife], at_fire: true },
    Recipe { name: "BarkContainer", makes: ItemKind::BarkContainer, needs: &[(ItemKind::Bark, 2), (ItemKind::Vine, 1)], tools: &[], at_fire: false },
    Recipe { name: "BoiledWater", makes: ItemKind::BoiledWater, needs: &[(ItemKind::BarkContainer, 1), (ItemKind::RawWater, 1)], tools: &[], at_fire: true },
    Recipe { name: "Torch", makes: ItemKind::Torch, needs: &[(ItemKind::Branch, 1), (ItemKind::Husk, 1)], tools: &[], at_fire: true },
    Recipe { name: "Rope", makes: ItemKind::Rope, needs: &[(ItemKind::Husk, 2)], tools: &[], at_fire: false },
    Recipe { name: "RopeFromBark", makes: ItemKind::Rope, needs: &[(ItemKind::Bark, 3)], tools: &[ItemKind::StoneKnife], at_fire: false },
    Recipe { name: "Thatch", makes: ItemKind::Thatch, needs: &[(ItemKind::PalmFrond, 4), (ItemKind::Vine, 1)], tools: &[], at_fire: false },
    Recipe {
        name: "Shelter",
        makes: ItemKind::Shelter,
        needs: &[(ItemKind::Branch, 4), (ItemKind::Rope, 2), (ItemKind::Thatch, 2)],
        tools: &[],
        at_fire: false,
    },
    Recipe {
        name: "PackFrame",
        makes: ItemKind::PackFrame,
        needs: &[(ItemKind::Branch, 4), (ItemKind::Rope, 2), (ItemKind::Bark, 2)],
        tools: &[],
        at_fire: false,
    },
    Recipe { name: "FireDrill", makes: ItemKind::FireDrill, needs: &[(ItemKind::Branch, 2), (ItemKind::Vine, 1)], tools: &[], at_fire: false },
    Recipe {
        name: "Ladder",
        makes: ItemKind::Ladder,
        needs: &[(ItemKind::LauanTimber, 2), (ItemKind::Branch, 3), (ItemKind::Rope, 2)],
        tools: &[ItemKind::StoneAxe],
        at_fire: false,
    },
    Recipe {
        name: "LogBridge",
        makes: ItemKind::LogBridge,
        needs: &[(ItemKind::PineTimber, 2), (ItemKind::Rope, 2)],
        tools: &[ItemKind::StoneAxe],
        at_fire: false,
    },
    Recipe {
        name: "WaterWheel",
        makes: ItemKind::WaterWheel,
        needs: &[(ItemKind::LauanTimber, 4), (ItemKind::Bamboo, 4), (ItemKind::Stone, 2), (ItemKind::Rope, 2)],
        tools: &[ItemKind::StoneAxe, ItemKind::StoneKnife],
        at_fire: false,
    },
    Recipe { name: "DiggingStick", makes: ItemKind::DiggingStick, needs: &[(ItemKind::Branch, 1)], tools: &[ItemKind::StoneKnife], at_fire: false },
    Recipe {
        name: "BambooSpade",
        makes: ItemKind::BambooSpade,
        needs: &[(ItemKind::Bamboo, 1), (ItemKind::Rope, 1)],
        tools: &[ItemKind::StoneAxe, ItemKind::StoneKnife],
        at_fire: false,
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
}

impl Default for RecipeBook {
    fn default() -> Self {
        let none = vec![false; RECIPES.len()];
        Self { known: none.clone(), fresh: none.clone(), chosen: none }
    }
}

/// How a recipe book is saved: the recipes known, those not yet looked at
/// and the ways last made, by name (so adding recipes or reordering them
/// leaves a save as it was).
#[derive(serde::Serialize, serde::Deserialize)]
struct KnownRecipes {
    known: Vec<String>,
    fresh: Vec<String>,
    #[serde(default)]
    chosen: Vec<String>,
}

impl From<RecipeBook> for KnownRecipes {
    fn from(b: RecipeBook) -> Self {
        let pick = |flags: &[bool]| RECIPES.iter().zip(flags).filter(|&(_, &on)| on).map(|(r, _)| r.name.to_string()).collect();
        Self { known: pick(&b.known), fresh: pick(&b.fresh), chosen: pick(&b.chosen) }
    }
}

impl From<KnownRecipes> for RecipeBook {
    fn from(k: KnownRecipes) -> Self {
        let flags = |names: &[String]| RECIPES.iter().map(|r| names.iter().any(|n| n == r.name)).collect();
        Self { known: flags(&k.known), fresh: flags(&k.fresh), chosen: flags(&k.chosen) }
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

    /// Something was made by recipe `i`: that way is shown for the item
    /// from now on.
    pub fn made(&mut self, i: usize) {
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

    #[test]
    fn no_recipe_needs_what_it_makes() {
        for r in RECIPES {
            assert!(r.needs.iter().all(|&(k, n)| k != r.makes && n > 0), "{:?}", r.makes);
            assert!(!r.tools.contains(&r.makes), "{:?}", r.makes);
        }
    }
}

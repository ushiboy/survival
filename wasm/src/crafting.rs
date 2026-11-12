//! Recipes for making things out of gathered materials. Crafting happens in
//! the bag screen and uses up materials carried in the bag; some recipes
//! can only be made beside a campfire.

use crate::items::ItemKind;

/// Something that can be made, and what goes into it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Recipe {
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
    Recipe { makes: ItemKind::StoneKnife, needs: &[(ItemKind::Stone, 2)], tools: &[], at_fire: false },
    Recipe { makes: ItemKind::OpenedCoconut, needs: &[(ItemKind::Coconut, 1)], tools: &[ItemKind::StoneKnife], at_fire: false },
    Recipe { makes: ItemKind::StoneAxe, needs: &[(ItemKind::Stone, 1), (ItemKind::Rope, 1), (ItemKind::Branch, 1)], tools: &[], at_fire: false },
    Recipe { makes: ItemKind::WoodSpear, needs: &[(ItemKind::Branch, 1), (ItemKind::Stone, 1)], tools: &[], at_fire: false },
    Recipe { makes: ItemKind::ShellHook, needs: &[(ItemKind::SeaShell, 1)], tools: &[ItemKind::StoneKnife], at_fire: false },
    Recipe { makes: ItemKind::FishingRod, needs: &[(ItemKind::Branch, 1), (ItemKind::Rope, 1), (ItemKind::ShellHook, 1)], tools: &[], at_fire: false },
    Recipe { makes: ItemKind::GrilledCrab, needs: &[(ItemKind::Crab, 1)], tools: &[ItemKind::StoneKnife], at_fire: true },
    Recipe { makes: ItemKind::GrilledFish, needs: &[(ItemKind::Fish, 1)], tools: &[ItemKind::StoneKnife], at_fire: true },
    Recipe { makes: ItemKind::BarkContainer, needs: &[(ItemKind::Bark, 2), (ItemKind::Vine, 1)], tools: &[], at_fire: false },
    Recipe { makes: ItemKind::BoiledWater, needs: &[(ItemKind::BarkContainer, 1), (ItemKind::RawWater, 1)], tools: &[], at_fire: true },
    Recipe { makes: ItemKind::Torch, needs: &[(ItemKind::Branch, 1), (ItemKind::Husk, 1)], tools: &[], at_fire: true },
    Recipe { makes: ItemKind::Rope, needs: &[(ItemKind::Husk, 2)], tools: &[], at_fire: false },
    Recipe { makes: ItemKind::Thatch, needs: &[(ItemKind::PalmFrond, 4), (ItemKind::Vine, 1)], tools: &[], at_fire: false },
    Recipe { makes: ItemKind::Shelter, needs: &[(ItemKind::Branch, 4), (ItemKind::Rope, 2), (ItemKind::Thatch, 2)], tools: &[], at_fire: false },
    Recipe { makes: ItemKind::PackFrame, needs: &[(ItemKind::Branch, 4), (ItemKind::Rope, 2), (ItemKind::Bark, 2)], tools: &[], at_fire: false },
    Recipe { makes: ItemKind::FireDrill, needs: &[(ItemKind::Branch, 2), (ItemKind::Vine, 1)], tools: &[], at_fire: false },
    Recipe { makes: ItemKind::Ladder, needs: &[(ItemKind::LauanTimber, 2), (ItemKind::Branch, 3), (ItemKind::Rope, 2)], tools: &[ItemKind::StoneAxe], at_fire: false },
    Recipe { makes: ItemKind::LogBridge, needs: &[(ItemKind::PineTimber, 2), (ItemKind::Rope, 2)], tools: &[ItemKind::StoneAxe], at_fire: false },
];

/// The recipes the castaway has thought of. A recipe comes to mind the
/// first time one of each of its materials is carried at once (tools and
/// the fire aren't needed for that: finding out they're missing is a hint
/// in itself), and stays known. One not yet looked at is fresh.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(into = "KnownRecipes", from = "KnownRecipes")]
pub struct RecipeBook {
    known: Vec<bool>,
    fresh: Vec<bool>,
}

impl Default for RecipeBook {
    fn default() -> Self {
        Self { known: vec![false; RECIPES.len()], fresh: vec![false; RECIPES.len()] }
    }
}

/// How a recipe book is saved: the recipes known and those not yet looked
/// at, by what each makes (so adding recipes or reordering them leaves a
/// save as it was).
#[derive(serde::Serialize, serde::Deserialize)]
struct KnownRecipes {
    known: Vec<ItemKind>,
    fresh: Vec<ItemKind>,
}

impl From<RecipeBook> for KnownRecipes {
    fn from(b: RecipeBook) -> Self {
        let pick = |flags: &[bool]| RECIPES.iter().zip(flags).filter(|&(_, &on)| on).map(|(r, _)| r.makes).collect();
        Self { known: pick(&b.known), fresh: pick(&b.fresh) }
    }
}

impl From<KnownRecipes> for RecipeBook {
    fn from(k: KnownRecipes) -> Self {
        let flags = |kinds: &[ItemKind]| RECIPES.iter().map(|r| kinds.contains(&r.makes)).collect();
        Self { known: flags(&k.known), fresh: flags(&k.fresh) }
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

    /// Indices into `RECIPES` of the known recipes, in list order.
    pub fn listed(&self) -> Vec<usize> {
        (0..RECIPES.len()).filter(|&i| self.known[i]).collect()
    }

    /// Whether the recipe hasn't been looked at since it came to mind.
    pub fn is_fresh(&self, i: usize) -> bool {
        self.fresh[i]
    }

    pub fn looked_at(&mut self, i: usize) {
        self.fresh[i] = false;
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
    fn known_recipes_are_listed_in_recipe_order() {
        let mut book = RecipeBook::default();
        book.learn_all();
        assert_eq!(book.listed(), (0..RECIPES.len()).collect::<Vec<_>>());
        assert!((0..RECIPES.len()).all(|i| !book.is_fresh(i)));
    }

    #[test]
    fn a_saved_book_keeps_recipes_by_what_they_make() {
        let mut book = RecipeBook::default();
        book.notice(carrying(&[ItemKind::Stone]));
        book.looked_at(index(ItemKind::StoneKnife));
        let json: serde_json::Value = serde_json::to_value(&book).unwrap();
        assert!(json["known"].as_array().unwrap().contains(&serde_json::json!("StoneKnife")));
        assert!(!json["fresh"].as_array().unwrap().contains(&serde_json::json!("StoneKnife")));
        assert_eq!(serde_json::from_value::<RecipeBook>(json).unwrap(), book);
    }

    #[test]
    fn every_recipe_makes_something_different() {
        for (i, r) in RECIPES.iter().enumerate() {
            assert_eq!(index(r.makes), i, "{:?} is made by two recipes", r.makes);
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

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
    /// Only made beside a burning campfire.
    pub at_fire: bool,
}

pub const RECIPES: &[Recipe] = &[
    Recipe { makes: ItemKind::StoneAxe, needs: &[(ItemKind::Stone, 1), (ItemKind::Vine, 1), (ItemKind::Branch, 1)], at_fire: false },
    Recipe { makes: ItemKind::WoodSpear, needs: &[(ItemKind::Branch, 1), (ItemKind::Stone, 1)], at_fire: false },
    Recipe { makes: ItemKind::GrilledCrab, needs: &[(ItemKind::Crab, 1)], at_fire: true },
    Recipe { makes: ItemKind::BarkContainer, needs: &[(ItemKind::Bark, 2), (ItemKind::Vine, 1)], at_fire: false },
    Recipe { makes: ItemKind::BoiledWater, needs: &[(ItemKind::BarkContainer, 1), (ItemKind::RawWater, 1)], at_fire: true },
    Recipe { makes: ItemKind::FireDrill, needs: &[(ItemKind::Branch, 2), (ItemKind::Vine, 1)], at_fire: false },
];

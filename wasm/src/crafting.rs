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
    Recipe { makes: ItemKind::GrilledCrab, needs: &[(ItemKind::Crab, 1)], tools: &[ItemKind::StoneKnife], at_fire: true },
    Recipe { makes: ItemKind::BarkContainer, needs: &[(ItemKind::Bark, 2), (ItemKind::Vine, 1)], tools: &[], at_fire: false },
    Recipe { makes: ItemKind::BoiledWater, needs: &[(ItemKind::BarkContainer, 1), (ItemKind::RawWater, 1)], tools: &[], at_fire: true },
    Recipe { makes: ItemKind::Torch, needs: &[(ItemKind::Branch, 1), (ItemKind::Husk, 1)], tools: &[], at_fire: true },
    Recipe { makes: ItemKind::Rope, needs: &[(ItemKind::Husk, 2)], tools: &[], at_fire: false },
    Recipe { makes: ItemKind::Thatch, needs: &[(ItemKind::PalmFrond, 4), (ItemKind::Vine, 1)], tools: &[], at_fire: false },
    Recipe { makes: ItemKind::Shelter, needs: &[(ItemKind::Branch, 4), (ItemKind::Rope, 2), (ItemKind::Thatch, 2)], tools: &[], at_fire: false },
    Recipe { makes: ItemKind::PackFrame, needs: &[(ItemKind::Branch, 4), (ItemKind::Rope, 2), (ItemKind::Bark, 2)], tools: &[], at_fire: false },
    Recipe { makes: ItemKind::FireDrill, needs: &[(ItemKind::Branch, 2), (ItemKind::Vine, 1)], tools: &[], at_fire: false },
];

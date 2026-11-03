//! Recipes for making things out of gathered materials. Crafting happens in
//! the bag screen and uses up materials carried in the bag.

use crate::items::ItemKind;

/// Something that can be made, and what goes into it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Recipe {
    pub makes: ItemKind,
    /// Materials used up, with how many of each.
    pub needs: &'static [(ItemKind, u32)],
}

pub const RECIPES: &[Recipe] = &[
    Recipe { makes: ItemKind::StoneAxe, needs: &[(ItemKind::Stone, 1), (ItemKind::Vine, 1), (ItemKind::Branch, 1)] },
    Recipe { makes: ItemKind::FireDrill, needs: &[(ItemKind::Branch, 2), (ItemKind::Vine, 1)] },
];

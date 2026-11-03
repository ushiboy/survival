//! Moving actors.

pub mod body;
pub mod item;
pub mod crab;
pub mod gull;
pub mod particles;
pub mod player;

pub use crab::Crab;
pub use gull::Gull;
pub use item::{Item, ItemKind};
pub use particles::Particles;
pub use player::{Deed, Player};

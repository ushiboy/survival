//! Moving actors.

pub mod body;
pub mod campfire;
pub mod crab;
pub mod fish;
pub mod gull;
pub mod item;
pub mod particles;
pub mod player;
pub mod signal_fire;

pub use campfire::Campfire;
pub use crab::Crab;
pub use fish::Fish;
pub use gull::Gull;
pub use item::{Item, ItemKind};
pub use particles::Particles;
pub use player::{Deed, Player, Situation};
pub use signal_fire::SignalFire;

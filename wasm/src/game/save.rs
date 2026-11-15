//! The saved game: one save, kept by the page (in the browser's storage)
//! as JSON text.
//!
//! The island is made the same way every time from its plan and seed, so
//! only what has changed is kept: the time, the castaway and what they
//! carry, the cells whose features differ from the island as made, and the
//! things lying about or living on it. Passing effects (sparks, splashes,
//! gulls, a line in the water, a shaking tree) are not kept.
//!
//! Every save carries the `version` of its layout. Loading reads the
//! version first; an older save is brought up to date one version at a time
//! by `MIGRATIONS` before it is read. When what a save holds changes, raise
//! `SAVE_VERSION` and add the step from the version before (see
//! docs/save.md). The versions were started over at 1 before the game's
//! release.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::bag::Bag;
use super::state::{GameClock, Gear, Vitals};
use crate::assets::character::Outfit;
use crate::common::math::Dir;
use crate::crafting::RecipeBook;
use crate::entity::Campfire;
use crate::items::ItemKind;
use crate::world::Feature;
use crate::world::harvest::HarvestState;

/// The layout of saves this game writes. Version 1 is the layout as it
/// stood when the versions were started over, before the game's release
/// (the steps from earlier layouts went with them: no one held such a
/// save). From here on, a change to what a save holds raises the version.
pub const SAVE_VERSION: u32 = 1;

/// Steps bringing a save up a version: `MIGRATIONS[n]` turns a version
/// `n + 1` save into version `n + 2`, editing the JSON in place. There are
/// none yet: version 1 is the first.
const MIGRATIONS: &[fn(&mut Value)] = &[];

const _: () = assert!(MIGRATIONS.len() as u32 == SAVE_VERSION - 1, "one migration for each version before the current one");

/// Why a save can't be loaded.
#[derive(Debug, PartialEq, Eq)]
pub enum LoadError {
    /// Not a save this game can read (damaged, or not a save at all).
    Unreadable,
    /// Written by a newer version of the game.
    Newer(u32),
}

/// Everything a save holds (the current version's layout).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SaveData {
    pub version: u32,
    /// The island's seed (it is made again from this).
    pub seed: u32,
    pub clock: Clock,
    pub vitals: Vitals,
    pub castaway: Castaway,
    pub inventory: Carried,
    pub island: IslandState,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Clock {
    pub day: u32,
    pub minute: u32,
}

impl Clock {
    pub fn of(c: &GameClock) -> Self {
        Self { day: c.day, minute: c.minute }
    }

    pub fn to_clock(&self) -> GameClock {
        let mut c = GameClock::new(self.day, 0);
        c.advance_minutes(self.minute);
        c
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Castaway {
    /// The body's top-left (world px).
    pub at: (i32, i32),
    pub facing: Dir,
    /// Which of the outfit presets is worn.
    pub outfit: usize,
    /// Has come upon the mill site (which happens once).
    pub mill_noticed: bool,
}

impl Castaway {
    pub fn outfit_index(outfit: &Outfit) -> usize {
        Outfit::PRESETS.iter().position(|o| o == outfit).unwrap_or(0)
    }

    pub fn outfit(&self) -> Outfit {
        Outfit::PRESETS.get(self.outfit).copied().unwrap_or(Outfit::CASTAWAY)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Carried {
    pub bag: Bag,
    /// When the torch carried was lit (game minutes), if one is.
    pub torch: Option<u32>,
    pub recipes: RecipeBook,
    pub gear: Gear,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IslandState {
    /// Cells whose feature differs from the island as made.
    pub features: Vec<((i32, i32), Feature)>,
    /// Coconuts on each palm or cones on each pine (`fruit`), and bark on
    /// each broadleaf tree, by anchor.
    pub fruit: Vec<((i32, i32), u8)>,
    pub bark: Vec<((i32, i32), u8)>,
    /// Trees being worked and bushes picked.
    pub harvest: HarvestState,
    pub items: Vec<Lying>,
    pub fires: Vec<Campfire>,
    /// Crabs still about (their bodies' top-left, world px).
    pub crabs: Vec<(i32, i32)>,
    /// Fish in the water (world px), and when new ones come in for those
    /// caught.
    pub fish: Vec<(i32, i32)>,
    pub fish_due: Vec<u32>,
    /// The day shells last washed up, and the most the beach holds.
    pub shell_day: u32,
    pub shell_cap: usize,
    /// Since when each chunk has slept (`None`: awake), row by row.
    pub asleep_since: Vec<Option<u32>>,
}

/// Something lying on the ground.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Lying {
    pub kind: ItemKind,
    pub count: u32,
    pub at: (i32, i32),
}

impl SaveData {
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("a save always turns into JSON")
    }

    /// Reads a save, bringing an older one up to date first.
    pub fn from_json(text: &str) -> Result<Self, LoadError> {
        let mut value: Value = serde_json::from_str(text).map_err(|_| LoadError::Unreadable)?;
        let version = value.get("version").and_then(Value::as_u64).ok_or(LoadError::Unreadable)? as u32;
        if version > SAVE_VERSION {
            return Err(LoadError::Newer(version));
        }
        if version == 0 {
            return Err(LoadError::Unreadable);
        }
        for step in &MIGRATIONS[(version - 1) as usize..] {
            step(&mut value);
        }
        value["version"] = Value::from(SAVE_VERSION);
        serde_json::from_value(value).map_err(|_| LoadError::Unreadable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A save of each version, from the first to the current one (see
    /// docs/save.md: a sample of the new version is added whenever the
    /// version goes up).
    const SAMPLES: &[&str] = &[include_str!("save_samples/v1.json")];

    fn load(version: u32) -> SaveData {
        SaveData::from_json(SAMPLES[version as usize - 1]).expect("the sample loads")
    }

    #[test]
    fn there_is_a_sample_of_every_version() {
        assert_eq!(SAMPLES.len() as u32, SAVE_VERSION);
        for (i, text) in SAMPLES.iter().enumerate() {
            let v: Value = serde_json::from_str(text).unwrap();
            assert_eq!(v["version"], i + 1, "save_samples/v{}.json", i + 1);
        }
    }

    #[test]
    fn every_older_save_loads_as_the_current_version() {
        for version in 1..=SAVE_VERSION {
            assert_eq!(load(version).version, SAVE_VERSION, "a version {version} save");
        }
    }

    #[test]
    fn a_current_save_comes_back_the_same() {
        let text = SAMPLES[SAVE_VERSION as usize - 1];
        let again: Value = serde_json::from_str(&load(SAVE_VERSION).to_json()).unwrap();
        assert_eq!(again, serde_json::from_str::<Value>(text).unwrap());
    }

    #[test]
    fn a_save_from_a_newer_game_is_not_read() {
        let text = format!(r#"{{"version":{}}}"#, SAVE_VERSION + 1);
        assert_eq!(SaveData::from_json(&text).unwrap_err(), LoadError::Newer(SAVE_VERSION + 1));
    }

    #[test]
    fn what_is_not_a_save_is_unreadable() {
        let incomplete = format!(r#"{{"version":{SAVE_VERSION}}}"#);
        for text in ["", "not json", "{}", r#"{"version":0}"#, r#"{"version":"1"}"#, incomplete.as_str()] {
            assert_eq!(SaveData::from_json(text).unwrap_err(), LoadError::Unreadable, "{text:?}");
        }
    }

    #[test]
    fn the_sample_keeps_a_clay_bed_dug_today_and_what_was_dug() {
        let s = load(SAVE_VERSION);
        let dug = s.island.features.iter().filter(|(_, f)| matches!(f, Feature::ClayBed(c) if c.dug)).count();
        assert_eq!(dug, 4, "every cell of the bed the sample dug");
        assert!(s.inventory.bag.has(ItemKind::BambooSpade) && s.inventory.bag.count(ItemKind::Clay) == 2);
        assert!(s.castaway.mill_noticed, "the sample has come upon the mill site");
    }
}

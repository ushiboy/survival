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
//! docs/save.md).

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

/// The layout of saves this game writes.
pub const SAVE_VERSION: u32 = 2;

/// Steps bringing a save up a version: `MIGRATIONS[n]` turns a version
/// `n + 1` save into version `n + 2`, editing the JSON in place.
const MIGRATIONS: &[fn(&mut Value)] = &[v1_to_v2];

/// Version 2: the map grew 24 cells north (for the forest and the cliff
/// above it), so everything moves down; the top 16 rows of the old map
/// were redrawn (the jungle there became forest), so changes kept there are
/// dropped. Fruit and bark became changes from the island as made (they
/// were every tree's): palms picked bare were left out before, so they are
/// added back as bare.
fn v1_to_v2(save: &mut Value) {
    const ROWS: i64 = 24;
    const PX: i64 = ROWS * 16;
    const REDRAWN: i64 = 16;
    const OLD_SIZE: (i64, i64) = (96, 48);
    let shift = |v: &mut Value, by: i64| {
        if let Some(y) = v.get_mut(1).and_then(|y| y.as_i64().map(|n| (y, n))) {
            *y.0 = Value::from(y.1 + by);
        }
    };
    // `[[x, y], ...]` lists keyed by cell: moved down, dropped where redrawn.
    let cells = |list: Option<&mut Value>| {
        if let Some(Value::Array(entries)) = list {
            entries.retain(|e| e[0][1].as_i64().is_some_and(|y| y >= REDRAWN));
            for e in entries.iter_mut() {
                shift(&mut e[0], ROWS);
            }
        }
    };
    let island = &mut save["island"];
    for key in ["features", "fruit", "bark"] {
        cells(island.get_mut(key));
    }
    for key in ["damage", "shakes", "picked"] {
        cells(island["harvest"].get_mut(key));
    }
    // Palms picked bare (in the old map) are no longer left out.
    let seed = save["seed"].as_u64().unwrap_or(0) as u32;
    let made = crate::world::island::generate(seed);
    let island = &mut save["island"];
    if let Some(Value::Array(fruit)) = island.get_mut("fruit") {
        let kept: Vec<(i64, i64)> = fruit.iter().filter_map(|e| Some((e[0][0].as_i64()?, e[0][1].as_i64()?))).collect();
        for (x, y) in made.palm_anchors() {
            let (x, y) = (x as i64, y as i64);
            let old = (0..OLD_SIZE.0).contains(&x) && (ROWS + REDRAWN..ROWS + OLD_SIZE.1).contains(&y);
            if old && made.fruit((x as i32, y as i32)) > 0 && !kept.contains(&(x, y)) {
                fruit.push(serde_json::json!([[x, y], 0]));
            }
        }
    }
    // Things lying about and moving: their pixels move down.
    for key in ["crabs", "fish"] {
        if let Some(Value::Array(list)) = island.get_mut(key) {
            list.iter_mut().for_each(|p| shift(p, PX));
        }
    }
    if let Some(Value::Array(items)) = island.get_mut("items") {
        items.iter_mut().for_each(|i| shift(&mut i["at"], PX));
    }
    if let Some(Value::Array(fires)) = island.get_mut("fires") {
        for f in fires {
            if let Some(y) = f["y"].as_i64() {
                f["y"] = Value::from(y + PX);
            }
        }
    }
    // The chunks are laid out afresh.
    island["asleep_since"] = Value::Array(Vec::new());
    shift(&mut save["castaway"]["at"], PX);
}

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
    /// Coconuts on each palm and bark on each broadleaf tree, by anchor.
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

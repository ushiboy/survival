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
pub const SAVE_VERSION: u32 = 5;

/// Steps bringing a save up a version: `MIGRATIONS[n]` turns a version
/// `n + 1` save into version `n + 2`, editing the JSON in place.
const MIGRATIONS: &[fn(&mut Value)] = &[v1_to_v2, v2_to_v3, v3_to_v4, v4_to_v5];

/// Version 2: the map grew 24 cells north (for the forest and the cliff
/// above it), so everything moves down; the top 16 rows of the old map
/// were redrawn (the jungle there became forest), so changes kept there are
/// dropped. Fruit and bark became changes from the island as made (they
/// were every tree's): palms picked bare were left out before, so they are
/// added back as bare.
/// Version 3 added the ladder: an item, and a feature on the cells it's
/// stood against (a version 2 save has neither). It also redrew the north
/// cliff around its low stretch (a wider recess, the edge straight either
/// side of it, and a clearing in the forest in front): what changed there
/// in a version 2 save no longer fits the island, so it is dropped.
fn v2_to_v3(save: &mut Value) {
    drop_changes_in(save, (54..74, 6..26), |_| false);
}

/// Version 4: the low stretch of the north cliff became a dip in its top
/// (its edge a row lower, its foot in line with the rest of the cliff),
/// and the clearing in front of it shrank, so the trees around it were
/// placed again: what changed there in a version 3 save is dropped. A
/// ladder stood there moves down the row with the foot, as does anything
/// lying at the old foot and the castaway standing there.
fn v3_to_v4(save: &mut Value) {
    const DIP: (std::ops::Range<i64>, std::ops::Range<i64>) = (62..65, 12..16);
    const ROW: i64 = 16;
    let ladder = |e: &Value| e[1].get("Ladder").is_some();
    if let Some(Value::Array(features)) = save["island"].get_mut("features") {
        for e in features.iter_mut().filter(|e| ladder(e)) {
            if let Some(y) = e[0][1].as_i64() {
                e[0][1] = Value::from(y + 1);
            }
        }
    }
    drop_changes_in(save, (54..80, 10..40), ladder);
    // From world px to a cell in the dip's old rows, moved down a row.
    let lower = |at: &mut Value, half: i64| {
        if let (Some(x), Some(y)) = (at[0].as_i64(), at[1].as_i64())
            && DIP.0.contains(&((x + half) / ROW))
            && DIP.1.contains(&((y + half) / ROW))
        {
            at[1] = Value::from(y + ROW);
        }
    };
    if let Some(Value::Array(items)) = save["island"].get_mut("items") {
        for item in items {
            lower(&mut item["at"], 0);
        }
    }
    lower(&mut save["castaway"]["at"], ROW / 2);
}

/// Version 5: most of the trees on the high ground became pines (which
/// can be shaken and felled), so the trees there were placed again: what
/// changed on the high ground in a version 4 save is dropped. The high
/// ground is read from the island made from the save's seed.
fn v4_to_v5(save: &mut Value) {
    let seed = save["seed"].as_u64().unwrap_or(1) as u32;
    let island = crate::world::island::generate(seed);
    let high = |e: &Value| match (e[0][0].as_i64(), e[0][1].as_i64()) {
        (Some(x), Some(y)) => island.terrain(x as i32, y as i32) == crate::world::Terrain::Highland,
        _ => false,
    };
    drop_changes_where(save, high, |_| false);
}

/// Drops the changes to cells in `region` (x, y) kept in a save (features
/// but those `spare` keeps, fruit, bark, and trees being worked), where
/// the island was redrawn so they no longer fit it.
fn drop_changes_in(save: &mut Value, region: (std::ops::Range<i64>, std::ops::Range<i64>), spare: impl Fn(&Value) -> bool) {
    let inside = |e: &Value| match (e[0][0].as_i64(), e[0][1].as_i64()) {
        (Some(x), Some(y)) => region.0.contains(&x) && region.1.contains(&y),
        _ => false,
    };
    drop_changes_where(save, inside, spare);
}

/// Drops the changes kept in a save to the cells `redrawn` picks out (each
/// entry keyed by its cell): features but those `spare` keeps, fruit,
/// bark, and trees being worked.
fn drop_changes_where(save: &mut Value, redrawn: impl Fn(&Value) -> bool, spare: impl Fn(&Value) -> bool) {
    let outside = |e: &Value| !redrawn(e);
    let island = &mut save["island"];
    for key in ["features", "fruit", "bark"] {
        if let Some(Value::Array(entries)) = island.get_mut(key) {
            entries.retain(|e| outside(e) || (key == "features" && spare(e)));
        }
    }
    for key in ["damage", "shakes", "picked"] {
        if let Some(Value::Array(entries)) = island["harvest"].get_mut(key) {
            entries.retain(|e| outside(e));
        }
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::terrain::StumpPart;

    /// A save of each version, from the first to the current one (see
    /// docs/save.md: a sample of the new version is added whenever the
    /// version goes up).
    const SAMPLES: &[&str] = &[
        include_str!("save_samples/v1.json"),
        include_str!("save_samples/v2.json"),
        include_str!("save_samples/v3.json"),
        include_str!("save_samples/v4.json"),
        include_str!("save_samples/v5.json"),
    ];

    fn load(version: u32) -> SaveData {
        SaveData::from_json(SAMPLES[version as usize - 1]).expect("the sample loads")
    }

    fn has_feature(s: &SaveData, at: (i32, i32), f: Feature) -> bool {
        s.island.features.contains(&(at, f))
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
        for text in ["", "not json", "{}", r#"{"version":0}"#, r#"{"version":"4"}"#, r#"{"version":4}"#] {
            assert_eq!(SaveData::from_json(text).unwrap_err(), LoadError::Unreadable, "{text:?}");
        }
    }

    #[test]
    fn version_2_moves_everything_down_with_the_map() {
        let s = load(1);
        assert_eq!(s.castaway.at, (512, 296 + 384));
        assert_eq!(s.island.items[0].at, (500, 300 + 384));
        assert_eq!((s.island.fires[0].x, s.island.fires[0].y), (100, 200 + 384));
        assert_eq!(s.island.crabs, vec![(300, 400 + 384)]);
        assert_eq!(s.island.fish, vec![(600, 700 + 384)]);
        assert!(has_feature(&s, (20, 30 + 24), Feature::VineStake));
        assert_eq!(s.island.bark, vec![((30, 40 + 24), 2)]);
        assert_eq!(s.island.harvest.damage, vec![((40, 20 + 24), 1)]);
        assert_eq!(s.island.harvest.picked, vec![((44, 30 + 24), 2)]);
    }

    #[test]
    fn version_2_drops_changes_in_the_redrawn_top_rows() {
        let s = load(1);
        assert!(!s.island.features.iter().any(|&(at, _)| at == (10, 5) || at == (10, 5 + 24)));
        assert!(!s.island.bark.iter().any(|&((_, y), _)| y == 10 || y == 10 + 24));
        assert!(!s.island.harvest.damage.iter().any(|&((x, _), _)| x == 10));
    }

    #[test]
    fn version_2_starts_every_chunk_awake() {
        assert!(load(1).island.asleep_since.is_empty());
    }

    #[test]
    fn version_2_keeps_palms_picked_bare_as_bare() {
        let s = load(1);
        assert!(!s.island.fruit.is_empty());
        assert!(s.island.fruit.iter().all(|&(_, n)| n == 0));
    }

    #[test]
    fn version_3_drops_changes_by_the_redrawn_cliff() {
        let s = load(2);
        assert!(!has_feature(&s, (60, 20), Feature::Rock));
        assert!(has_feature(&s, (20, 50), Feature::VineStake));
        assert!(s.island.bark.is_empty());
        assert_eq!(s.island.harvest.damage, vec![((20, 50), 1)]);
    }

    #[test]
    fn version_4_moves_a_ladder_down_with_the_cliff_foot() {
        let s = load(3);
        assert!(has_feature(&s, (63, 13), Feature::Ladder(0)));
        assert!(has_feature(&s, (63, 14), Feature::Ladder(1)));
        assert!(!has_feature(&s, (63, 12), Feature::Ladder(0)));
    }

    #[test]
    fn version_4_drops_other_changes_by_the_redrawn_cliff() {
        let s = load(3);
        assert!(!has_feature(&s, (70, 30), Feature::Rock));
        assert!(has_feature(&s, (20, 50), Feature::VineStake));
        assert!(s.island.harvest.damage.is_empty());
    }

    #[test]
    fn version_5_drops_changes_on_the_high_ground_where_pines_now_grow() {
        let s = load(4);
        assert!(!has_feature(&s, (40, 5), Feature::BroadleafStump(StumpPart { flip: false })));
        assert!(s.island.bark.is_empty());
        assert!(has_feature(&s, (40, 40), Feature::Rock), "changes below the cliff stay");
        assert_eq!(s.island.harvest.damage, vec![((40, 40), 1)]);
    }

    #[test]
    fn version_4_moves_what_stood_at_the_old_foot_down_a_row() {
        let s = load(3);
        assert_eq!(s.castaway.at, (1000, 200 + 16));
        let at = |kind| s.island.items.iter().find(|l| l.kind == kind).unwrap().at;
        assert_eq!(at(ItemKind::Rope), (1008, 208 + 16));
        assert_eq!(at(ItemKind::Stone), (400, 600));
    }
}

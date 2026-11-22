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
use super::cart::Cart;
use super::mill::{Furnishing, MILL_STAGES, MillWorks};
use super::shelf::Shelf;
use super::state::{GameClock, Gear, Vitals};
use crate::assets::character::Outfit;
use crate::common::math::Dir;
use crate::crafting::{RECIPES, RecipeBook};
use crate::entity::{Campfire, SignalFire};
use crate::items::ItemKind;
use crate::world::Feature;
use crate::world::harvest::HarvestState;

/// The layout of saves this game writes. Version 1 is the layout as it
/// stood when the versions were started over, before the game's release
/// (the steps from earlier layouts went with them: no one held such a
/// save). From here on, a change to what a save holds raises the version.
pub const SAVE_VERSION: u32 = 8;

/// Steps bringing a save up a version: `MIGRATIONS[n]` turns a version
/// `n + 1` save into version `n + 2`, editing the JSON in place.
const MIGRATIONS: &[fn(&mut Value)] = &[v1_to_v2, v2_to_v3, v3_to_v4, v4_to_v5, v5_to_v6, v6_to_v7, v7_to_v8];

/// Version 2 added the waist to the gear (the tool belt) and the tools on
/// its hooks, and the handcart (on the island) and its load (carried): a
/// version 1 save wears nothing there and has no cart made.
fn v1_to_v2(save: &mut Value) {
    if let Some(gear) = save.pointer_mut("/inventory/gear").and_then(Value::as_object_mut) {
        gear.insert("waist".into(), Value::Null);
        gear.insert("belt".into(), serde_json::json!([null, null]));
    }
    if let Some(inventory) = save.get_mut("inventory").and_then(Value::as_object_mut) {
        inventory.insert("cart".into(), Value::Null);
    }
    if let Some(island) = save.get_mut("island").and_then(Value::as_object_mut) {
        island.insert("cart".into(), Value::Null);
    }
}

/// Version 3 added signal fires (on the island, with the day the last was
/// lit and when the castaway is to be rescued) and the recipes that have
/// made something (for the kinds of things made the end shows): a version
/// 2 save has no signal fire lit or set, and nothing counted as made.
fn v2_to_v3(save: &mut Value) {
    if let Some(island) = save.get_mut("island").and_then(Value::as_object_mut) {
        island.insert("signal_fires".into(), serde_json::json!([]));
        island.insert("signal_day".into(), Value::Null);
        island.insert("rescue_at".into(), Value::Null);
    }
    if let Some(recipes) = save.pointer_mut("/inventory/recipes").and_then(Value::as_object_mut) {
        recipes.insert("made".into(), serde_json::json!([]));
    }
}

/// Version 4 added the order the CRAFT list is sorted in: a version 3 save
/// lists the recipes A to Z.
fn v3_to_v4(save: &mut Value) {
    if let Some(recipes) = save.pointer_mut("/inventory/recipes").and_then(Value::as_object_mut) {
        recipes.insert("sort".into(), Value::from("AToZ"));
    }
}

/// Version 5 added how far the water mill has got (on the island): a
/// version 4 save has built none of it and left nothing at its site.
fn v4_to_v5(save: &mut Value) {
    if let Some(island) = save.get_mut("island").and_then(Value::as_object_mut) {
        island.insert("mill".into(), serde_json::json!({ "built": 0, "delivered": [] }));
    }
}

/// Version 6 added every kind of thing that has been in the bag (recipes
/// come to mind from it, not from what's carried at once). A version 5
/// save doesn't know it, so it's taken to have had what it carries (in
/// the bag, worn, on the belt, in the cart), the materials of the recipes
/// it has thought of, what its recipes have made, and what has gone into
/// the water mill.
fn v5_to_v6(save: &mut Value) {
    let had = had_by_a_version_5_save(save);
    if let Some(inventory) = save.get_mut("inventory").and_then(Value::as_object_mut) {
        inventory.insert("had".into(), serde_json::to_value(had).unwrap_or_else(|_| serde_json::json!([])));
    }
}

/// Version 7 keeps how many times each recipe has made something, not
/// only whether it has (some things may be made only so many times). A
/// version 6 save is taken to have made once with each recipe it has made
/// with, but to have made the wheels its handcart took if it has one.
fn v6_to_v7(save: &mut Value) {
    let has_cart = save.pointer("/inventory/cart").is_some_and(|c| !c.is_null());
    let Some(made) = save.pointer_mut("/inventory/recipes/made") else { return };
    let names: Vec<String> = serde_json::from_value(made.clone()).unwrap_or_default();
    let mut times: serde_json::Map<String, Value> = names.into_iter().map(|n| (n, Value::from(1))).collect();
    if has_cart || times.contains_key("Handcart") {
        let cart = RECIPES.iter().find(|r| r.makes == ItemKind::Handcart);
        let wheels = cart.and_then(|c| c.needs.iter().find(|&&(k, _)| k == ItemKind::SpokedWheel)).map_or(0, |&(_, n)| n);
        if let Some(way) = RECIPES.iter().find(|r| r.makes == ItemKind::SpokedWheel) {
            times.insert(way.name.into(), Value::from(wheels));
        }
    }
    *made = Value::Object(times);
}

/// Version 8 added what the water mill has been fitted out with at its
/// workbench (on the island), and what has been left on its shelf
/// (carried along with the castaway's things, as it goes into what's made
/// in the mill): a version 7 save has fitted out nothing and left nothing
/// on a shelf.
fn v7_to_v8(save: &mut Value) {
    if let Some(mill) = save.pointer_mut("/island/mill").and_then(Value::as_object_mut) {
        mill.insert("furnished".into(), serde_json::json!([]));
    }
    if let Some(inventory) = save.get_mut("inventory").and_then(Value::as_object_mut) {
        inventory.insert("shelf".into(), serde_json::json!({ "stock": [] }));
        inventory.insert("looked_at".into(), serde_json::json!([]));
    }
}

fn had_by_a_version_5_save(save: &Value) -> Vec<ItemKind> {
    fn read<T: serde::de::DeserializeOwned>(save: &Value, at: &str) -> Option<T> {
        save.pointer(at).and_then(|v| serde_json::from_value(v.clone()).ok())
    }
    let mut kinds: Vec<ItemKind> = Vec::new();
    let bags: [Option<Bag>; 2] = [read(save, "/inventory/bag"), read(save, "/inventory/cart")];
    kinds.extend(bags.iter().flatten().flat_map(|b| b.items().iter().map(|p| p.kind)));
    if let Some(gear) = read::<Gear>(save, "/inventory/gear") {
        kinds.extend([gear.hand, gear.back, gear.waist].into_iter().chain(gear.belt).flatten());
    }
    let names = |field: &str| read::<Vec<String>>(save, &format!("/inventory/recipes/{field}")).unwrap_or_default();
    let named = |n: &String| RECIPES.iter().find(|r| r.name == n);
    kinds.extend(names("known").iter().filter_map(named).flat_map(|r| r.needs.iter().map(|&(k, _)| k)));
    kinds.extend(names("made").iter().filter_map(named).map(|r| r.makes));
    // The mill as a version 5 save kept it (before it was fitted out).
    #[derive(serde::Deserialize)]
    struct Mill {
        built: usize,
        delivered: Vec<u32>,
    }
    if let Some(mill) = read::<Mill>(save, "/island/mill") {
        let built = MILL_STAGES.iter().take(mill.built).flat_map(|s| s.needs.iter().map(|&(k, _)| k));
        let left = MILL_STAGES.get(mill.built).into_iter().flat_map(|s| s.needs.iter().zip(&mill.delivered).filter(|&(_, &n)| n > 0).map(|(&(k, _), _)| k));
        kinds.extend(built.chain(left));
    }
    let mut had = Vec::new();
    for k in kinds {
        if !had.contains(&k) {
            had.push(k);
        }
    }
    had
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
    /// Has come upon the mill site (which happens once).
    pub mill_noticed: bool,
    /// Has come upon the river's narrow (which happens once).
    pub bridge_noticed: bool,
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
    /// The handcart's load, once one has been made.
    pub cart: Option<Bag>,
    /// Every kind of thing that has been in the bag.
    pub had: Vec<ItemKind>,
    /// What has been left on the shelf in the mill.
    pub shelf: Shelf,
    /// The furnishings opened at the mill's workbench.
    pub looked_at: Vec<Furnishing>,
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
    /// Signal fires set on the island (their cells are also among
    /// `features`), and when each was lit while it burns.
    pub signal_fires: Vec<SignalFire>,
    /// The day a signal fire was last lit (the day's chance of its smoke
    /// being seen was had), and when the castaway is to be rescued, if
    /// the smoke is to be seen.
    pub signal_day: Option<u32>,
    pub rescue_at: Option<u32>,
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
    /// The handcart, once made: where it stands, whether it's being
    /// pulled, and which way its poles point.
    pub cart: Option<Cart>,
    /// How far the water mill has got: the stages built and what has been
    /// left at its site for the next.
    pub mill: MillWorks,
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
    use crate::crafting::RecipeOrder;

    /// A save of each version, from the first to the current one (see
    /// docs/save.md: a sample of the new version is added whenever the
    /// version goes up).
    const SAMPLES: &[&str] = &[
        include_str!("save_samples/v1.json"),
        include_str!("save_samples/v2.json"),
        include_str!("save_samples/v3.json"),
        include_str!("save_samples/v4.json"),
        include_str!("save_samples/v5.json"),
        include_str!("save_samples/v6.json"),
        include_str!("save_samples/v7.json"),
        include_str!("save_samples/v8.json"),
    ];

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
    fn a_version_1_save_wears_nothing_at_the_waist() {
        let s = load(1);
        assert_eq!(s.gear_waist(), (None, [None, None]));
    }

    #[test]
    fn a_version_2_save_has_no_signal_fire_and_nothing_made_but_its_handcarts_wheels() {
        let s = load(2);
        assert!(s.island.signal_fires.is_empty());
        assert_eq!((s.island.signal_day, s.island.rescue_at), (None, None));
        assert!(s.inventory.cart.is_some(), "the sample has a handcart");
        assert!(s.inventory.recipes.times_made(ItemKind::SpokedWheel) > 0, "its wheels were made (version 7)");
        assert_eq!(s.inventory.recipes.kinds_made(), 1, "nothing else counted as made (version 3)");
    }

    #[test]
    fn the_sample_keeps_a_burning_signal_fire_and_what_was_made() {
        let s = load(SAVE_VERSION);
        let fire = s.island.signal_fires.first().expect("the sample has a signal fire");
        assert!(fire.lit_at.is_some(), "burning");
        assert!(s.island.features.iter().any(|&(cell, f)| cell == fire.cell && f == Feature::SignalFire(true)));
        assert!(s.island.signal_day.is_some(), "the day's chance was had");
        assert!(s.inventory.recipes.kinds_made() > 0);
    }

    #[test]
    fn a_version_3_save_lists_recipes_a_to_z() {
        assert_eq!(load(3).inventory.recipes.order(), RecipeOrder::AToZ);
    }

    #[test]
    fn the_sample_keeps_the_order_the_recipes_were_sorted_in() {
        assert_eq!(load(SAVE_VERSION).inventory.recipes.order(), RecipeOrder::CanMake);
    }

    #[test]
    fn a_version_4_save_has_built_none_of_the_mill() {
        let s = load(4);
        assert_eq!(s.island.mill, MillWorks::default());
    }

    #[test]
    fn the_sample_keeps_what_was_left_at_the_mills_site() {
        let s = load(SAVE_VERSION);
        assert_eq!(s.island.mill.built, 0);
        assert_eq!(s.island.mill.delivered(ItemKind::Stone), 6);
        assert_eq!(s.island.mill.delivered(ItemKind::Soil), 2);
    }

    #[test]
    fn a_version_5_save_has_had_what_it_carries_what_its_recipes_take_and_what_it_left_at_the_mill() {
        let had = load(5).inventory.had;
        let has = |kind| had.contains(&kind);
        assert!(has(ItemKind::Husk) && has(ItemKind::Clay), "in the bag");
        assert!(has(ItemKind::ToolBelt) && has(ItemKind::StoneAxe), "worn and hung on the belt");
        assert!(has(ItemKind::LauanTimber), "in the cart");
        assert!(has(ItemKind::Branch) && has(ItemKind::Bamboo), "materials of recipes thought of");
        assert!(has(ItemKind::Soil), "left at the mill's site");
        assert!(!has(ItemKind::Crab), "nothing the save gives no sign of");
        let mut once = had.clone();
        once.dedup();
        assert_eq!(once.len(), had.len(), "each kind once");
    }

    #[test]
    fn the_sample_keeps_what_has_been_had() {
        let had = load(SAVE_VERSION).inventory.had;
        assert!(had.contains(&ItemKind::Crab), "had before, though no longer carried");
    }

    #[test]
    fn a_version_6_save_has_made_once_what_it_had_made() {
        let s = load(6);
        assert_eq!(s.inventory.recipes.times_made(ItemKind::StoneKnife), 1);
        assert_eq!(s.inventory.recipes.times_made(ItemKind::StoneAxe), 0);
    }

    #[test]
    fn a_version_6_save_with_a_handcart_has_made_its_wheels() {
        let s = load(6);
        assert!(s.inventory.cart.is_some(), "the sample has a handcart");
        let cart = RECIPES.iter().find(|r| r.makes == ItemKind::Handcart).unwrap();
        let wheels = cart.needs.iter().find(|&&(k, _)| k == ItemKind::SpokedWheel).unwrap().1;
        assert_eq!(s.inventory.recipes.times_made(ItemKind::SpokedWheel), wheels);
    }

    #[test]
    fn the_sample_keeps_how_many_times_each_thing_was_made() {
        assert_eq!(load(SAVE_VERSION).inventory.recipes.times_made(ItemKind::StoneKnife), 3);
    }

    #[test]
    fn a_version_7_save_has_fitted_out_nothing_and_left_nothing_on_a_shelf() {
        let s = load(7);
        assert!(s.island.mill.furnished.is_empty());
        assert!(s.inventory.shelf.lines().is_empty());
        assert!(s.inventory.looked_at.is_empty());
    }

    #[test]
    fn the_sample_keeps_what_the_mill_is_fitted_out_with_and_whats_on_its_shelf() {
        let s = load(SAVE_VERSION);
        assert!(s.island.mill.has(Furnishing::Shelf));
        assert_eq!(s.inventory.shelf.count(ItemKind::Stone), 12);
        assert_eq!(s.inventory.shelf.count(ItemKind::Bamboo), 7);
        assert_eq!(s.inventory.looked_at, vec![Furnishing::Shelf]);
    }

    #[test]
    fn a_version_1_save_has_no_handcart() {
        let s = load(1);
        assert!(s.inventory.cart.is_none() && s.island.cart.is_none());
    }

    #[test]
    fn the_sample_keeps_the_handcart_where_it_stands_and_its_load() {
        let s = load(SAVE_VERSION);
        let cart = s.island.cart.expect("the sample has a cart");
        assert!(!cart.pulled, "set down");
        assert_eq!(s.inventory.cart.expect("its load").count(ItemKind::LauanTimber), 3);
    }

    #[test]
    fn the_sample_keeps_the_belt_and_the_tools_on_it() {
        let s = load(SAVE_VERSION);
        assert_eq!(s.gear_waist(), (Some(ItemKind::ToolBelt), [Some(ItemKind::StoneAxe), None]));
        assert!(!s.inventory.bag.has(ItemKind::StoneAxe), "a tool on the belt is out of the bag");
    }

    impl SaveData {
        fn gear_waist(&self) -> (Option<ItemKind>, [Option<ItemKind>; 2]) {
            (self.inventory.gear.waist, self.inventory.gear.belt)
        }
    }

    #[test]
    fn the_sample_keeps_a_clay_bed_dug_today_and_what_was_dug() {
        let s = load(SAVE_VERSION);
        let dug = s.island.features.iter().filter(|(_, f)| matches!(f, Feature::ClayBed(c) if c.dug)).count();
        assert_eq!(dug, 4, "every cell of the bed the sample dug");
        assert!(s.inventory.bag.has(ItemKind::BambooSpade) && s.inventory.bag.count(ItemKind::Clay) == 2);
        assert!(s.castaway.mill_noticed, "the sample has come upon the mill site");
        assert!(s.castaway.bridge_noticed, "and upon the river's narrow");
    }
}

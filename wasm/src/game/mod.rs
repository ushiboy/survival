//! Top-level orchestration: owns every subsystem and runs one frame at a time.
//! `step` advances simulation; `render` produces the picture. They are kept
//! separate so the host can run fixed-rate updates independent of display.

pub mod autosave;
pub mod bag;
pub mod cart;
pub mod ending;
pub mod fishing;
pub mod menu;
pub mod mill;
pub mod regions;
pub mod save;
pub mod shelf;
pub mod state;

use crate::assets::character::Outfit;
use crate::assets::palettes::{OBJ_CRAB, OBJ_CRAB_NIGHT, OBJ_FX, OBJ_WATER_FX, base_cgram, set_health_palette, set_player_palettes};
use crate::assets::props::{PALM_W, coconut_spots};
use crate::assets::{Assets, SpriteIds, ending_art};
use crate::common::input::{Buttons, InputState};
use crate::common::math::{Dir, Rect};
use crate::common::rng::Rng;
use crate::crafting::{RECIPES, Recipe};
use crate::entity::fish;
use crate::entity::player::ACTOR_PRIORITY;
use crate::entity::{Campfire, Crab, Deed, Fish, Gull, Item, Particles, Player, SignalFire, Situation, signal_fire};
use crate::items::{ItemKind, Use};
use crate::ppu::effects::{ColorMath, MathOp, MathSource, WindowRegion, layer};
use crate::ppu::oam::ObjEntry;
use crate::ppu::palette::Cgram;
use crate::ppu::{BgLayer, FrameState, Ppu, SCREEN_H, SCREEN_W};
use crate::render::bag_screen;
use crate::render::camera::Camera;
use crate::render::end_screen;
use crate::render::hud::{self, HUD_LINES};
use crate::render::light_canvas;
use crate::render::lighting::{self, Lighting};
use crate::render::scene::{NoLayer, Scene, SpriteSet};
use crate::render::sprites::DrawList;
use crate::render::tileset::{Tileset, VisualMap, sway_palm};
use crate::render::title_screen::{self, TitleItem};
use crate::world::driftwood::WestCoast;
use crate::world::harvest::{self, Harvest, Shaken, Strike};
use crate::world::map::BridgeSpan;
use crate::world::{ChunkId, Feature, TILE_SIZE, Terrain, WorldMap, driftwood, island};
use autosave::{AutoSave, Cue};
use bag::Bag;
use cart::{CART_REACH, CART_ROWS, CART_W, Cart};
pub use ending::EndingView;
use fishing::{Float, Line};
use menu::{BagMenu, Outcome};
use mill::{Furnishing, MillWorks};
use regions::Regions;
use save::{SAVE_VERSION, SaveData};
use state::{Ailment, GameClock, Inventory, Rest, Vitals};

/// Something the dev tools can hand over the makings of: a recipe (by its
/// place in `RECIPES`) or a furnishing made at the mill's workbench.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Making {
    Recipe(usize),
    Furnishing(Furnishing),
}

/// Dev tools: every recipe, then every furnishing, as
/// `Game::debug_give_makings` numbers them.
pub fn debug_makings() -> Vec<Making> {
    (0..RECIPES.len()).map(Making::Recipe).chain(Furnishing::ALL.map(Making::Furnishing)).collect()
}

const START_HOUR: u32 = 7;
/// Above this darkness the sun casts no shadows and night darkness begins.
const NIGHT_THRESHOLD: f32 = 0.35;
/// Frames the action freezes when a blow lands (hit-stop).
const HITSTOP_TREE: u32 = 4;
const HITSTOP_FELL: u32 = 7;
const HITSTOP_HARD: u32 = 3;
const HITSTOP_CRAB: u32 = 5;
/// In-game minutes that pass each frame while asleep (an hour in half a
/// second).
const SLEEP_MINUTES_PER_FRAME: u32 = 2;
/// Frames each picture of the sign that the game was saved shows; the sign
/// plays through its pictures once (a pen writing, a page turned).
const SAVED_SIGN_HOLD: u32 = 6;
const SAVED_SIGN_FRAMES: u32 = SAVED_SIGN_HOLD * crate::assets::hud_art::SAVED_FRAMES as u32;
/// Frames the screen takes to fade out once the castaway collapses, and
/// before the end can be dismissed.
const COLLAPSE_FADE: u32 = 120;
const COLLAPSE_HOLD: u32 = 60;
/// Chance a shake brings a frond down, and fronds a felled palm sheds.
const FROND_CHANCE: f32 = 0.15;
const FELLED_FRONDS: u32 = 3;
/// The chance, once a day, that a signal fire's smoke is seen by a small
/// plane passing the island.
const RESCUE_CHANCE: f32 = 1.0 / 128.0;
/// How many times likelier black smoke (fish oil poured on the fire) is
/// to be seen than pale: dark against the sky, it shows from far off.
const BLACK_SMOKE_ODDS: f32 = 2.0;

/// The chance a day's signal fire smoke is seen, black or pale.
fn rescue_chance(black: bool) -> f32 {
    if black { RESCUE_CHANCE * BLACK_SMOKE_ODDS } else { RESCUE_CHANCE }
}
/// How long after a signal fire is lit (in-game minutes) its smoke may be
/// seen: some time in its day of burning, not the moment it's lit.
const SEEN_FROM: u32 = 60;
const SEEN_UNTIL: u32 = signal_fire::BURN_MINUTES - 60;
/// How near the mill site (cells) the castaway comes upon it.
const MILL_NOTICE_CELLS: i32 = 4;
/// How close (cells) to the bank at the river's narrow coming near it
/// first sets the castaway walking over there.
const BRIDGE_NOTICE_CELLS: i32 = 4;
/// Shells the tide leaves on the beach each morning, at most (for each
/// stretch of island the size of the first area; see `island::for_area`).
const SHELLS_PER_DAY: usize = 3;
/// Fish in the water (for each stretch the size of the first area), and
/// how long (game minutes) before another comes in for one caught.
const FISH_COUNT: usize = 12;
/// Fish in the lake on the high ground, besides those in the sea and the
/// river (which `FISH_COUNT` counts).
pub(crate) const LAKE_FISH: usize = 12;
const FISH_RETURN_MINUTES: u32 = 3 * 60;
/// How near a fish must be to the water ahead for the castaway to wonder
/// about catching it (px).
const FISH_NEAR: i32 = 40;

/// Standing within this distance of a campfire counts as being by it (px).
const NEAR_FIRE: i32 = 40;
/// How far from the middle of the wood lathe (px) things can be turned on
/// it, as far as from a fire.
const NEAR_LATHE: i32 = 40;
/// No two fires closer than this (px).
const FIRE_SPACING: i32 = 24;
/// Frames between embers rising from a fire (on average).
const EMBER_EVERY: u32 = 14;
/// How close to the spear's line a crab must be to be caught (px).
const CRAB_HIT_RADIUS: f32 = 9.0;
/// Frames a struck canopy wobbles, and the wobble offsets (BG2 line scroll).
const TREE_SHAKE_FRAMES: u32 = 16;
const TREE_SHAKE: [i32; 8] = [0, 1, -1, 1, -2, 2, -2, 2];
const CAMERA_SHAKE_FRAMES: u32 = 10;
/// A shaken palm sways for less time than a struck one.
const TREE_SWAY_FRAMES: u32 = 10;
/// Four 4bpp 8x8 tiles.
const METATILE_BYTES: usize = 128;

/// Day: sprites in palettes 4-7 (shadows, foam, submerged legs) are averaged
/// with the background on the sub-screen, i.e. drawn at 50% opacity.
const MATH_TRANSLUCENT: ColorMath = ColorMath {
    layers: layer::OBJ,
    op: MathOp::Add,
    half: true,
    source: MathSource::SubScreen,
    region: WindowRegion::Everywhere,
    sub_layers: layer::BG1 | layer::BG2,
};

/// Night: subtract the sub-screen (BG3 light mask, else the fixed colour)
/// from the scene. The HUD and sprites in palettes 0-3 (the castaway) are
/// exempt, as on hardware, so the player always sees themselves and their
/// status.
const MATH_NIGHT: ColorMath = ColorMath {
    layers: layer::BG1 | layer::BG2 | layer::OBJ | layer::BACKDROP,
    op: MathOp::Sub,
    half: false,
    source: MathSource::SubScreen,
    region: WindowRegion::Everywhere,
    sub_layers: layer::BG3,
};

/// Going through the mill's door, in or out: the screen fades out, the
/// castaway is set down on the other side (`to`, their feet, facing
/// `facing`), and it fades back in (`PASSAGE_FADE` frames each way).
#[derive(Clone, Copy, Debug)]
struct Passage {
    t: u32,
    to: (i32, i32),
    facing: Dir,
}

/// Frames the screen takes to fade out, or back in, going through the
/// mill's door.
const PASSAGE_FADE: u32 = 10;

/// A palm canopy wobbling after a hit: world-space rows and frames left.
#[derive(Clone, Copy, Debug)]
struct TreeShake {
    anchor: (i32, i32),
    frames: u32,
}

pub struct Game {
    assets: Assets,
    map: WorldMap,
    /// The island as it was made from the seed, before anything on it
    /// changed: a save keeps only what differs from it.
    made: WorldMap,
    tileset: Tileset,
    visual: VisualMap,
    harvest: Harvest,
    particles: Particles,
    /// Things lying on (or falling to) the ground, waiting to be picked up.
    items: Vec<Item>,
    rng: Rng,
    hitstop: u32,
    tree_shake: Option<TreeShake>,
    camera_shake: u32,
    player: Player,
    crabs: Vec<Crab>,
    fish: Vec<Fish>,
    /// The line in the water, if one's cast; and when (game minutes) new
    /// fish come in to replace the ones caught.
    line: Option<Line>,
    fish_due: Vec<u32>,
    fires: Vec<Campfire>,
    /// Signal fires set on the island, burning or not.
    signal_fires: Vec<SignalFire>,
    /// The day a signal fire was last lit (it has its chance of being seen
    /// once a day, however many are lit), and when (game minutes) its
    /// smoke is to be seen and the castaway rescued, if it is to be.
    signal_day: Option<u32>,
    rescue_at: Option<u32>,
    /// Asked at a signal fire, the castaway said to pour fish oil on it as
    /// it's lit (not saved: lighting it is under way).
    pour_oil: bool,
    /// Dev tools: whether smoke is always (`Some(true)`) or never
    /// (`Some(false)`) seen, instead of by chance.
    rescue_luck: Option<bool>,
    /// The day shells last washed up, and the most the beach holds (as
    /// many as it started with).
    shell_day: u32,
    shell_cap: usize,
    /// The most driftwood the west coast holds: as many logs as it had
    /// when the island was made.
    driftwood_cap: usize,
    /// The castaway has come upon the mill site (looked round, walked to
    /// its bank and thought), which happens once.
    mill_noticed: bool,
    /// Has come upon the river's narrow (which happens once).
    bridge_noticed: bool,
    /// How far the water mill has got.
    works: MillWorks,
    /// In-game minutes the castaway has been asleep.
    /// Which chunks of the island are awake (creatures move only there).
    regions: Regions,
    /// The island's seed (to start again from).
    seed: u32,
    /// The title screen, while it shows.
    title: Option<TitleState>,
    /// The saved game (as last loaded or written), if there is one.
    saved: Option<SaveData>,
    /// What the page must do with the saved game.
    outbox: Option<SaveOp>,
    /// The page keeps a saved game, even one that can't be read here (from
    /// a newer version, say): beginning anew asks first.
    save_kept: bool,
    /// When the game saves itself.
    autosave: AutoSave,
    /// What was carried when the bag was opened (as saved), while it's
    /// open: closing it saves if that changed.
    bag_opened_with: Option<String>,
    /// Where the castaway stood (feet, world px) before climbing up onto
    /// the mill's roof to build it, while they're up there: they come back
    /// down there once it's done.
    roofing_from: Option<(i32, i32)>,
    /// What the mill is being fitted out with, while the castaway works at
    /// it (its materials already taken).
    fitting: Option<Furnishing>,
    /// Going through the mill's door, while the screen fades.
    passage: Option<Passage>,
    /// Frames left of the sign that the game was saved.
    saved_sign: u32,
    /// How the game ended (the castaway collapsed, or was rescued), and
    /// frames since.
    end: Option<(Ending, u32)>,
    /// The rescue's cut showing, fitted to the PPU (made when it first
    /// shows, and put away for the next), and the plane drawn over the
    /// last.
    cut: Option<(usize, Scene)>,
    plane: Option<SpriteSet>,
    gulls: Vec<Gull>,
    camera: Camera,
    clock: GameClock,
    vitals: Vitals,
    /// Frames of the minute going by spent running (it tires faster).
    ran_frames: u32,
    /// Dev tools: whether something raw always (`Some(false)`) or never
    /// (`Some(true)`) upsets the stomach, instead of by chance.
    stomach_luck: Option<bool>,
    inventory: Inventory,
    /// The handcart on the island, once one has been made (its load is in
    /// `inventory.cart`).
    cart: Option<Cart>,
    input: InputState,
    paused: bool,
    /// The bag screen, while open (the world stands still).
    menu: Option<BagMenu>,
    frame_count: u32,
    base_cgram: Cgram,
    draw_list: DrawList,
    frame: FrameState,
    ppu: Ppu,
}

impl Game {
    pub fn new(seed: u32) -> Self {
        Self::build(seed, None)
    }

    /// A new game at the title screen, with `save` (the page's saved game,
    /// as JSON) to offer to CONTINUE if it can be read.
    pub fn with_save(seed: u32, save: Option<&str>) -> Self {
        let mut game = Self::new(seed);
        game.saved = save.and_then(|text| SaveData::from_json(text).ok());
        game.save_kept = save.is_some();
        game.title = Some(TitleState::opening(game.saved.is_some()));
        game
    }

    /// The island made from `seed`, as a new game or (with `save`) as it was
    /// saved.
    fn build(seed: u32, save: Option<&SaveData>) -> Self {
        let mut assets = Assets::load();
        let mut map = island::generate(seed);
        let made = map.clone();
        let driftwood_cap = driftwood::logs_on_west_coast(&map);
        if let Some(s) = save {
            for &((x, y), f) in &s.island.features {
                map.set_feature(x, y, f);
            }
            map.restore_trees(&s.island.fruit, &s.island.bark);
            // What the mill is fitted out with stands in its cells (set
            // again, so they're right whatever the save kept of them).
            for &f in &s.island.mill.furnished {
                map.furnish(f);
            }
        }
        let items = match save {
            Some(s) => s.island.items.iter().map(|l| Item::lying(l.kind, (l.at.0 as f32, l.at.1 as f32)).with_count(l.count)).collect(),
            None => map.loot.iter().map(|&(kind, (x, y))| Item::lying(kind, (x as f32, y as f32))).collect(),
        };
        let (tileset, visual) = Tileset::new(&map, &mut assets);
        let start = GameClock::new(1, START_HOUR);
        let clock = save.map_or(start.clone(), |s| s.clock.to_clock());
        let mut regions = Regions::new(&map, start.total_minutes());
        let player = match save {
            Some(s) => {
                let mut p = Player::new(s.castaway.at.0, s.castaway.at.1);
                p.facing = s.castaway.facing;
                p
            }
            // The story opens with the castaway washed up on the beach.
            None => Player::washed_up(map.spawn.0, map.spawn.1),
        };
        let crab_at: Vec<(i32, i32)> = match save {
            Some(s) => s.island.crabs.clone(),
            None => map.crab_spawns.clone(),
        };
        let crabs = crab_at.iter().enumerate().map(|(i, &(x, y))| Crab::new(x, y, seed.wrapping_add(i as u32 * 31))).collect();
        let (sx, sy) = (map.spawn.0 as f32, map.spawn.1 as f32);
        let gulls = vec![Gull::new((sx + 40.0, sy - 30.0), (120.0, 60.0), 0.006, 0.0), Gull::new((sx - 80.0, sy + 10.0), (90.0, 70.0), -0.008, 2.0)];
        if let Some(s) = save {
            regions.restore(&s.island.asleep_since);
        }
        let mut game = Self {
            assets,
            map,
            made,
            tileset,
            visual,
            harvest: save.map_or_else(Harvest::default, |s| Harvest::from_state(&s.island.harvest)),
            particles: Particles::new(seed ^ 0xFA57),
            items,
            rng: Rng::new(seed ^ 0xC0C0),
            hitstop: 0,
            tree_shake: None,
            camera_shake: 0,
            player,
            crabs,
            fish: Vec::new(),
            line: None,
            fish_due: save.map_or_else(Vec::new, |s| s.island.fish_due.clone()),
            fires: save.map_or_else(Vec::new, |s| s.island.fires.clone()),
            signal_fires: save.map_or_else(Vec::new, |s| s.island.signal_fires.clone()),
            signal_day: save.and_then(|s| s.island.signal_day),
            rescue_at: save.and_then(|s| s.island.rescue_at),
            pour_oil: false,
            rescue_luck: None,
            shell_day: 0,
            shell_cap: 0,
            driftwood_cap,
            mill_noticed: save.is_some_and(|s| s.castaway.mill_noticed),
            works: save.map_or_else(MillWorks::default, |s| s.island.mill.clone()),
            bridge_noticed: save.is_some_and(|s| s.castaway.bridge_noticed),
            regions,
            seed,
            title: save.is_none().then(|| TitleState::opening(false)),
            saved: None,
            outbox: None,
            save_kept: false,
            autosave: AutoSave::new(),
            bag_opened_with: None,
            roofing_from: None,
            fitting: None,
            passage: None,
            saved_sign: 0,
            end: None,
            cut: None,
            plane: None,
            gulls,
            camera: Camera::new(0),
            clock,
            vitals: save.map_or_else(Vitals::new, |s| s.vitals.clone()),
            ran_frames: 0,
            stomach_luck: None,
            inventory: Inventory::default(),
            cart: save.and_then(|s| s.island.cart.clone()),
            input: InputState::default(),
            paused: false,
            menu: None,
            frame_count: 0,
            base_cgram: base_cgram(),
            draw_list: DrawList::default(),
            frame: FrameState::default(),
            ppu: Ppu::new(),
        };
        match save {
            Some(s) => {
                let inv = &mut game.inventory;
                inv.bag = s.inventory.bag.clone();
                inv.torch = s.inventory.torch;
                inv.recipes = s.inventory.recipes.clone();
                inv.gear = s.inventory.gear;
                inv.cart = s.inventory.cart.clone();
                inv.had = s.inventory.had.clone();
                inv.shelf = s.inventory.shelf.clone();
                inv.looked_at = s.inventory.looked_at.clone();
                let outfit = s.castaway.outfit();
                if outfit != game.inventory.outfit {
                    game.equip(outfit);
                }
                game.fish = s.island.fish.iter().enumerate().map(|(i, &at)| Fish::new(at, seed ^ (i as u32).wrapping_mul(0x9E37))).collect();
                game.shell_day = s.island.shell_day;
                game.shell_cap = s.island.shell_cap;
                let now = game.clock.total_minutes();
                game.player.torch = game.inventory.torch_lit().then(|| game.inventory.torch_dying(now));
            }
            None => {
                for _ in 0..island::for_area(&game.map, FISH_COUNT) {
                    game.spawn_fish(false, false);
                }
                for _ in 0..LAKE_FISH {
                    game.spawn_fish(false, true);
                }
                game.shell_day = game.clock.day;
                game.shell_cap = game.shells_on_beach().len();
            }
        }
        let bounds = game.view_bounds();
        game.camera.follow_within(game.player.center(), bounds);
        game.stream_ground();
        game.wake_regions();
        game
    }

    /// The game as it stands, to be saved.
    fn save_data(&self) -> SaveData {
        let made = &self.made;
        let mut features = Vec::new();
        for y in 0..self.map.height() {
            for x in 0..self.map.width() {
                let f = self.map.feature(x, y);
                if f != made.feature(x, y) {
                    features.push(((x, y), f));
                }
            }
        }
        let (fruit, bark) = self.map.tree_changes(made);
        let inv = &self.inventory;
        SaveData {
            version: SAVE_VERSION,
            seed: self.seed,
            clock: save::Clock::of(&self.clock),
            vitals: self.vitals.clone(),
            castaway: save::Castaway {
                // Asleep in the bed, as if just up beside it.
                at: if self.player.in_bed { self.bedside_px() } else { self.player.body.px() },
                facing: self.player.facing,
                outfit: save::Castaway::outfit_index(&inv.outfit),
                mill_noticed: self.mill_noticed,
                bridge_noticed: self.bridge_noticed,
            },
            inventory: self.carried_now(),
            island: save::IslandState {
                features,
                fruit,
                bark,
                harvest: self.harvest.state(),
                items: self.items.iter().map(|i| save::Lying { kind: i.kind, count: i.count, at: i.ground() }).collect(),
                fires: self.fires.clone(),
                signal_fires: self.signal_fires.clone(),
                signal_day: self.signal_day,
                rescue_at: self.rescue_at,
                crabs: self.crabs.iter().map(|c| c.body.px()).collect(),
                fish: self.fish.iter().map(|f| f.pos()).collect(),
                fish_due: self.fish_due.clone(),
                shell_day: self.shell_day,
                shell_cap: self.shell_cap,
                asleep_since: self.regions.asleep_since(),
                cart: self.cart.clone(),
                mill: self.works.clone(),
            },
        }
    }

    /// What the castaway carries, as saved.
    fn carried_now(&self) -> save::Carried {
        let inv = &self.inventory;
        save::Carried {
            bag: inv.bag.clone(),
            torch: inv.torch,
            recipes: inv.recipes.clone(),
            gear: inv.gear,
            cart: inv.cart.clone(),
            had: inv.had.clone(),
            shelf: inv.shelf.clone(),
            looked_at: inv.looked_at.clone(),
        }
    }

    /// What is carried, as text to tell whether it changed.
    fn carried_json(&self) -> String {
        serde_json::to_string(&self.carried_now()).unwrap_or_default()
    }

    /// Saves the game: the page is handed it to keep.
    fn save(&mut self) {
        let data = self.save_data();
        self.outbox = Some(SaveOp::Write(data.to_json()));
        self.saved = Some(data);
        self.save_kept = true;
        self.autosave.saved();
        self.saved_sign = SAVED_SIGN_FRAMES;
    }

    /// Saves straight away whatever waits to be saved (the page is being
    /// left). Not at the title, nor once the game has ended.
    pub fn flush_save(&mut self) {
        if self.title.is_some() || self.end.is_some() {
            return;
        }
        let bag_changed = self.bag_opened_with.as_ref().is_some_and(|before| *before != self.carried_json());
        if self.autosave.pending() || bag_changed {
            self.save();
        }
    }

    /// Erases the saved game (the game has ended).
    fn erase_save(&mut self) {
        self.saved = None;
        self.save_kept = false;
        self.outbox = Some(SaveOp::Erase);
    }

    /// The page takes what is to be done with the saved game this frame, if
    /// anything.
    pub fn take_save_op(&mut self) -> Option<SaveOp> {
        self.outbox.take()
    }

    /// The title's items: START, and CONTINUE below it when there is a
    /// saved game.
    /// One frame of the title: moving the cursor and choosing. Beginning
    /// anew with a game saved asks first (it will be overwritten when the
    /// new game is saved).
    fn title_step(&mut self, mut title: TitleState) {
        let (up, down) = (self.input.pressed(Buttons::UP), self.input.pressed(Buttons::DOWN));
        let (left, right) = (self.input.pressed(Buttons::LEFT), self.input.pressed(Buttons::RIGHT));
        let ok = self.input.pressed(Buttons::START) || self.input.pressed(Buttons::A);
        if let Some(yes) = title.confirm {
            let yes = if left || right || up || down { !yes } else { yes };
            title.confirm = Some(yes);
            if self.input.pressed(Buttons::B) {
                title.confirm = None;
            } else if ok {
                if yes {
                    self.title = None;
                    return;
                }
                title.confirm = None;
            }
            self.title = Some(title);
            return;
        }
        let items = title_items(self.saved.is_some());
        let n = items.len();
        if up {
            title.choice = (title.choice + n - 1) % n;
        } else if down {
            title.choice = (title.choice + 1) % n;
        }
        title.choice = title.choice.min(n - 1);
        self.title = Some(title);
        if ok {
            match items[title.choice] {
                TitleItem::Start if self.save_kept => self.title = Some(TitleState { confirm: Some(false), ..title }),
                TitleItem::Start => self.title = None,
                TitleItem::Continue => self.continue_saved(),
            }
        }
    }

    /// Carries on from the saved game.
    fn continue_saved(&mut self) {
        let Some(data) = self.saved.take() else { return };
        let ppu = std::mem::take(&mut self.ppu);
        let input = std::mem::take(&mut self.input);
        *self = Game::build(data.seed, Some(&data));
        self.ppu = ppu;
        self.input = input;
        self.saved = Some(data);
        self.save_kept = true;
    }

    /// Advances the simulation by one frame (1/60 s).
    pub fn step(&mut self, buttons: u16) {
        self.input.update(buttons);
        if let Some(title) = self.title {
            // The title: the world waits (the sea still shimmers) until an
            // item is chosen.
            self.frame_count = self.frame_count.wrapping_add(1);
            self.title_step(title);
            return;
        }
        if let Some((ending, t)) = self.end {
            // The end: the screen fades, then START begins again. Rescued,
            // the cuts play first (START skips them to the records).
            self.frame_count = self.frame_count.wrapping_add(1);
            self.end = Some((ending, t + 1));
            let start = self.input.pressed(Buttons::START);
            match ending {
                Ending::Collapsed if start && t >= COLLAPSE_FADE + COLLAPSE_HOLD => self.start_over(),
                Ending::Rescued if start && ending::can_begin_again(t) => self.start_over(),
                Ending::Rescued if start && ending::view_at(t) != EndingView::Records => self.end = Some((ending, ending::skip(t))),
                _ => {}
            }
            return;
        }
        if self.input.pressed(Buttons::START) {
            self.paused = !self.paused;
        }
        if self.paused {
            return;
        }
        if self.input.pressed(Buttons::SELECT) {
            // Debug: skip ahead to preview lighting.
            self.clock.advance_minutes(3 * 60);
        }

        self.frame_count = self.frame_count.wrapping_add(1);
        self.saved_sign = self.saved_sign.saturating_sub(1);
        if let Some(passage) = self.passage {
            // The world waits while the screen fades.
            self.pass(passage);
            return;
        }
        let (near_fire, near_lathe, cart_at_hand) = (self.near_fire(), self.is_near_lathe(), self.cart_at_hand());
        // What's on the shelf goes into what's made in the mill.
        self.inventory.shelf_at_hand = self.is_in_mill_room() && self.works.has(Furnishing::Shelf);
        if self.menu.is_some() && self.bag_opened_with.is_none() {
            self.bag_opened_with = Some(self.carried_json());
        }
        if let Some(menu) = &mut self.menu {
            menu.near_fire = near_fire;
            menu.near_lathe = near_lathe;
            menu.cart_at_hand = cart_at_hand;
            // Only at the bench (the BUILD screen), where it's asked.
            if menu.works.is_some() {
                menu.fit_to_build = self.works.next().is_none_or(|s| self.vitals.fit_to_work(s.hours * 60));
                if self.works.next().is_none() {
                    menu.fit_for = Furnishing::ALL.into_iter().filter(|f| self.vitals.fit_to_work(f.hours() * 60)).collect();
                }
            }
            if menu.works.is_some() {
                menu.works = Some(self.works.clone());
            }
            let outcome = menu.update(&self.input, &mut self.inventory);
            // A tool lifted out to be moved is still in hand.
            let moving = menu.held_placement().map(|p| p.kind);
            self.resolve_menu(outcome);
            if moving.is_none() || moving != self.inventory.gear.hand {
                self.inventory.settle_hand();
            }
            // Something made in the bag may bring a recipe to mind: it
            // shows up in the list straight away.
            self.inventory.think();
            return;
        }
        if let Some(before) = self.bag_opened_with.take()
            && before != self.carried_json()
        {
            self.autosave.note(Cue::BagClosed);
        }
        if self.input.pressed(Buttons::X) && self.player.is_idle() {
            self.menu = Some(BagMenu::browse());
            return;
        }
        if self.player.is_asleep() {
            // No shelter's pitched under the mill's roof (its roof's as good).
            self.player.sheltered = self.inventory.bag.has(ItemKind::Shelter) && !self.is_in_mill_room() && !self.player.in_bed;
            self.sleep_on();
            self.autosave.slept(SLEEP_MINUTES_PER_FRAME);
        } else if self.player.is_working() {
            self.work_on();
        } else {
            if self.player.running {
                self.ran_frames += 1;
            }
            if self.clock.tick() {
                self.vitals.on_minute(Rest::Awake { ran: self.ran_frames });
                self.ran_frames = 0;
            }
        }
        if self.vitals.collapsed() {
            self.collapse();
            return;
        }
        if self.rescue_at.is_some_and(|at| self.clock.total_minutes() >= at) {
            self.rescue();
            return;
        }
        self.player.weak = self.vitals.starving();
        self.player.can_run = self.vitals.can_run();
        let queasy = self.vitals.has(Ailment::UpsetStomach);
        if queasy != self.player.queasy {
            // The colour drains from the face (and comes back).
            set_player_palettes(&mut self.base_cgram, &self.inventory.outfit, queasy);
        }
        self.player.queasy = queasy;
        // Any ailment turns the health gauge a poisonous purple.
        set_health_palette(&mut self.base_cgram, !self.vitals.ailments.is_empty());
        let now = self.clock.total_minutes();
        self.inventory.tend_torch(now);
        if self.clock.day != self.shell_day {
            self.shell_day = self.clock.day;
            self.wash_up_shells();
            self.wash_up_driftwood();
            let refilled = harvest::refill_clay_beds(&mut self.map);
            self.tileset.refresh(&self.map, &mut self.assets, &mut self.visual, &refilled);
        }
        self.player.torch = self.inventory.torch_lit().then(|| self.inventory.torch_dying(now));
        if self.inventory.think() {
            self.player.eureka();
        }
        self.tree_shake = self.tree_shake.filter(|s| s.frames > 0).map(|s| TreeShake { frames: s.frames - 1, ..s });
        self.camera_shake = self.camera_shake.saturating_sub(1);
        if self.hitstop > 0 {
            // Freeze the action for a few frames so the impact is felt.
            self.hitstop -= 1;
            return;
        }
        self.player.pulling = self.cart.as_ref().is_some_and(|c| c.pulled);
        self.tend_mill_door();
        let idle_before = self.player.is_idle();
        let situation = Situation {
            tool: self.inventory.gear.hand,
            has_leaf: self.inventory.bag.has(ItemKind::Leaf),
            has_knife: self.inventory.bag.has(ItemKind::StoneKnife),
            item_in_reach: self.pickup_target().map(|t| self.pickup_point(t)),
            fish_near: !self.inventory.carries(ItemKind::FishingRod) && self.fish_near(self.player.cast_point()),
            kindling: self.inventory.carries(ItemKind::FireDrill) && self.inventory.bag.has(ItemKind::Branch) && self.inventory.bag.has(ItemKind::PalmFrond),
            fuel: fuel(&self.inventory.bag).is_some(),
            hearth_full: !self.works.hearth_takes_fuel(self.clock.total_minutes()),
            fish_oil: self.inventory.bag.has(ItemKind::FishOil),
        };
        let was_asleep = self.player.is_asleep();
        let deed = self.player.update(&self.input, &self.map, situation);
        if let Some(cue) = deed.as_ref().and_then(autosave::cue_of) {
            self.autosave.note(cue);
        }
        match deed {
            Some(Deed::Chop(point)) => self.resolve_axe(point),
            Some(Deed::Dig(point)) => self.dig(point),
            Some(Deed::Shake(point)) => self.resolve_shake(point),
            Some(Deed::PickUp) => self.pick_up(),
            Some(Deed::Pull(point)) => self.pull(point),
            Some(Deed::DrawWater) => self.draw_water(),
            Some(Deed::Drink(water)) => self.consumed(water),
            Some(Deed::Thrust(from, to)) => self.resolve_thrust(from, to),
            Some(Deed::Eat(food)) => {
                self.consumed(food);
                let (fx, fy) = self.player.body.feet();
                self.particles.chips((fx, fy - 2), (0, 1), 2);
            }
            Some(Deed::Smoulder(spot)) => self.particles.ember(spot),
            Some(Deed::KickDust(feet)) => self.particles.dust(feet),
            Some(Deed::Cooked(dish)) => self.take(dish, 1),
            Some(Deed::LightFire(spot)) => self.light_fire(spot),
            Some(Deed::Cast(at)) => {
                self.line = Some(Line::new(at));
                self.particles.splash(at);
            }
            Some(Deed::Strike) => self.strike(),
            Some(Deed::ReelIn) => self.reel_in(),
            Some(Deed::StandLadder(foot, rows)) => self.stand_ladder(foot, rows),
            Some(Deed::LayBridge(span)) => self.lay_bridge(span),
            Some(Deed::SetWheel(site)) => self.set_wheel(site),
            Some(Deed::SetLathe(site)) => self.set_lathe(site),
            Some(Deed::SetStakes(site)) => self.set_stakes(site),
            Some(Deed::Built(site)) => self.built(site),
            // At the bench beside the mill's ground, with a stage of it
            // left to build: the BUILD screen opens.
            Some(Deed::AtBench) => {
                if self.works.next().is_some() {
                    self.menu = Some(BagMenu::building(self.works.clone()));
                    return;
                }
                // The workbench in the built mill: fitting it out.
                if self.is_in_mill_room() {
                    self.menu = Some(BagMenu::fitting_out(self.works.clone()));
                    return;
                }
            }
            Some(Deed::AtShelf) => {
                self.menu = Some(BagMenu::at_the_shelf());
                return;
            }
            // Pulling the cart, there's no lying down: it wants setting
            // down first.
            Some(Deed::ToBed) if self.player.pulling => self.player.ponder(),
            Some(Deed::ToBed) => self.player.lie_in_bed(self.map.bed_pillow()),
            Some(Deed::StokeHearth) => self.stoke_hearth(),
            Some(Deed::AskSignalOil(cell)) => {
                self.menu = Some(BagMenu::at_the_signal_fire(cell));
                return;
            }
            Some(Deed::SetSignalFire(cell)) => self.set_signal_fire(cell),
            Some(Deed::LightSignalFire(cell)) => self.light_signal_fire(cell),
            None => {
                // Y sets the cart down, or takes up the one set down close
                // by.
                if idle_before && self.player.is_idle() && !self.player.climbing && self.input.pressed(Buttons::Y) && self.handle_cart() {
                    self.autosave.note(Cue::Changed);
                }
            }
        }
        let (feet, facing) = (self.player.body.feet(), self.player.facing);
        if let Some(cart) = &mut self.cart {
            cart.hitch(feet, facing);
        }
        self.go_through_mill_door();
        self.come_upon_the_mill_site();
        self.come_upon_the_narrow();
        if was_asleep && !self.player.is_asleep() && self.player.in_bed {
            // Out of bed, getting up beside it.
            self.player.in_bed = false;
            let (x, y) = self.map.bedside();
            self.player.body.set_feet((x * TILE_SIZE + TILE_SIZE / 2, y * TILE_SIZE + TILE_SIZE / 2));
        }
        if was_asleep && !self.player.is_asleep() {
            // Each waking is saved.
            self.autosave.woke();
        }
        if self.player.moving {
            self.autosave.note(Cue::Moved);
        }
        self.tend_line();
        let now = self.clock.total_minutes();
        self.fires.retain(|f| !f.burnt_out(now));
        self.tend_signal_fires(now);
        for fire in &self.fires {
            if self.rng.range(EMBER_EVERY) == 0 {
                self.particles.ember(fire.flame_point());
            }
        }
        self.particles.update();
        // Things move only in the chunks around the screen.
        self.wake_regions();
        let regions = &self.regions;
        for item in self.items.iter_mut().filter(|i| regions.is_awake(i.ground())) {
            item.update(&self.map);
        }
        let pc = self.player.center();
        let (feet, moving) = (self.player.body.feet(), self.player.moving);
        for crab in self.crabs.iter_mut().filter(|c| regions.is_awake(c.center())) {
            crab.update(&self.map, feet, moving);
        }
        for gull in &mut self.gulls {
            gull.update();
        }
        for fish in self.fish.iter_mut().filter(|f| regions.is_awake(f.pos())) {
            fish.update(&self.map);
        }
        let now = self.clock.total_minutes();
        let due = self.fish_due.len();
        self.fish_due.retain(|&t| now < t);
        for _ in self.fish_due.len()..due {
            // Back where one is missing: the lake first, if it's short.
            let lake = self.fish_in_lake() < LAKE_FISH;
            self.spawn_fish(true, lake);
        }
        self.camera.follow_within(pc, self.view_bounds());
        // Saved when something worth keeping happened and the castaway is
        // calm (see `AutoSave`).
        let calm = self.player.is_idle() && !self.player.moving;
        if self.autosave.tick(calm) {
            self.save();
        }
    }

    /// Changes what the castaway wears: re-renders their frames into VRAM and
    /// updates their palettes.
    pub fn equip(&mut self, outfit: Outfit) {
        self.inventory.outfit = outfit;
        self.assets.sprites.player.rebuild(&mut self.assets.vram.sprites, &outfit);
        set_player_palettes(&mut self.base_cgram, &outfit, self.player.queasy);
    }

    /// Applies an axe blow landing at `point` (world px).
    fn resolve_axe(&mut self, point: (i32, i32)) {
        let (fx, fy) = self.player.body.feet();
        // Chips fly back toward the swinger.
        let toward = ((fx - point.0).signum(), (fy - point.1).signum());
        match self.harvest.axe(&mut self.map, point) {
            Strike::TreeHit { anchor, .. } => {
                self.hitstop = HITSTOP_TREE;
                self.particles.chips(point, toward, 4);
                let (canopy, ground) = palm_canopy(anchor);
                self.particles.leaves(canopy, ground, 2);
                self.start_sway(anchor, TREE_SHAKE_FRAMES);
            }
            Strike::TreeFelled { anchor, changed, timber, wood, fruit, fronds, bark, cones } => {
                self.hitstop = HITSTOP_FELL;
                self.camera_shake = CAMERA_SHAKE_FRAMES;
                self.tree_shake = None;
                self.particles.chips(point, toward, 7);
                let (canopy, ground) = palm_canopy(anchor);
                self.particles.leaves(canopy, ground, 14);
                self.tileset.refresh(&self.map, &mut self.assets, &mut self.visual, &changed);
                // The trunk falls apart into timber around the stump.
                let (ax, ay) = (anchor.0 * TILE_SIZE, anchor.1 * TILE_SIZE);
                let from = (ax as f32 + PALM_W as f32 / 2.0, (ay + 3 * TILE_SIZE) as f32);
                for _ in 0..wood {
                    self.drop_item(timber, anchor, from);
                }
                // Whatever coconuts were left come down with the crown.
                let flip = palm_flip(&self.map, anchor);
                for slot in 0..fruit {
                    self.drop_coconut(anchor, slot, flip);
                }
                // And a palm's fronds off the crown, or a broadleaf's bark
                // peeled off the fallen trunk.
                for _ in 0..if fronds { FELLED_FRONDS } else { 0 } {
                    self.drop_frond(anchor);
                }
                for _ in 0..bark {
                    self.drop_item(ItemKind::Bark, anchor, from);
                }
                // A pine's cones come down out of its tiers.
                for _ in 0..cones {
                    self.drop_cone(anchor);
                }
            }
            Strike::LogHit { .. } => {
                self.hitstop = HITSTOP_TREE;
                self.particles.chips(point, toward, 4);
            }
            Strike::LogSplit { anchor, changed } => {
                self.hitstop = HITSTOP_TREE;
                self.particles.chips(point, toward, 8);
                self.tileset.refresh(&self.map, &mut self.assets, &mut self.visual, &changed);
                // The log falls apart into branches where it lay.
                let n = harvest::DRIFTWOOD_BRANCHES + self.rng.chance(0.5) as u32;
                let (lx, ly) = (anchor.0 * TILE_SIZE, anchor.1 * TILE_SIZE);
                for k in 0..n {
                    let x = lx + 6 + (k as i32 * 20 / n as i32) + self.rng.between(-2, 2);
                    let y = ly + 9 + self.rng.between(-3, 3);
                    let from = (x as f32, (y - 6) as f32);
                    self.items.push(Item::falling(ItemKind::Branch, from, (x as f32, y as f32), (self.rng.between(-2, 2) as f32 * 0.1, 0.2)));
                }
            }
            Strike::Brush => {
                self.particles.leaves((point.0 - 6, point.1 - 10, 12, 8), point.1 + 2, 3);
            }
            Strike::Hard { .. } => {
                self.hitstop = HITSTOP_HARD;
                self.particles.sparks(point, 2);
            }
            Strike::Miss => {}
        }
    }

    /// Applies a spear jab sweeping the ground from `from` to `to`: the
    /// first crab on that line is caught and drops where it was.
    fn resolve_thrust(&mut self, from: (i32, i32), to: (i32, i32)) {
        let hit = self.crabs.iter().position(|c| near_line(c.center(), from, to, CRAB_HIT_RADIUS));
        let Some(i) = hit else {
            return;
        };
        let crab = self.crabs.swap_remove(i);
        let (x, y) = crab.center();
        self.hitstop = HITSTOP_CRAB;
        self.particles.sparks((x, y - 4), 2);
        self.items.push(Item::falling(ItemKind::Crab, (x as f32, y as f32 - 8.0), (x as f32, y as f32 + 1.0), (0.0, 0.3)));
    }

    /// Applies one shake of the trunk at `point` (world px).
    fn resolve_shake(&mut self, point: (i32, i32)) {
        let (anchor, sway) = match self.harvest.shake(&mut self.map, point) {
            Shaken::Miss => return,
            Shaken::Rustle { anchor, .. } => (anchor, TREE_SWAY_FRAMES),
            Shaken::Cone { anchor } => {
                self.drop_cone(anchor);
                (anchor, TREE_SHAKE_FRAMES)
            }
            Shaken::Drop { anchor, slot, changed } => {
                let flip = palm_flip(&self.map, anchor);
                self.tileset.refresh(&self.map, &mut self.assets, &mut self.visual, &[changed]);
                self.drop_coconut(anchor, slot, flip);
                (anchor, TREE_SHAKE_FRAMES)
            }
        };
        let (canopy, ground) = palm_canopy(anchor);
        if self.rng.chance(0.35) {
            self.particles.leaves(canopy, ground, 1);
        }
        // Now and then a dead frond comes away from a palm too.
        let palm = matches!(self.map.feature(anchor.0, anchor.1), crate::world::Feature::Palm(_));
        if palm && self.rng.chance(FROND_CHANCE) {
            self.drop_frond(anchor);
        }
        self.start_sway(anchor, sway);
    }

    /// The ladder in hand is stood against the cliff face whose foot is at
    /// `foot`: it leaves the bag and stays there.
    fn stand_ladder(&mut self, foot: (i32, i32), rows: i32) {
        if !self.inventory.bag.take_kind(ItemKind::Ladder) {
            return;
        }
        self.inventory.settle_hand();
        let mut changed = self.map.stand_ladder(foot, rows);
        // Its tops may stick up into the cell above (see `Tileset`).
        changed.push((foot.0, foot.1 - rows - 1));
        self.tileset.refresh(&self.map, &mut self.assets, &mut self.visual, &changed);
    }

    /// The log bridge in hand is laid across the river along `span`: it
    /// leaves the bag and stays there.
    fn lay_bridge(&mut self, span: BridgeSpan) {
        if !self.inventory.bag.take_kind(ItemKind::LogBridge) {
            return;
        }
        self.inventory.settle_hand();
        let changed = self.map.lay_bridge(span);
        self.tileset.refresh(&self.map, &mut self.assets, &mut self.visual, &changed);
    }

    /// The water wheel in hand is set in the river off the mill site's
    /// bank `site`: it leaves the bag and stays there, turning.
    fn set_wheel(&mut self, site: (i32, i32)) {
        if !self.inventory.bag.take_kind(ItemKind::WaterWheel) {
            return;
        }
        self.inventory.settle_hand();
        let changed = self.map.set_wheel(site);
        self.tileset.refresh(&self.map, &mut self.assets, &mut self.visual, &changed);
    }

    /// The wood lathe in hand is set on the bank beside the water wheel off
    /// the mill site's bank `site`: it leaves the bag and stays there, the
    /// wheel driving it.
    fn set_lathe(&mut self, site: (i32, i32)) {
        if !self.inventory.bag.take_kind(ItemKind::WoodLathe) {
            return;
        }
        self.inventory.settle_hand();
        let changed = self.map.set_lathe(site);
        self.tileset.refresh(&self.map, &mut self.assets, &mut self.visual, &changed);
        self.step_clear();
    }

    /// The signal fire in hand is set down in the cell ahead: it leaves the
    /// bag and stays there (not on a campfire).
    fn set_signal_fire(&mut self, cell: (i32, i32)) {
        let middle = (cell.0 * TILE_SIZE + TILE_SIZE / 2, cell.1 * TILE_SIZE + TILE_SIZE / 2);
        if !self.map.signal_fire_spot(cell.0, cell.1) || self.fires.iter().any(|f| f.within(middle, FIRE_SPACING)) {
            return;
        }
        if !self.inventory.bag.take_kind(ItemKind::SignalFire) {
            return;
        }
        self.inventory.settle_hand();
        self.map.set_signal_fire(cell);
        self.tileset.refresh(&self.map, &mut self.assets, &mut self.visual, &[cell]);
        self.signal_fires.push(SignalFire::new(cell));
    }

    /// The drill has caught in the signal fire in the cell: a branch and a
    /// palm frond burn on it, sending up smoke for a day. The first lit
    /// each day has its chance of being seen.
    fn light_signal_fire(&mut self, cell: (i32, i32)) {
        let now = self.clock.total_minutes();
        let Some(i) = self.signal_fires.iter().position(|f| f.cell == cell && !f.burning()) else { return };
        let bag = &mut self.inventory.bag;
        if !(self.inventory.gear.on_belt(ItemKind::FireDrill) || bag.has(ItemKind::FireDrill)) || !bag.has(ItemKind::Branch) || !bag.has(ItemKind::PalmFrond) {
            return;
        }
        bag.take_kind(ItemKind::Branch);
        bag.take_kind(ItemKind::PalmFrond);
        // Fish oil poured on (if the castaway said so when asked): the
        // smoke goes up black.
        let black = std::mem::take(&mut self.pour_oil) && bag.take_kind(ItemKind::FishOil);
        self.signal_fires[i].lit_at = Some(now);
        self.signal_fires[i].black = black;
        self.map.set_feature(cell.0, cell.1, Feature::SignalFire(true));
        self.particles.sparks((cell.0 * TILE_SIZE + TILE_SIZE / 2, cell.1 * TILE_SIZE - 4), 3);
        if self.signal_day != Some(self.clock.day) {
            self.signal_day = Some(self.clock.day);
            let seen = self.rescue_luck.unwrap_or_else(|| self.rng.chance(rescue_chance(black)));
            if seen {
                // Made sure of (dev tools), it's seen as soon as it can be.
                let after = if self.rescue_luck == Some(true) { SEEN_FROM } else { self.rng.between(SEEN_FROM as i32, SEEN_UNTIL as i32) as u32 };
                self.rescue_at = Some(now + after);
            }
        }
    }

    /// Signal fires that have had their day of burning go out (the hearth
    /// stays, to be lit again).
    fn tend_signal_fires(&mut self, now: u32) {
        for fire in self.signal_fires.iter_mut().filter(|f| f.burnt_out(now)) {
            fire.lit_at = None;
            fire.black = false;
            self.map.set_feature(fire.cell.0, fire.cell.1, Feature::SignalFire(false));
        }
    }

    /// The marking stakes in hand are driven in round the mill's ground off
    /// the mill site's bank `site`, rope run between them: they leave the
    /// bag, and the ground is marked out for the mill.
    fn set_stakes(&mut self, site: (i32, i32)) {
        if !self.inventory.bag.take_kind(ItemKind::MarkingStakes) {
            return;
        }
        self.inventory.settle_hand();
        let changed = self.map.set_stakes(site);
        self.tileset.refresh(&self.map, &mut self.assets, &mut self.visual, &changed);
        self.step_clear();
    }

    /// Steps the castaway out of anything just set down where they stand
    /// (the lathe takes cells they may be at the edge of): the shortest way
    /// out, west, south, east or north.
    fn step_clear(&mut self) {
        let feet = self.player.body.world_hitbox();
        if !self.map.is_solid_rect(feet) {
            return;
        }
        let way = (1..=2 * TILE_SIZE).flat_map(|d| [(-d, 0), (0, d), (d, 0), (0, -d)]).find(|&(dx, dy)| !self.map.is_solid_rect(feet.translate(dx, dy)));
        if let Some((dx, dy)) = way {
            self.player.body.x += crate::common::math::px_to_fx(dx);
            self.player.body.y += crate::common::math::px_to_fx(dy);
        }
    }

    /// The first time the castaway comes near the mill site (with no wheel
    /// in the river yet), they look round, walk over to its bank and stop
    /// to think, unless a wheel is already in hand.
    fn come_upon_the_mill_site(&mut self) {
        let Some(site) = self.map.spots.mill.filter(|_| !self.mill_noticed && !self.map.has_wheel()) else { return };
        let (x, y) = self.player_tile();
        if !self.player.is_idle() || (x - site.0).pow(2) + (y - site.1).pow(2) > MILL_NOTICE_CELLS * MILL_NOTICE_CELLS {
            return;
        }
        // Only on its own side of the river (the west coast): seen from the
        // first area's bank, across the water, it's nothing to go to yet.
        if !WestCoast::of(&self.map).contains(x, y) {
            return;
        }
        self.mill_noticed = true;
        self.autosave.note(Cue::Changed);
        self.player.survey(site, Dir::Right, self.inventory.gear.hand != Some(ItemKind::WaterWheel));
    }

    /// Coming near the river's narrow from the first area for the first
    /// time (no bridge laid yet): an idea strikes, the castaway looks
    /// round, walks to the bank a log bridge would rest on, and turns to
    /// the river; without one in hand, stops to think there.
    fn come_upon_the_narrow(&mut self) {
        let Some(bank) = self.map.bridge_bank().filter(|_| !self.bridge_noticed) else { return };
        let (x, y) = self.player_tile();
        if !self.player.is_idle()
            || (x - bank.0).pow(2) + (y - bank.1).pow(2) > BRIDGE_NOTICE_CELLS * BRIDGE_NOTICE_CELLS
            || WestCoast::of(&self.map).contains(x, y)
        {
            return;
        }
        self.bridge_noticed = true;
        self.autosave.note(Cue::Changed);
        self.player.survey(bank, Dir::Left, self.inventory.gear.hand != Some(ItemKind::LogBridge));
    }

    /// Sends the coconut hanging in `slot` of the palm at `anchor` falling.
    fn drop_coconut(&mut self, anchor: (i32, i32), slot: u8, flip: bool) {
        let (hx, hy) = coconut_spots()[slot as usize];
        let hx = if flip { PALM_W as f32 - hx } else { hx };
        let from = ((anchor.0 * TILE_SIZE) as f32 + hx, (anchor.1 * TILE_SIZE) as f32 + hy);
        self.drop_item(ItemKind::Coconut, anchor, from);
    }

    /// Sends a cone down from somewhere in the tiers of the pine at
    /// `anchor`.
    fn drop_cone(&mut self, anchor: (i32, i32)) {
        let ((x, y, w, h), _) = palm_canopy(anchor);
        let from = ((x + self.rng.between(10, w - 10)) as f32, (y + self.rng.between(h / 3, h - 4)) as f32);
        self.drop_item(ItemKind::PineCone, anchor, from);
    }

    /// Sends a frond down from somewhere in the crown of the palm at
    /// `anchor`.
    fn drop_frond(&mut self, anchor: (i32, i32)) {
        let ((x, y, w, h), _) = palm_canopy(anchor);
        let from = ((x + self.rng.between(8, w - 8)) as f32, (y + self.rng.between(4, h / 2)) as f32);
        self.drop_item(ItemKind::PalmFrond, anchor, from);
    }

    /// Sends an item from `from` (world px, as seen on screen) to a free spot
    /// of dry ground at the foot of the palm at `anchor`.
    fn drop_item(&mut self, kind: ItemKind, anchor: (i32, i32), from: (f32, f32)) {
        let (ax, ay) = (anchor.0 * TILE_SIZE, anchor.1 * TILE_SIZE);
        // Below the tree or beside the trunk (whose cell is solid).
        let trunk_x = ax as f32 + PALM_W as f32 / 2.0;
        let base_y = (ay + 3 * TILE_SIZE) as f32;
        let mut to = (from.0, base_y + 20.0);
        for _ in 0..8 {
            let side = if self.rng.chance(0.5) { -1.0 } else { 1.0 };
            let cand = if self.rng.chance(0.6) {
                (trunk_x + self.rng.between(-14, 14) as f32, base_y + 16.0 + self.rng.between(2, 10) as f32)
            } else {
                (trunk_x + side * self.rng.between(11, 18) as f32, base_y + self.rng.between(8, 15) as f32)
            };
            let p = self.map.props_at_px(cand.0 as i32, cand.1 as i32);
            let dry = self.map.terrain(cand.0 as i32 / TILE_SIZE, cand.1 as i32 / TILE_SIZE).is_land();
            if !p.solid && dry {
                to = cand;
                break;
            }
        }
        let roll = (((to.0 - trunk_x) * 0.04).clamp(-0.6, 0.6), self.rng.between(0, 4) as f32 * 0.1);
        self.items.push(Item::falling(kind, from, to, roll));
    }

    /// The resting item nearest the castaway's feet or reach point, if any
    /// is close enough to pick up.
    fn item_in_reach(&self) -> Option<usize> {
        let (feet, reach) = (self.player.body.feet(), self.player.reach_point());
        self.items
            .iter()
            .enumerate()
            .filter_map(|(i, item)| {
                let d = [item.reach(feet), item.reach(reach)].into_iter().flatten().reduce(f32::min)?;
                Some((i, d))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(i, _)| i)
    }

    /// What A would pick up: an item lying within reach, or else a shell
    /// on the sand underfoot or just ahead.
    fn pickup_target(&self) -> Option<Pickup> {
        if let Some(i) = self.item_in_reach() {
            return Some(Pickup::Item(i));
        }
        [self.player.body.feet(), self.player.reach_point()]
            .into_iter()
            .map(|(x, y)| (x.div_euclid(TILE_SIZE), y.div_euclid(TILE_SIZE)))
            .find(|&(x, y)| ground_pickup(self.map.feature(x, y)).is_some())
            .map(Pickup::Ground)
    }

    /// Puts a fish somewhere it can swim, in the lake or else in the sea
    /// and the river; `unseen` keeps it off screen.
    fn spawn_fish(&mut self, unseen: bool, lake: bool) {
        let (w, h) = self.map.island_pixel_size();
        for _ in 0..4000 {
            let p = (self.rng.between(0, w - 1), self.rng.between(0, h - 1));
            if fish::swimmable(&self.map, p) && self.in_lake(p) == lake && !(unseen && self.camera.sees(p.0 - 16, p.1 - 16, 32, 32)) {
                let seed = self.rng.range(u32::MAX);
                self.fish.push(Fish::new(p, seed));
                return;
            }
        }
    }

    /// Whether the world pixel is in the lake on the high ground.
    fn in_lake(&self, (x, y): (i32, i32)) -> bool {
        self.map.is_high(x.div_euclid(TILE_SIZE), y.div_euclid(TILE_SIZE))
    }

    fn fish_in_lake(&self) -> usize {
        self.fish.iter().filter(|f| self.in_lake(f.pos())).count()
    }

    /// Whether a fish swims near the point (and there's water there).
    fn fish_near(&self, (x, y): (i32, i32)) -> bool {
        fish::swimmable(&self.map, (x, y))
            && self.fish.iter().any(|f| {
                let (fx, fy) = f.pos();
                (fx - x).pow(2) + (fy - y).pow(2) < FISH_NEAR * FISH_NEAR
            })
    }

    /// The line in the water this frame; it's gone if the castaway stopped
    /// fishing some other way.
    fn tend_line(&mut self) {
        let Some(line) = &mut self.line else { return };
        if !self.player.is_fishing() {
            line.let_go(&mut self.fish);
            self.line = None;
            return;
        }
        if line.update(&mut self.fish, &mut self.rng) {
            // The float goes under with a splash.
            self.particles.splash(line.float);
        }
    }

    /// B while fishing: a fish biting is landed at the castaway's feet;
    /// otherwise the line just comes in and frightens off any fish there.
    fn strike(&mut self) {
        let Some(mut line) = self.line.take() else { return };
        let Some(i) = line.fish.filter(|_| line.biting()) else {
            line.let_go(&mut self.fish);
            return;
        };
        self.fish.swap_remove(i);
        self.fish_due.push(self.clock.total_minutes() + FISH_RETURN_MINUTES);
        // It comes flying out of the water to land beside the castaway.
        let (fx, fy) = self.player.body.feet();
        let side = if line.float.0 < fx { 10 } else { -10 };
        let to = ((fx + side) as f32, (fy + 2) as f32);
        // Swung up out of the water: it drops onto the sand from head height.
        let from = ((line.float.0 + fx + side) as f32 / 2.0, to.1 - 28.0);
        self.particles.splash(line.float);
        self.items.push(Item::falling(ItemKind::Fish, from, to, (0.0, 0.0)));
        self.hitstop = HITSTOP_CRAB;
    }

    /// Moving off while fishing: the line comes in.
    fn reel_in(&mut self) {
        if let Some(mut line) = self.line.take() {
            line.let_go(&mut self.fish);
        }
    }

    /// The cells with a shell lying on the sand.
    fn shells_on_beach(&self) -> Vec<(i32, i32)> {
        (0..self.map.height()).flat_map(|y| (0..self.map.width()).map(move |x| (x, y))).filter(|&(x, y)| self.map.feature(x, y) == Feature::Shell).collect()
    }

    /// A new day: the tide leaves a few shells on open sand near the water,
    /// until the beach holds as many as it started with.
    fn wash_up_shells(&mut self) {
        let room = self.shell_cap.saturating_sub(self.shells_on_beach().len()).min(island::for_area(&self.map, SHELLS_PER_DAY));
        let (w, h) = (self.map.width(), self.map.height());
        let near_water = |map: &WorldMap, x: i32, y: i32| (-2..=2).any(|dy| (-2..=2).any(|dx| map.terrain(x + dx, y + dy).holds_water()));
        let mut changed = Vec::new();
        for _ in 0..400 {
            if changed.len() >= room {
                break;
            }
            let (x, y) = (self.rng.between(0, w - 1), self.rng.between(0, h - 1));
            let open =
                self.map.terrain(x, y) == Terrain::Sand && !self.map.is_high(x, y) && self.map.feature(x, y) == Feature::None && !self.map.is_riverside(x, y);
            if open && near_water(&self.map, x, y) {
                self.map.set_feature(x, y, Feature::Shell);
                changed.push((x, y));
            }
        }
        self.tileset.refresh(&self.map, &mut self.assets, &mut self.visual, &changed);
    }

    /// A new morning: now and then the tide leaves a log on the west
    /// coast's sand, clear of the castaway, the crabs, dropped items and
    /// fires.
    fn wash_up_driftwood(&mut self) {
        let cell = |(px, py): (i32, i32)| (px.div_euclid(TILE_SIZE), py.div_euclid(TILE_SIZE));
        let (cx, cy) = cell(self.player.body.feet());
        let mut taken: Vec<(i32, i32)> = self.items.iter().map(|i| cell(i.ground())).collect();
        taken.extend(self.crabs.iter().map(|c| cell(c.body.feet())));
        taken.extend(self.fires.iter().map(|f| cell((f.x, f.y))));
        let keep_clear = |x: i32, y: i32| (x - cx).abs() <= 3 && (y - cy).abs() <= 3 || taken.contains(&(x, y));
        let changed = driftwood::wash_up(&mut self.map, &mut self.rng, self.driftwood_cap, &keep_clear);
        self.tileset.refresh(&self.map, &mut self.assets, &mut self.visual, &changed);
    }

    /// Where a pickup lies (ground px), to turn toward it.
    fn pickup_point(&self, target: Pickup) -> (i32, i32) {
        match target {
            Pickup::Item(i) => self.items[i].ground(),
            Pickup::Ground((x, y)) => (x * TILE_SIZE + 8, y * TILE_SIZE + 11),
        }
    }

    /// Takes what's within reach (an item, if it hasn't rolled away, or a
    /// shell off the sand) into the bag.
    fn pick_up(&mut self) {
        let Some(target) = self.pickup_target() else {
            return;
        };
        let i = match target {
            Pickup::Item(i) => i,
            Pickup::Ground((x, y)) => {
                let Some(kind) = ground_pickup(self.map.feature(x, y)) else { return };
                self.map.set_feature(x, y, Feature::None);
                self.tileset.refresh(&self.map, &mut self.assets, &mut self.visual, &[(x, y)]);
                self.particles.sparks((x * TILE_SIZE + 8, y * TILE_SIZE + 7), 1);
                self.take(kind, 1);
                // The pine cone at the foot of the cliff's recess, rolled
                // down from the pine above: the castaway looks up and
                // round, and (with no ladder) tries to climb.
                if kind == ItemKind::PineCone && self.map.spots.ladder == Some((x, y)) {
                    self.player.notice_after_pickup(!self.inventory.bag.has(ItemKind::Ladder));
                }
                return;
            }
        };
        let item = self.items.swap_remove(i);
        let (x, y) = item.ground();
        self.particles.sparks((x, y - 4), 1);
        self.take(item.kind, item.count);
    }

    /// Tugs at the plant at `point`: takes the vine off a climbing plant,
    /// peels a strip of bark off a broadleaf trunk, or takes a handful of
    /// leaves off a bush (only rustling one already stripped today).
    fn pull(&mut self, point: (i32, i32)) {
        if harvest::is_broadleaf_trunk(&self.map, point) {
            // Bark won't come away by hand: it takes a knife's edge.
            if !self.inventory.bag.has(ItemKind::StoneKnife) {
                return;
            }
            if let Some((x, y)) = harvest::strip_bark(&mut self.map, point) {
                self.tileset.refresh(&self.map, &mut self.assets, &mut self.visual, &[(x, y)]);
                let (px, py) = (x * TILE_SIZE, y * TILE_SIZE);
                self.particles.chips((px + 8, py + 6), (0, 1), 3);
                self.take(ItemKind::Bark, 1);
            }
            return;
        }
        // Vine has to be cut free; without a knife it won't give.
        if harvest::can_pull_vine(&self.map, point) && !self.inventory.bag.has(ItemKind::StoneKnife) {
            return;
        }
        if let Some((x, y)) = harvest::pull_vine(&mut self.map, point) {
            self.tileset.refresh(&self.map, &mut self.assets, &mut self.visual, &[(x, y)]);
            let (px, py) = (x * TILE_SIZE, y * TILE_SIZE);
            self.particles.leaves((px + 2, py, 12, 10), py + 14, 3);
            self.take(ItemKind::Vine, 1);
            return;
        }
        let (px, py) = (point.0.div_euclid(TILE_SIZE) * TILE_SIZE, point.1.div_euclid(TILE_SIZE) * TILE_SIZE);
        if self.harvest.pick_leaves(&self.map, point, self.clock.day).is_some() {
            self.particles.leaves((px + 2, py, 12, 10), py + 14, 2);
            self.take(ItemKind::Leaf, harvest::BUSH_LEAVES);
        } else {
            self.particles.leaves((px + 2, py, 12, 10), py + 14, 1);
        }
    }

    /// Lifts out what the tool in hand loosened at `point`: soil from bare
    /// earth with a digging stick, clay from a clay bed with a bamboo spade
    /// (leaving it dug until the morning).
    fn dig(&mut self, point: (i32, i32)) {
        let cell = (point.0.div_euclid(TILE_SIZE), point.1.div_euclid(TILE_SIZE));
        match (self.inventory.gear.hand, harvest::diggable(&self.map, cell)) {
            (Some(ItemKind::DiggingStick), Some(harvest::Diggable::Earth)) => self.take(ItemKind::Soil, harvest::SOIL_PER_DIG),
            (Some(ItemKind::BambooSpade), Some(harvest::Diggable::Clay { dug: false })) => {
                if let Some(cells) = harvest::dig_clay(&mut self.map, cell) {
                    self.tileset.refresh(&self.map, &mut self.assets, &mut self.visual, &cells);
                    self.take(ItemKind::Clay, harvest::CLAY_PER_DIG);
                }
            }
            _ => {}
        }
    }

    /// Scoops water up in a leaf folded into a cup.
    fn draw_water(&mut self) {
        if self.inventory.bag.take_kind(ItemKind::Leaf) {
            self.take(ItemKind::RawWater, 1);
        }
    }

    /// Something made or given that never goes in the bag: a torch is
    /// lit and carried; a handcart is pulled off behind (there's only ever
    /// the one).
    fn carry_off(&mut self, kind: ItemKind) {
        match kind {
            ItemKind::Handcart => {
                if self.cart.is_none() {
                    self.cart = Some(Cart::behind(self.player.body.feet(), self.player.facing));
                    self.inventory.cart = Some(Bag::with_size(CART_W, CART_ROWS));
                }
            }
            _ => self.inventory.light_torch(self.clock.total_minutes()),
        }
    }

    /// Y pressed: a cart being pulled is set down where it stands; a cart
    /// set down close by (its load at hand), whichever way the castaway
    /// faces, is taken up. Whether it was.
    fn handle_cart(&mut self) -> bool {
        let at_hand = self.cart_at_hand();
        let Some(cart) = &mut self.cart else { return false };
        if cart.pulled {
            cart.set_down();
            return true;
        }
        if !at_hand {
            return false;
        }
        // Step round to its poles, facing the way they point, and take
        // hold (not if there's no room to stand there).
        let (dx, dy) = cart::hitch_offset(cart.poles);
        let to = (cart.at.0 - dx, cart.at.1 - dy);
        let (fx, fy) = self.player.body.feet();
        let feet = self.player.body.world_hitbox().translate(to.0 - fx, to.1 - fy);
        if self.map.is_solid_rect(feet) {
            self.player.ponder();
            return false;
        }
        self.player.body.x += crate::common::math::px_to_fx(to.0 - fx);
        self.player.body.y += crate::common::math::px_to_fx(to.1 - fy);
        self.player.facing = cart.poles;
        cart.take_up();
        true
    }

    /// Whether the cart's load is at hand: it's being pulled, or set down
    /// close by.
    fn cart_at_hand(&self) -> bool {
        let (fx, fy) = self.player.body.feet();
        self.cart.as_ref().is_some_and(|c| c.pulled || (c.at.0 - fx).pow(2) + (c.at.1 - fy).pow(2) <= CART_REACH * CART_REACH)
    }

    /// Puts `count` of something just obtained into the bag; if they don't
    /// all fit, the bag opens with the rest in hand so room can be made (or
    /// they can be left).
    fn take(&mut self, kind: ItemKind, count: u32) {
        if !kind.goes_in_bag() {
            // A torch lit at the fire is carried straight off.
            self.carry_off(kind);
            return;
        }
        let left = self.inventory.bag.stow(kind, count);
        if left > 0 {
            self.menu = Some(BagMenu::holding(kind, left));
        }
        self.inventory.took(kind);
    }

    /// Acts on what happened in the bag screen.
    fn resolve_menu(&mut self, outcome: Outcome) {
        match outcome {
            Outcome::Stay => {}
            Outcome::Close => self.menu = None,
            Outcome::Use(kind) => {
                if kind.usage() == Some(Use::Light)
                    && let Err(why) = self.fire_spot()
                {
                    // Stay in the bag and say why.
                    if let Some(menu) = &mut self.menu {
                        menu.notice = Some(why);
                    }
                    return;
                }
                self.menu = None;
                self.player.consume(kind);
            }
            Outcome::Drop(kind, count) => self.put_down(kind, count),
            Outcome::Cook(i) => self.start_cooking(&RECIPES[i]),
            Outcome::Made(kind) => self.carry_off(kind),
            Outcome::Deliver(kind) => self.deliver(kind),
            Outcome::Furnish(f) => self.start_furnishing(f),
            Outcome::LightSignal(cell, pour) => {
                self.menu = None;
                self.pour_oil = pour;
                self.player.light_signal(cell);
            }
            Outcome::Build => {
                let Some(site) = self.map.spots.mill else { return };
                let Some(stage) = self.works.next() else { return };
                self.menu = None;
                let frames = stage.hours * 60 / SLEEP_MINUTES_PER_FRAME;
                // Once the walls are up, the middle of the ground can't be
                // walked to from the bench: the roof's left, built up on
                // it (from the scaffold). It's drawn from the start, the
                // castaway at work up on it over the middle of the ground.
                if self.map.has_mill_walls() {
                    let changed = self.map.start_roofing(site);
                    self.tileset.refresh(&self.map, &mut self.assets, &mut self.visual, &changed);
                    self.roofing_from = Some(self.player.body.feet());
                    let (x, y) = crate::world::map::stakes_stand(site);
                    self.player.body.set_feet((x * TILE_SIZE + TILE_SIZE / 2, y * TILE_SIZE + TILE_SIZE / 2));
                    self.player.start_work_here(site, frames);
                } else {
                    self.player.start_work(site, frames);
                }
            }
        }
    }

    /// Sets to fitting out the mill with `f`: its materials are taken (from
    /// the bag, then the cart, then the shelf), and the castaway walks to
    /// where it's to stand and works at it.
    fn start_furnishing(&mut self, f: Furnishing) {
        self.menu = None;
        let cart_at_hand = self.cart_at_hand();
        let inv = &mut self.inventory;
        for &(kind, n) in f.needs() {
            let mut left = n;
            while left > 0 && inv.bag.take_kind(kind) {
                left -= 1;
            }
            if cart_at_hand && let Some(cart) = &mut inv.cart {
                while left > 0 && cart.take_kind(kind) {
                    left -= 1;
                }
            }
            if inv.shelf_at_hand {
                left -= inv.shelf.take(kind, left);
            }
            debug_assert_eq!(left, 0, "materials at hand");
        }
        self.fitting = Some(f);
        let (stand, face) = self.map.furnishing_stand(f);
        self.player.start_work_at(stand, stand, face, f.hours() * 60 / SLEEP_MINUTES_PER_FRAME);
        self.autosave.note(Cue::Changed);
    }

    /// Leaves as many of `kind` as the mill's next stage still takes at
    /// its site: from the bag first, then the cart (while it's at hand).
    fn deliver(&mut self, kind: ItemKind) {
        let want = self.works.still_needs(kind);
        let mut left = 0;
        while left < want && self.inventory.bag.take_kind(kind) {
            left += 1;
        }
        if self.cart_at_hand()
            && let Some(cart) = &mut self.inventory.cart
        {
            while left < want && cart.take_kind(kind) {
                left += 1;
            }
        }
        self.works.deliver(kind, left);
        if let Some(menu) = &mut self.menu {
            menu.works = Some(self.works.clone());
            menu.notice = Some(match (want, left) {
                (0, _) => "THAT'S ALL IT TAKES",
                (_, 0) => "NONE AT HAND",
                _ => "PUT IN AT THE SITE",
            });
        }
    }

    /// One frame of building the mill: time hurries on as in sleep, but
    /// the work wears the castaway as hours awake do.
    fn work_on(&mut self) {
        for _ in 0..SLEEP_MINUTES_PER_FRAME {
            self.clock.advance_minutes(1);
            self.vitals.on_minute(Rest::Awake { ran: 0 });
            if self.vitals.collapsed() {
                return;
            }
        }
    }

    /// The mill's door slides a step open while the castaway (or the cart)
    /// is by the doorway, outside or in, and a step shut while not, every
    /// few frames.
    fn tend_mill_door(&mut self) {
        if !self.frame_count.is_multiple_of(MILL_DOOR_FRAMES) {
            return;
        }
        let Some(site) = self.map.spots.mill.filter(|_| self.map.has_mill_roof()) else { return };
        let room_door = self.map.mill_room_door();
        let near = |(fx, fy): (i32, i32)| {
            let (x, y) = (fx.div_euclid(TILE_SIZE), fy.div_euclid(TILE_SIZE));
            crate::world::map::mill_doorway(site).iter().any(|&(dx, dy)| dx == x && (dy - 1..=dy + 1).contains(&y))
                || room_door.iter().any(|&(dx, dy)| dx == x && (dy - 2..=dy).contains(&y))
        };
        let cart = self.cart.as_ref().is_some_and(|c| {
            let r = c.rect();
            near((r.x, r.y + r.h - 1)) || near((r.x + r.w - 1, r.y + r.h - 1)) || near((r.x, r.y)) || near((r.x + r.w - 1, r.y))
        });
        let changed = self.map.slide_mill_door(near(self.player.body.feet()) || cart);
        self.tileset.refresh(&self.map, &mut self.assets, &mut self.visual, &changed);
    }

    /// The castaway walking into the mill's door (once the roof is on), in
    /// from outside or out from the room: the screen starts to fade, to
    /// put them on the other side. In, they stand two rows in from the
    /// room's door (room for a cart pulled behind them), facing in; out,
    /// just outside the hut's, facing away from it.
    fn go_through_mill_door(&mut self) {
        let Some(site) = self.map.spots.mill.filter(|_| self.map.has_mill_roof() && self.player.moving) else { return };
        let (fx, fy) = self.player.body.feet();
        let cell = (fx.div_euclid(TILE_SIZE), fy.div_euclid(TILE_SIZE));
        let doorway = crate::world::map::mill_doorway(site);
        let room_door = self.map.mill_room_door();
        let between = |cells: [(i32, i32); 2]| (cells[1].0 * TILE_SIZE, cells[0].1);
        let to = if doorway.contains(&cell) {
            let (x, y) = between(room_door);
            ((x, (y - 2) * TILE_SIZE + TILE_SIZE / 2), Dir::Up)
        } else if room_door.contains(&cell) {
            let (x, y) = between(doorway);
            ((x, (y + 1) * TILE_SIZE + TILE_SIZE / 2), Dir::Down)
        } else {
            return;
        };
        self.passage = Some(Passage { t: 0, to: to.0, facing: to.1 });
    }

    /// A frame of going through the mill's door: halfway, with the screen
    /// dark, the castaway (and a cart pulled behind them) is set down on
    /// the other side and the camera with them.
    fn pass(&mut self, passage: Passage) {
        let t = passage.t + 1;
        if t == PASSAGE_FADE {
            self.player.body.set_feet(passage.to);
            self.player.facing = passage.facing;
            self.player.moving = false;
            if let Some(cart) = &mut self.cart {
                cart.hitch(passage.to, passage.facing);
            }
            self.camera.follow_within(self.player.center(), self.view_bounds());
            self.autosave.note(Cue::Moved);
        }
        self.passage = (t < 2 * PASSAGE_FADE).then_some(Passage { t, ..passage });
    }

    /// How bright the world is drawn (0 dark to 15): it fades out and back
    /// in going through the mill's door.
    pub fn brightness(&self) -> u8 {
        match self.passage {
            Some(Passage { t, .. }) if t < PASSAGE_FADE => (15 - 15 * t / PASSAGE_FADE) as u8,
            Some(Passage { t, .. }) => (15 * (t - PASSAGE_FADE) / PASSAGE_FADE) as u8,
            None => 15,
        }
    }

    /// Where the castaway's sprite would be (top-left, world px) standing
    /// beside the bed, as they do on getting out of it.
    fn bedside_px(&self) -> (i32, i32) {
        let mut body = self.player.body.clone();
        let (x, y) = self.map.bedside();
        body.set_feet((x * TILE_SIZE + TILE_SIZE / 2, y * TILE_SIZE + TILE_SIZE / 2));
        body.px()
    }

    /// Whether the castaway is in the mill's room.
    pub fn is_in_mill_room(&self) -> bool {
        self.map.in_mill_room(self.player.body.feet())
    }

    /// Where the camera keeps to (world px): the mill's room while inside,
    /// else the island.
    fn view_bounds(&self) -> Rect {
        if self.is_in_mill_room() {
            // The dark above the room too, for the HUD over the top.
            let (x0, y0, x1, y1) = self.map.mill_room();
            let top = y0 - crate::world::map::MILL_ROOM_ABOVE;
            Rect::new(x0 * TILE_SIZE, top * TILE_SIZE, (x1 - x0 + 1) * TILE_SIZE, (y1 - top + 1) * TILE_SIZE)
        } else {
            let (w, h) = self.map.island_pixel_size();
            Rect::new(0, 0, w, h)
        }
    }

    /// The mill's next stage is up, off the mill site's bank `site`.
    fn built(&mut self, site: (i32, i32)) {
        // Something made for the built mill: it stands in its place.
        if let Some(f) = self.fitting.take() {
            self.works.furnish(f);
            let changed = self.map.furnish(f);
            self.tileset.refresh(&self.map, &mut self.assets, &mut self.visual, &changed);
            return;
        }
        // Down off the roof, to where they stood.
        if let Some(feet) = self.roofing_from.take() {
            self.player.body.set_feet(feet);
        }
        if !self.works.all_in() {
            return;
        }
        let stage = self.works.built;
        self.works.finish();
        let changed = match stage {
            0 => self.map.lay_foundation(site),
            1 => self.map.raise_frame(site),
            2 => self.map.raise_walls(site),
            3 => self.map.put_on_roof(site),
            _ => Vec::new(),
        };
        self.tileset.refresh(&self.map, &mut self.assets, &mut self.visual, &changed);
    }

    /// One frame of sleep: time hurries on (hunger and thirst with it, if
    /// more slowly) and sleep mends health (faster under a shelter; see
    /// `Vitals`). The castaway sleeps on until woken with a button.
    fn sleep_on(&mut self) {
        let rest = if self.player.in_bed {
            Rest::Bedded
        } else if self.player.sheltered || self.is_in_mill_room() {
            Rest::Sheltered
        } else {
            Rest::Asleep
        };
        for _ in 0..SLEEP_MINUTES_PER_FRAME {
            self.clock.advance_minutes(1);
            self.vitals.on_minute(rest);
            if self.vitals.collapsed() {
                return;
            }
        }
    }

    /// Health spent: the castaway falls where they are, and the game ends.
    fn collapse(&mut self) {
        if let Some(line) = &mut self.line {
            line.let_go(&mut self.fish);
        }
        self.line = None;
        self.player.collapse();
        self.end = Some((Ending::Collapsed, 0));
        // There's no going back: the saved game goes with them.
        self.autosave.forget();
        self.erase_save();
    }

    /// The smoke was seen: a passing plane turns for the island, and the
    /// game ends: the world fades out, the cuts play, then the records.
    fn rescue(&mut self) {
        if let Some(line) = &mut self.line {
            line.let_go(&mut self.fish);
        }
        self.line = None;
        self.end = Some((Ending::Rescued, 0));
        // It's over: the saved game goes, as when collapsing.
        self.autosave.forget();
        self.erase_save();
    }

    /// What the end tells of the game: the day it ended, how long the
    /// castaway lasted, the kinds of things made and the recipes found.
    pub fn records(&self) -> Records {
        let book = &self.inventory.recipes;
        Records {
            day: self.clock.day,
            lasted: self.clock.total_minutes() - GameClock::new(1, START_HOUR).total_minutes(),
            made: book.kinds_made(),
            makeable: crate::crafting::makeable_kinds(),
            recipes_known: book.known_count(),
            recipes: RECIPES.len(),
        }
    }

    /// Starts a new game on the same island.
    fn start_over(&mut self) {
        // Keep the PPU (its frame buffer stays where the page reads it, and
        // the sprite limit as set) and the buttons as they are held (so the
        // START that began again doesn't count as pressed once more).
        let ppu = std::mem::take(&mut self.ppu);
        let input = std::mem::take(&mut self.input);
        *self = Game::new(self.seed);
        self.ppu = ppu;
        self.input = input;
    }

    /// Whether the castaway stands by a burning campfire.
    fn near_fire(&self) -> bool {
        self.nearest_fire().is_some()
    }

    /// Whether the castaway is by the wood lathe (it stands beside the
    /// water wheel), where things are turned on it.
    pub fn is_near_lathe(&self) -> bool {
        let Some((lx, ly)) = self.map.lathe_middle() else { return false };
        let (fx, fy) = self.player.body.feet();
        (fx - lx).pow(2) + (fy - ly).pow(2) <= NEAR_LATHE * NEAR_LATHE
    }

    /// Where the fire the castaway stands by is (a campfire, or the
    /// burning hearth in the mill), if any: the middle of it.
    fn nearest_fire(&self) -> Option<(i32, i32)> {
        let (fx, fy) = self.player.body.feet();
        let hearth = self.hearth_fire().filter(|&(x, y)| (x - fx).pow(2) + (y - fy).pow(2) <= NEAR_FIRE * NEAR_FIRE);
        self.fires.iter().filter(|f| f.within((fx, fy), NEAR_FIRE)).map(|f| (f.x, f.y)).chain(hearth).min_by_key(|&(x, y)| (x - fx).pow(2) + (y - fy).pow(2))
    }

    /// Where the hearth's fire is in the mill (world px, the middle of
    /// it), while it burns.
    fn hearth_fire(&self) -> Option<(i32, i32)> {
        if !self.works.hearth_burning(self.clock.total_minutes()) {
            return None;
        }
        let (x, y) = self.map.furnishing_cells(Furnishing::Hearth)[0];
        let (dx, dy) = crate::render::tileset::HEARTH_FIRE;
        Some((x * TILE_SIZE + dx, y * TILE_SIZE + dy))
    }

    /// A piece of fuel (a pine cone first, else a branch) is laid on the
    /// hearth: it burns a day more, catching from the embers in its ash
    /// if it had gone out.
    fn stoke_hearth(&mut self) {
        let now = self.clock.total_minutes();
        if !self.works.hearth_takes_fuel(now) || !fuel(&self.inventory.bag).is_some_and(|kind| self.inventory.bag.take_kind(kind)) {
            return;
        }
        self.inventory.settle_hand();
        self.works.stoke_hearth(now);
        if let Some(at) = self.hearth_fire() {
            self.particles.sparks((at.0, at.1 - 6), 3);
        }
    }

    /// Takes a recipe's materials and kneels to cook it over the nearest
    /// fire; it goes in the bag when done.
    fn start_cooking(&mut self, recipe: &Recipe) {
        let Some(at) = self.nearest_fire() else {
            return;
        };
        self.menu = None;
        self.inventory.use_up(recipe);
        self.player.cook(recipe.needs[0].0, recipe.makes, at);
    }

    /// Where a fire would be lit (in front of the castaway's feet), or why
    /// one can't be: no fuel, or no bare dry ground there.
    fn fire_spot(&self) -> Result<(i32, i32), &'static str> {
        if fuel(&self.inventory.bag).is_none() {
            return Err("NOTHING TO BURN");
        }
        let (x, y) = self.player.fire_spot();
        let dry = self.map.terrain(x.div_euclid(TILE_SIZE), y.div_euclid(TILE_SIZE)).is_land();
        if self.player.wading || !dry || self.map.props_at_px(x, y).solid {
            return Err("NO DRY GROUND HERE");
        }
        if self.fires.iter().any(|f| f.within((x, y), FIRE_SPACING)) {
            return Err("A FIRE IS ALREADY HERE");
        }
        Ok((x, y))
    }

    /// The drill has caught: a pine cone (or, without one, a branch) goes
    /// on as fuel and the fire burns for a day.
    fn light_fire(&mut self, spot: (i32, i32)) {
        if !fuel(&self.inventory.bag).is_some_and(|kind| self.inventory.bag.take_kind(kind)) {
            return;
        }
        self.fires.push(Campfire::new(spot, self.clock.total_minutes()));
        self.particles.sparks((spot.0, spot.1 - 6), 3);
    }

    /// Leaves an item from the bag on the ground in front of the castaway
    /// (water is poured away).
    fn put_down(&mut self, kind: ItemKind, count: u32) {
        if !kind.can_put_down() {
            return;
        }
        let (fx, fy) = self.player.body.feet();
        let (mut x, mut y) = self.player.front_of_feet();
        if !self.map.terrain(x.div_euclid(TILE_SIZE), y.div_euclid(TILE_SIZE)).is_land() || self.map.props_at_px(x, y).solid {
            (x, y) = (fx, fy + 1);
        }
        self.items.push(Item::falling(kind, (x as f32, y as f32 - 10.0), (x as f32, y as f32), (0.0, 0.0)).with_count(count));
    }

    /// Something taken from the bag has been eaten or drunk: it fills the
    /// gauges (and may heal), and whatever held it goes back in the bag.
    fn consumed(&mut self, kind: ItemKind) {
        let (fill, quench) = kind.nourishment();
        self.vitals.eat(fill);
        self.vitals.drink(quench);
        self.vitals.heal(kind.heals());
        if let Some((ailment, chance)) = kind.risk() {
            // The very first draw of raw water is always safe.
            let spared = kind == ItemKind::RawWater && !self.vitals.drunk_raw_water;
            if kind == ItemKind::RawWater {
                self.vitals.drunk_raw_water = true;
            }
            let unlucky = self.stomach_luck.map_or_else(|| self.rng.chance(chance), |lucky| !lucky);
            if !spared && unlucky {
                self.vitals.catch(ailment);
            }
        }
        if let Some(left) = kind.leaves_behind() {
            self.take(left, 1);
        }
    }

    /// Sways the palm at `anchor` for `frames`, settling any other tree
    /// that was still swaying.
    fn start_sway(&mut self, anchor: (i32, i32), frames: u32) {
        if let Some(old) = self.tree_shake.filter(|s| s.anchor != anchor) {
            sway_palm(&self.map, &self.assets.props, &mut self.visual, old.anchor, 0);
        }
        self.tree_shake = Some(TreeShake { anchor, frames });
    }

    /// Wakes the chunks around the screen (and lets those left behind
    /// sleep), catching each one woken up on the time it slept.
    fn wake_regions(&mut self) {
        let view = Rect::new(self.camera.x, self.camera.y, SCREEN_W as i32, SCREEN_H as i32);
        for (chunk, slept) in self.regions.update(view, self.clock.total_minutes()) {
            self.catch_up(chunk, slept);
        }
    }

    /// Where a chunk waking up will make up for the `slept` game minutes it
    /// was left alone (crabs breeding or dying off there, say). Nothing
    /// changes while asleep yet: things simply carry on from where they
    /// were.
    fn catch_up(&mut self, _chunk: ChunkId, _slept: u32) {}

    /// Keeps the ground around the screen built (see `Tileset::stream`).
    fn stream_ground(&mut self) {
        let view = Rect::new(self.camera.x, self.camera.y, SCREEN_W as i32, SCREEN_H as i32);
        self.tileset.stream(&self.map, &mut self.assets, &mut self.visual, view);
    }

    /// Sets up PPU state for this frame and renders it to the RGBA buffer.
    pub fn render(&mut self) {
        self.stream_ground();
        if let Some(shake) = self.tree_shake {
            // Tile animation on the struck tree only (a BG2 line scroll would
            // also move every other tree on the same lines).
            let dx = TREE_SHAKE[(shake.frames / 2) as usize % TREE_SHAKE.len()];
            sway_palm(&self.map, &self.assets.props, &mut self.visual, shake.anchor, dx);
        }
        let lighting = Lighting::at(self.clock.minute_f());
        let night = lighting.darkness > NIGHT_THRESHOLD;
        let inside = self.map.in_mill_room(self.player.body.feet());
        // Once the sun is going down, the lamps in the mill are lit: in
        // the room it's as light as by day.
        let lamps_lit = self.works.has(Furnishing::Lamps) && lighting.darkness > 0.0;
        let lamplit = inside && lamps_lit;
        let hearth_fire = self.hearth_fire();
        let hearth_dying = !self.works.hearth_burning(self.clock.total_minutes() + HEARTH_DYING_MINUTES);
        let lamps = self.works.has(Furnishing::Lamps).then(|| self.map.furnishing_cells(Furnishing::Lamps));
        // The sign that the game was saved plays through its pictures.
        let saved_sign = (self.saved_sign > 0).then(|| ((SAVED_SIGN_FRAMES - self.saved_sign) / SAVED_SIGN_HOLD) as usize);
        let mut camera = self.camera;
        if self.camera_shake > 0 {
            camera.y += if self.camera_shake % 4 < 2 { 1 } else { -1 };
        }
        let records = self.records();
        if let Some((Ending::Rescued, t)) = self.end
            && ending::view_at(t) != EndingView::FadingOut
        {
            self.render_rescue(t, &records);
            return;
        }
        let brightness = self.brightness();
        let f = &mut self.frame;
        f.clock = self.frame_count;
        f.scroll_x = camera.x;
        f.scroll_y = camera.y;
        f.brightness = brightness;
        if let Some((ending, t)) = self.end {
            if t >= COLLAPSE_FADE {
                // The end screen, fading in on black.
                f.cgram.clone_from(&self.base_cgram);
                f.cgram.colors[0] = crate::ppu::palette::Rgb555::BLACK;
                f.brightness = ((t - COLLAPSE_FADE) / 2).min(15) as u8;
                f.lines.clear();
                f.math = ColorMath::OFF;
                f.main_layers = layer::BG3;
                f.bg3_priority = false;
                let prompt = t >= COLLAPSE_FADE + COLLAPSE_HOLD && (self.frame_count / 30).is_multiple_of(2);
                // (Rescued, the cuts and records are drawn by `render_rescue`.)
                debug_assert_eq!(ending, Ending::Collapsed);
                end_screen::draw(&mut f.bg3, &self.assets.hud, &self.clock, records.lasted, prompt);
                f.oam.clear();
                self.ppu.render(&self.assets.vram, &self.frame, &self.visual.ground, &self.visual.overhead);
                return;
            }
            // The world fades out (around the fallen castaway).
            f.brightness = (15 - 15 * t / COLLAPSE_FADE) as u8;
        }

        // CGRAM: base palette → water cycling → time-of-day tint.
        f.cgram.clone_from(&self.base_cgram);
        lighting::cycle_water(&mut f.cgram, self.frame_count);
        if lamplit {
            Lighting::LAMPLIT.apply(&mut f.cgram);
        } else {
            lighting.apply(&mut f.cgram);
        }
        if inside {
            // The dark round the mill's room.
            f.cgram.colors[0] = crate::ppu::palette::Rgb555::BLACK;
        }

        if let Some(menu) = &self.menu {
            // The bag screen: world layers off, BG3 behind the icon sprites.
            f.cgram.clone_from(&self.base_cgram);
            f.lines.clear();
            f.math = ColorMath::OFF;
            f.main_layers = layer::BG3 | layer::OBJ;
            f.bg3_priority = false;
            hud::draw(&mut f.bg3, &self.assets.hud, &self.vitals, &self.clock, night, saved_sign);
            let list = &mut self.draw_list;
            list.clear();
            bag_screen::draw(&mut f.bg3, list, &self.assets.hud, &self.assets.sprites, &self.inventory, menu, self.frame_count);
            f.oam.clear();
            list.flush(&mut f.oam);
            self.ppu.render(&self.assets.vram, &self.frame, &self.visual.ground, &self.visual.overhead);
            return;
        }
        f.main_layers = layer::BG1 | layer::BG2 | layer::BG3 | layer::OBJ;
        f.bg3_priority = true;
        bag_screen::clear(&mut f.bg3);

        // Colour math + HDMA tables.
        if night && !lamplit {
            // Every light: a torch carried, each campfire and the hearth.
            let torch = self.inventory.torch_lit().then(|| self.player.center());
            let hearth = hearth_fire.map(|(x, y)| (x, y - 8));
            let lights: Vec<(i32, i32)> = torch.into_iter().chain(self.fires.iter().map(|f| f.flame_point())).chain(hearth).collect();
            let at = (camera.x, camera.y);
            let assets = &mut self.assets;
            let lit = light_canvas::lay(&mut f.bg3, &mut assets.vram.hud_tiles, &assets.hud, at, &lights);
            let t = ((lighting.darkness - NIGHT_THRESHOLD) / (1.0 - NIGHT_THRESHOLD)).clamp(0.0, 1.0);
            let strength = t * t * (3.0 - 2.0 * t);
            lighting::night(&mut f.lines, &mut f.cgram, lit.then(|| light_canvas::scroll(at)), HUD_LINES, strength, self.frame_count);
            f.math = MATH_NIGHT;
        } else {
            f.lines.clear();
            f.math = MATH_TRANSLUCENT;
        }

        match self.title {
            Some(title) => {
                let items = title_items(self.saved.is_some());
                title_screen::draw(&mut f.bg3, &self.assets.hud, &items, title.choice, title.confirm, !(self.frame_count / 20).is_multiple_of(3))
            }
            None => hud::draw(&mut f.bg3, &self.assets.hud, &self.vitals, &self.clock, night, saved_sign),
        }

        // Sprites.
        let shadows = !night;
        let list = &mut self.draw_list;
        list.clear();
        let sprites = &self.assets.sprites;
        // Up on the mill's roof, they're over everything.
        let in_front =
            player_in_front_of_canopy(&self.map, &self.player) || feet_before_mill_wall(&self.map, self.player.body.feet()) || self.roofing_from.is_some();
        // Feet hidden behind a wall of the mill cast no shadow over it.
        let feet = self.player.body.feet();
        let player_shadow = shadows && !feet_behind_mill_wall(&self.map, feet) && !feet_under_mill_roof(&self.map, feet);
        self.player.draw(list, &camera, sprites, self.frame_count, player_shadow, in_front);
        let now = self.clock.total_minutes();
        for fire in &self.fires {
            fire.draw(list, &camera, sprites, self.frame_count, now);
        }
        for fire in &self.signal_fires {
            fire.draw(list, &camera, sprites, self.frame_count);
        }
        if let Some(at) = hearth_fire {
            draw_hearth_fire(list, &camera, sprites, at, hearth_dying, self.frame_count);
        }
        if let Some(cells) = &lamps {
            draw_lamps(list, &camera, sprites, cells, lamps_lit, self.frame_count);
        }
        draw_mill_posts(list, &camera, sprites, &self.map, night);
        for crab in &self.crabs {
            crab.draw(list, &camera, sprites, night);
        }
        for fish in &self.fish {
            fish.draw(list, &camera, sprites, night);
        }
        if let Some(line) = &self.line {
            draw_line(list, &camera, sprites, line, self.player.rod_tip(sprites), self.player.depth(), self.frame_count);
        }
        for item in &self.items {
            item.draw(list, &camera, sprites, night);
        }
        if let Some(cart) = &self.cart {
            // Pulled, the handle crosses in front of the castaway.
            let handle_depth = cart.pulled.then(|| self.player.depth() + 1);
            draw_cart(list, &camera, sprites, cart, night, handle_depth);
        }
        self.particles.draw(list, &camera, sprites, self.frame_count);
        if !night && !inside {
            for gull in &self.gulls {
                gull.draw(list, &camera, sprites, shadows);
            }
        }
        f.oam.clear();
        list.flush(&mut f.oam);

        self.ppu.render(&self.assets.vram, &self.frame, &self.visual.ground, &self.visual.overhead);
    }

    /// Dev tools: draws the world alone (no HUD, castaway or creatures;
    /// items on the ground are kept) in daylight, with the screen's top-left
    /// at world px `at`, for maps of the island. Leaves the game's own
    /// camera alone.
    pub fn debug_render_view(&mut self, at: (i32, i32)) {
        let view = Rect::new(at.0, at.1, SCREEN_W as i32, SCREEN_H as i32);
        self.tileset.stream(&self.map, &mut self.assets, &mut self.visual, view);
        let camera = Camera { x: at.0, y: at.1, ..self.camera };
        let f = &mut self.frame;
        f.clock = 0;
        f.scroll_x = camera.x;
        f.scroll_y = camera.y;
        f.brightness = 15;
        f.cgram.clone_from(&self.base_cgram);
        f.lines.clear();
        f.math = MATH_TRANSLUCENT;
        f.main_layers = layer::BG1 | layer::BG2 | layer::OBJ;
        f.bg3_priority = false;
        let list = &mut self.draw_list;
        list.clear();
        for item in &self.items {
            item.draw(list, &camera, &self.assets.sprites, false);
        }
        f.oam.clear();
        list.flush(&mut f.oam);
        self.ppu.render(&self.assets.vram, &self.frame, &self.visual.ground, &self.visual.overhead);
    }

    /// Dev tools: the island's map, as the game holds it.
    pub fn map(&self) -> &WorldMap {
        &self.map
    }

    /// Dev tools: lays `days` pieces of fuel on the mill's hearth (no more
    /// than it takes), if it's made. Whether it was.
    pub fn debug_stoke_hearth(&mut self, days: u32) -> bool {
        if !self.works.has(Furnishing::Hearth) {
            return false;
        }
        let now = self.clock.total_minutes();
        for _ in 0..days {
            if self.works.hearth_takes_fuel(now) {
                self.works.stoke_hearth(now);
            }
        }
        true
    }

    /// Dev tools: lays a log bridge across the river's narrow (as if one
    /// were laid there), if it goes there. Whether it was laid.
    pub fn debug_lay_bridge(&mut self) -> bool {
        let Some(span) = self.map.spots.bridge.and_then(|(x, y)| self.map.bridge_spot(x, y)) else {
            return false;
        };
        let changed = self.map.lay_bridge(span);
        self.tileset.refresh(&self.map, &mut self.assets, &mut self.visual, &changed);
        true
    }

    /// Dev tools: sets a water wheel in the river at the mill site (none
    /// needed in the bag). Returns whether one went in.
    pub fn debug_set_wheel(&mut self) -> bool {
        let Some(site) = self.map.spots.mill.filter(|&(x, y)| self.map.wheel_spot(x + 1, y).is_some()) else {
            return false;
        };
        let changed = self.map.set_wheel(site);
        self.tileset.refresh(&self.map, &mut self.assets, &mut self.visual, &changed);
        true
    }

    /// Dev tools: sets a wood lathe beside the water wheel at the mill site
    /// (none needed in the bag; the wheel must be in). Returns whether one
    /// went in.
    pub fn debug_set_lathe(&mut self) -> bool {
        let Some(site) = self.map.spots.mill.filter(|&(x, y)| self.map.lathe_spot(x, y).is_some()) else {
            return false;
        };
        let changed = self.map.set_lathe(site);
        self.tileset.refresh(&self.map, &mut self.assets, &mut self.visual, &changed);
        true
    }

    /// Dev tools: marks out the mill's ground beside the wood lathe (no
    /// stakes needed in the bag; the lathe must be in). Returns whether it
    /// was marked out.
    pub fn debug_set_stakes(&mut self) -> bool {
        let Some(site) = self.map.spots.mill.filter(|&(x, y)| self.map.stakes_spot(x, y).is_some()) else {
            return false;
        };
        let changed = self.map.set_stakes(site);
        self.tileset.refresh(&self.map, &mut self.assets, &mut self.visual, &changed);
        self.step_clear();
        true
    }

    /// Dev tools: fits the built mill out with `f` straight away (no
    /// materials, no work). False if the mill isn't built yet.
    pub fn debug_furnish(&mut self, f: Furnishing) -> bool {
        if self.works.next().is_some() {
            return false;
        }
        self.works.furnish(f);
        self.inventory.look_at_furnishing(f);
        let changed = self.map.furnish(f);
        self.tileset.refresh(&self.map, &mut self.assets, &mut self.visual, &changed);
        true
    }

    /// Dev tools: puts everything in the bag on the mill's shelf.
    pub fn debug_shelve_bag(&mut self) {
        while !self.inventory.bag.items().is_empty() {
            let p = self.inventory.bag.remove(0);
            self.inventory.shelf.put(p.kind, p.count);
        }
        self.inventory.settle_hand();
    }

    /// What the mill has been fitted out with.
    pub fn furnished(&self) -> &[Furnishing] {
        &self.works.furnished
    }

    /// What's on the mill's shelf.
    pub fn shelf(&self) -> &shelf::Shelf {
        &self.inventory.shelf
    }

    /// Dev tools: builds the mill's stages until `stages` are up (nothing
    /// needed at the site or in the bag; its ground must be marked out).
    /// Returns whether as many are up.
    pub fn debug_build_mill(&mut self, stages: usize) -> bool {
        let Some(site) = self.map.spots.mill.filter(|_| self.map.is_staked() || self.works.built > 0) else { return false };
        while self.works.built < stages {
            let Some(stage) = self.works.next() else { return false };
            for &(kind, n) in stage.needs {
                self.works.deliver(kind, n);
            }
            self.built(site);
        }
        true
    }

    /// Dev tools: the steps the mill goes up in, as the debug tools set it
    /// up one at a time: the wheel, the lathe, the stakes, then each stage
    /// (by its name).
    pub fn debug_mill_steps(&self) -> Vec<&'static str> {
        ["WHEEL", "LATHE", "STAKES"].into_iter().chain(mill::MILL_STAGES.iter().map(|s| s.name)).collect()
    }

    /// Dev tools: how many of `debug_mill_steps` are done.
    pub fn debug_mill_progress(&self) -> usize {
        if !self.map.has_wheel() {
            0
        } else if self.map.lathe_middle().is_none() {
            1
        } else if !(self.map.is_staked() || self.works.built > 0) {
            2
        } else {
            3 + self.works.built
        }
    }

    /// Dev tools: sets the mill up one step further (`debug_mill_steps`).
    /// Returns whether there was a step left to take.
    pub fn debug_mill_advance(&mut self) -> bool {
        match self.debug_mill_progress() {
            0 => self.debug_set_wheel(),
            1 => self.debug_set_lathe(),
            2 => self.debug_set_stakes(),
            n if n - 3 < mill::MILL_STAGES.len() => self.debug_build_mill(n - 2),
            _ => false,
        }
    }

    /// Dev tools: hands over everything the mill's next stage takes (its
    /// materials, less what's been left at the site, and the tools not
    /// carried): into the bag, and what doesn't fit into the cart, if
    /// there is one. Returns how many didn't fit anywhere.
    pub fn debug_give_mill_stage(&mut self) -> u32 {
        let Some(stage) = self.works.next() else { return 0 };
        let needs: Vec<(ItemKind, u32)> = stage.needs.iter().map(|&(k, _)| (k, self.works.still_needs(k))).collect();
        self.debug_hand_over(&needs, stage.tools)
    }

    /// Dev tools: hands over everything the `index`th of `debug_makings`
    /// takes: its materials, topped up to what it takes, and the tools not
    /// carried (into the bag, then the cart). Returns how many didn't fit
    /// anywhere.
    pub fn debug_give_makings(&mut self, index: usize) -> u32 {
        let (needs, tools) = match debug_makings().get(index) {
            Some(&Making::Recipe(i)) => (RECIPES[i].needs, RECIPES[i].tools),
            Some(&Making::Furnishing(f)) => (f.needs(), f.tools()),
            None => return 0,
        };
        let needs: Vec<(ItemKind, u32)> = needs.iter().map(|&(k, n)| (k, n.saturating_sub(self.inventory.bag.count(k)))).collect();
        self.debug_hand_over(&needs, tools)
    }

    /// Gives `needs` and each of `tools` not carried, into the bag and then
    /// the cart; how many didn't fit anywhere.
    fn debug_hand_over(&mut self, needs: &[(ItemKind, u32)], tools: &[ItemKind]) -> u32 {
        let tools = tools.iter().filter(|&&t| !self.inventory.carries(t)).map(|&t| (t, 1));
        let wanted: Vec<(ItemKind, u32)> = tools.chain(needs.iter().copied()).filter(|&(_, n)| n > 0).collect();
        let mut left = 0;
        for (kind, n) in wanted {
            let rest = n - self.debug_give_n(kind, n);
            let rest = match &mut self.inventory.cart {
                Some(cart) => cart.stow(kind, rest),
                None => rest,
            };
            left += rest;
        }
        left
    }

    /// Dev tools: the most shells the tide leaves each morning (it grows
    /// with the island).
    pub fn debug_shells_per_morning(&self) -> usize {
        island::for_area(&self.map, SHELLS_PER_DAY)
    }

    /// Dev tools: how many driftwood logs lie in the first area (and
    /// elsewhere this side of the river) and on the west coast.
    pub fn debug_driftwood(&self) -> (usize, usize) {
        let west = driftwood::logs_on_west_coast(&self.map);
        let all = (0..self.map.height())
            .flat_map(|y| (0..self.map.width()).map(move |x| (x, y)))
            .filter(|&(x, y)| matches!(self.map.feature(x, y), Feature::Driftwood(p) if p.dx == 0))
            .count();
        (all - west, west)
    }

    /// Dev tools: splits every driftwood log on the island at once (the
    /// branches aren't dropped).
    pub fn debug_clear_driftwood(&mut self) {
        let mut changed = Vec::new();
        for y in 0..self.map.height() {
            for x in 0..self.map.width() {
                if matches!(self.map.feature(x, y), Feature::Driftwood(_)) {
                    self.map.set_feature(x, y, Feature::None);
                    changed.push((x, y));
                }
            }
        }
        self.tileset.refresh(&self.map, &mut self.assets, &mut self.visual, &changed);
    }

    /// Dev tools: the game as it stands, as a save (JSON).
    pub fn debug_save_json(&self) -> String {
        self.save_data().to_json()
    }

    /// The rescue's ending, past the world fading out: a cut, letterboxed
    /// in the middle of the screen and fading in and out; or the records
    /// fading in.
    fn render_rescue(&mut self, t: u32, records: &Records) {
        let view = ending::view_at(t);
        if let EndingView::Cut(i) = view {
            self.load_cut(i);
        }
        let f = &mut self.frame;
        f.clock = self.frame_count;
        f.cgram.clone_from(&self.base_cgram);
        f.cgram.colors[0] = crate::ppu::palette::Rgb555::BLACK;
        f.lines.clear();
        f.math = ColorMath::OFF;
        f.bg3_priority = false;
        f.oam.clear();
        bag_screen::clear(&mut f.bg3);
        match (view, &self.cut) {
            (EndingView::Cut(_), Some((i, scene))) => {
                let frame = ending::cut_frame(t).unwrap_or(0);
                f.brightness = ending::cut_brightness(frame);
                f.main_layers = layer::BG1 | layer::OBJ;
                f.scroll_x = 0;
                f.scroll_y = -CUT_TOP;
                for (k, colors) in scene.palettes.iter().enumerate() {
                    f.cgram.set_bg_palette(crate::render::scene::FIRST_PALETTE + k as u8, colors);
                }
                if *i == PLANE_CUT
                    && let Some(plane) = &self.plane
                {
                    // Banking toward the island (held still: the picture
                    // says it).
                    f.cgram.set_obj_palette(OBJ_CRAB, &plane.palette);
                    let (x, y) = PLANE_AT;
                    let list = &mut self.draw_list;
                    list.clear();
                    for &(gfx, dx, dy) in &plane.pieces {
                        list.actor(0, ObjEntry::new(x + dx, y + dy, gfx, OBJ_CRAB, 2));
                    }
                    list.flush(&mut f.oam);
                }
                self.ppu.render(&self.assets.vram, &self.frame, scene, &NoLayer);
            }
            (EndingView::Records, _) => {
                let since = t - ending::RECORDS_AT;
                f.brightness = (since / 2).min(15) as u8;
                f.main_layers = layer::BG3;
                let prompt = ending::can_begin_again(t) && (self.frame_count / 30).is_multiple_of(2);
                end_screen::draw_escape(&mut f.bg3, &self.assets.hud, records, prompt);
                self.ppu.render(&self.assets.vram, &self.frame, &NoLayer, &NoLayer);
            }
            _ => {
                f.brightness = 0;
                f.main_layers = 0;
                self.ppu.render(&self.assets.vram, &self.frame, &NoLayer, &NoLayer);
            }
        }
    }

    /// Makes the rescue's cut `i` ready to show (the one before is put
    /// away, as a new picture is loaded into VRAM in the dark between).
    fn load_cut(&mut self, i: usize) {
        if self.cut.as_ref().is_some_and(|(c, _)| *c == i) {
            return;
        }
        let vram = &mut self.assets.vram;
        if let Some((_, old)) = self.cut.take() {
            old.unload(&mut vram.metatiles);
        }
        let picture = match i {
            0 => ending_art::island_from_afar(),
            1 => ending_art::cockpit_view(),
            _ => ending_art::turning_for_the_island(),
        };
        self.cut = Some((i, Scene::load(&picture, &mut vram.metatiles)));
        if i == PLANE_CUT && self.plane.is_none() {
            self.plane = Some(SpriteSet::load(&ending_art::plane(), &mut vram.sprites));
        }
    }

    /// What the rescue's ending shows, once rescued.
    pub fn ending_view(&self) -> Option<EndingView> {
        match self.end {
            Some((Ending::Rescued, t)) => Some(ending::view_at(t)),
            _ => None,
        }
    }

    /// Whether the title screen shows.
    pub fn is_at_title(&self) -> bool {
        self.title.is_some()
    }

    /// Dev tools: the smoke is seen this moment: the castaway is rescued.
    pub fn debug_rescue_now(&mut self) {
        self.rescue();
    }

    /// Dev tools: a game carried on from a save (JSON), past the title.
    pub fn debug_continue(json: &str) -> Result<Self, save::LoadError> {
        let data = SaveData::from_json(json)?;
        let mut game = Self::build(data.seed, Some(&data));
        game.saved = Some(data);
        Ok(game)
    }

    /// Dev tools: leaves the title screen, as if START were chosen.
    pub fn skip_title(&mut self) {
        self.title = None;
    }

    /// Dev tools: puts the castaway, awake, with their feet in a cell.
    pub fn debug_place_player(&mut self, (x, y): (i32, i32)) {
        self.skip_title();
        self.player = Player::at_feet(x * TILE_SIZE, y * TILE_SIZE);
        self.camera.follow_within(self.player.center(), self.view_bounds());
    }

    /// Dev tools: puts the castaway, awake, with their feet on a world
    /// pixel.
    pub fn debug_place_player_px(&mut self, (x, y): (i32, i32)) {
        self.skip_title();
        self.player = Player::at_feet(x, y);
        let (fx, fy) = self.player.body.feet();
        self.player = Player::at_feet(2 * x - fx, 2 * y - fy);
        self.camera.follow_within(self.player.center(), self.view_bounds());
    }

    /// Whether the castaway can stand on the cell.
    fn standable(&self, (x, y): (i32, i32)) -> bool {
        self.map.in_bounds(x, y) && !self.map.props(x, y).solid && !matches!(self.map.terrain(x, y), Terrain::DeepWater | Terrain::River)
    }

    /// Debug: moves the castaway to the cell, or the nearest one they can
    /// stand on; keeps which way they face and their torch, and reels in
    /// any line. Returns the cell moved to, if any was found.
    pub fn debug_move_to(&mut self, (x, y): (i32, i32)) -> Option<(i32, i32)> {
        let to = (0..=24).find_map(|r: i32| {
            (-r..=r)
                .flat_map(|dy| (-r..=r).map(move |dx| (dx, dy)))
                .filter(|&(dx, dy)| dx.abs().max(dy.abs()) == r)
                .map(|(dx, dy)| (x + dx, y + dy))
                .filter(|&c| self.standable(c))
                .min_by_key(|&(cx, cy)| (cx - x).pow(2) + (cy - y).pow(2))
        })?;
        let (facing, torch) = (self.player.facing, self.player.torch);
        self.line = None;
        self.player = Player::at_feet(to.0 * TILE_SIZE, to.1 * TILE_SIZE);
        self.player.facing = facing;
        self.player.torch = torch;
        self.camera.follow_within(self.player.center(), self.view_bounds());
        Some(to)
    }

    /// Debug: places worth going to, by name, as cells (`debug_move_to`
    /// finds ground to stand on near each).
    pub fn debug_places(&self) -> Vec<(&'static str, (i32, i32))> {
        let cell = |(x, y): (i32, i32)| (x.div_euclid(TILE_SIZE), y.div_euclid(TILE_SIZE));
        let map = &self.map;
        let mut v = vec![("漂着地点", cell(map.spawn))];
        for &w in &map.water_spots {
            let (x, y) = cell(w);
            let name = if map.is_high(x, y) {
                "崖の上の水汲み場"
            } else if map.spots.falls.is_some_and(|(fx, _)| x < fx) {
                "滝壺の西の水汲み場"
            } else {
                "滝壺の東の水汲み場"
            };
            v.push((name, (x, y)));
        }
        if let Some(b) = map.spots.bridge {
            v.push(("川の狭い所", b));
        }
        if let Some(m) = map.spots.mill {
            v.push(("水車小屋の予定地", m));
            let (bx, by) = crate::world::map::site_bench(m);
            v.push(("水車小屋の普請台", (bx - 1, by)));
        }
        if let Some(l) = map.spots.ladder {
            v.push(("崖のくぼみ（ハシゴ）", l));
            // North from the recess across the high ground to the lake.
            let shore = (0..l.1).rev().map(|y| (l.0, y)).take_while(|&(x, y)| !map.terrain(x, y).is_water()).last();
            if let Some(s) = shore {
                v.push(("湖の南岸", s));
            }
        }
        if let Some(s) = map.spots.slope {
            v.push(("西の草の坂", s));
        }
        // The high ground's east end, where the ridge closes it off, and the
        // cape below it.
        let east_end = |y: i32| (0..map.width()).rev().find(|&x| map.is_high(x, y) && map.terrain(x, y) == Terrain::Highland).map(|x| (x, y));
        if let Some(c) = east_end(map.height() / 2 + 20) {
            v.push(("東の尾根の前", c));
        }
        if let Some(c) = (0..map.height()).rev().find_map(east_end) {
            v.push(("東の岬", c));
        }
        v
    }

    /// Debug: cycles outfit presets until equipment exists.
    pub fn debug_cycle_outfit(&mut self) {
        let presets = Outfit::PRESETS;
        let i = presets.iter().position(|o| *o == self.inventory.outfit).unwrap_or(0);
        self.equip(presets[(i + 1) % presets.len()]);
    }

    /// Dev tools: sets the castaway's health (of a 64-point gauge).
    pub fn debug_set_hp(&mut self, hp: u32) {
        self.vitals.hp = hp.min(self.vitals.max_hp);
    }

    /// Dev tools: sets health, food and water (each left as it is when
    /// `None`).
    pub fn debug_set_vitals(&mut self, hp: Option<u32>, food: Option<u32>, water: Option<u32>) {
        if let Some(hp) = hp {
            self.debug_set_hp(hp);
        }
        if let Some(food) = food {
            self.vitals.food = food.min(Vitals::MAX);
        }
        if let Some(water) = water {
            self.vitals.water = water.min(Vitals::MAX);
        }
    }

    /// Dev tools: makes something raw always (`Some(false)`) or never
    /// (`Some(true)`) upset the stomach; `None` leaves it to chance.
    pub fn debug_stomach_luck(&mut self, lucky: Option<bool>) {
        self.stomach_luck = lucky;
    }

    /// Dev tools: makes a signal fire's smoke always (`Some(true)`) or
    /// never (`Some(false)`) seen; `None` leaves it to chance.
    pub fn debug_rescue_luck(&mut self, lucky: Option<bool>) {
        self.rescue_luck = lucky;
    }

    /// Dev tools: makes rescue sure. A signal fire lit from now on is seen,
    /// an hour after it's lit, even if today's chance has been had; one
    /// already burning unseen is seen an hour from now.
    pub fn debug_sure_rescue(&mut self) {
        self.rescue_luck = Some(true);
        self.signal_day = None;
        if self.rescue_at.is_none() && self.signal_fires.iter().any(|f| f.burning()) {
            self.rescue_at = Some(self.clock.total_minutes() + SEEN_FROM);
        }
    }

    /// How many signal fires are burning.
    pub fn burning_signal_fires(&self) -> usize {
        self.signal_fires.iter().filter(|f| f.burning()).count()
    }

    /// How many signal fires are sending up black smoke (fish oil poured
    /// on them).
    pub fn black_smoking_signal_fires(&self) -> usize {
        self.signal_fires.iter().filter(|f| f.burning() && f.black).count()
    }

    /// Whether the castaway has been rescued (the game has ended so).
    pub fn is_rescued(&self) -> bool {
        matches!(self.end, Some((Ending::Rescued, _)))
    }

    /// Whether the castaway stands by a campfire (things can be cooked).
    pub fn is_by_a_fire(&self) -> bool {
        self.near_fire()
    }

    /// Dev tools: upsets the castaway's stomach, as if from something raw.
    pub fn debug_upset_stomach(&mut self) {
        self.vitals.catch(Ailment::UpsetStomach);
    }

    /// Dev tools: gets the castaway over whatever ails them.
    pub fn debug_cure(&mut self) {
        self.vitals.cure();
    }

    /// Dev tools: sets a crab down `dx` px to the right of the castaway's
    /// feet.
    pub fn debug_crab_ahead(&mut self, dx: i32) {
        let (fx, fy) = self.player.body.feet();
        if let Some(crab) = self.crabs.first_mut() {
            *crab = Crab::new(fx + dx - 8, fy - 11, 7);
        }
    }

    /// Dev tools: puts an item (a full stack of stackables) straight into
    /// the bag, if it fits.
    pub fn debug_advance_hours(&mut self, hours: u32) {
        self.clock.advance_minutes(hours * 60);
    }

    /// Dev tools: puts a fish in the water this far from the castaway's
    /// feet.
    pub fn debug_fish_at(&mut self, (dx, dy): (i32, i32)) {
        let (fx, fy) = self.player.body.feet();
        self.fish.push(Fish::new((fx + dx, fy + dy), 7));
    }

    /// Debug: how many fish swim anywhere, and in the lake.
    pub fn debug_fish_count(&self) -> usize {
        self.fish.len()
    }

    pub fn debug_fish_in_lake(&self) -> usize {
        self.fish_in_lake()
    }

    /// Debug: catches a fish in the lake as a line would (it comes back
    /// in time); false if there's none there.
    pub fn debug_catch_fish_in_lake(&mut self) -> bool {
        let Some(i) = self.fish.iter().position(|f| self.in_lake(f.pos())) else { return false };
        self.fish.swap_remove(i);
        self.fish_due.push(self.clock.total_minutes() + FISH_RETURN_MINUTES);
        true
    }

    /// Debug: knows every recipe.
    pub fn debug_learn_recipes(&mut self) {
        self.inventory.recipes.learn_all();
    }

    /// Debug: puts on the gear of `kind` carried in the bag (as choosing
    /// it for its slot on the gear page would); false if it couldn't be.
    pub fn debug_wear(&mut self, kind: ItemKind) -> bool {
        let Some(i) = self.inventory.bag.items().iter().position(|p| p.kind == kind) else { return false };
        self.inventory.wear(i).is_ok()
    }

    /// Debug: hangs a tool of `kind` from the bag on the belt's first free
    /// hook; false if it couldn't be.
    pub fn debug_hang(&mut self, kind: ItemKind) -> bool {
        let Some(i) = self.inventory.bag.items().iter().position(|p| p.kind == kind) else { return false };
        self.inventory.hang(i).is_ok()
    }

    /// Debug: takes the tool of `kind` carried in hand (as HOLD would);
    /// false if it isn't carried or isn't held in hand.
    pub fn debug_hold(&mut self, kind: ItemKind) -> bool {
        let ok = self.inventory.carries(kind) && kind.slot() == Some(crate::items::Slot::Hand);
        if ok {
            self.inventory.gear.hand = Some(kind);
        }
        ok
    }

    pub fn debug_give(&mut self, kind: ItemKind) {
        self.debug_give_n(kind, kind.max_stack());
    }

    /// Puts up to `n` of a kind in the bag; returns how many went in.
    pub fn debug_give_n(&mut self, kind: ItemKind, n: u32) -> u32 {
        if !kind.goes_in_bag() {
            self.carry_off(kind);
            return n.min(1);
        }
        let given = n - self.inventory.bag.stow(kind, n);
        if given > 0 {
            self.inventory.took(kind);
        }
        given
    }

    pub fn rgba(&self) -> &[u8] {
        self.ppu.rgba()
    }

    pub fn set_sprite_limit(&mut self, on: bool) {
        self.ppu.sprite_limit = on;
    }

    pub fn is_paused(&self) -> bool {
        self.paused
    }

    /// Whether the castaway is standing doing nothing.
    pub fn player_is_idle(&self) -> bool {
        self.player.is_idle()
    }

    pub fn player_tile(&self) -> (i32, i32) {
        let (x, y) = self.player.body.feet();
        (x.div_euclid(crate::world::TILE_SIZE), y.div_euclid(crate::world::TILE_SIZE))
    }

    /// Which way the castaway faces.
    pub fn player_facing(&self) -> Dir {
        self.player.facing
    }

    /// Debug: whole hours on the clock since the game started (what the
    /// snapshot's `SNAP_HOURS` takes to come back to this time of day).
    pub fn debug_hours_since_start(&self) -> u32 {
        (self.clock.total_minutes() - GameClock::new(1, START_HOUR).total_minutes()) / 60
    }

    pub fn clock(&self) -> &GameClock {
        &self.clock
    }

    /// The page the bag is open at, if it's open.
    pub fn bag_page(&self) -> Option<menu::Page> {
        self.menu.as_ref().map(BagMenu::page)
    }

    /// The pages the open bag has (none if it's shut).
    pub fn bag_pages(&self) -> Vec<menu::Page> {
        self.menu.as_ref().map_or_else(Vec::new, BagMenu::pages)
    }

    /// How many of `kind` have been left at the mill's site for its next
    /// stage.
    pub fn mill_delivered(&self, kind: ItemKind) -> u32 {
        self.works.delivered(kind)
    }

    /// How many stages of the mill have been built.
    pub fn mill_stages_built(&self) -> usize {
        self.works.built
    }

    /// Whether any of `kind` is carried (in the bag or on the belt).
    pub fn carries(&self, kind: ItemKind) -> bool {
        self.inventory.carries(kind)
    }

    /// How many of `kind` the bag holds.
    pub fn carried(&self, kind: ItemKind) -> u32 {
        self.inventory.bag.count(kind)
    }

    /// Where the castaway's feet are (world px).
    pub fn player_feet(&self) -> (i32, i32) {
        self.player.body.feet()
    }

    /// Whether the castaway is wading in the shallows.
    pub fn is_wading(&self) -> bool {
        self.player.is_wading()
    }

    /// Whether the castaway is pulling the handcart.
    pub fn is_pulling_cart(&self) -> bool {
        self.cart.as_ref().is_some_and(|c| c.pulled)
    }

    /// Where the handcart stands (its middle, world px), once made.
    pub fn cart_at(&self) -> Option<(i32, i32)> {
        self.cart.as_ref().map(|c| c.at)
    }

    /// How many of `kind` the handcart is loaded with.
    pub fn cart_load(&self, kind: ItemKind) -> u32 {
        self.inventory.cart.as_ref().map_or(0, |c| c.count(kind))
    }

    /// Debug: puts up to `n` of `kind` in the handcart; returns how many
    /// went in.
    pub fn debug_load_cart(&mut self, kind: ItemKind, n: u32) -> u32 {
        self.inventory.cart.as_mut().map_or(0, |c| n - c.stow(kind, n))
    }

    /// Whether a "!" is over the castaway's head (an idea has struck).
    pub fn is_struck_by_an_idea(&self) -> bool {
        self.player.is_struck_by_an_idea()
    }

    /// Whether the castaway has stopped to think.
    pub fn is_pondering(&self) -> bool {
        self.player.is_pondering()
    }

    /// Health, hunger and thirst.
    pub fn vitals(&self) -> &Vitals {
        &self.vitals
    }

    /// VRAM needed for the current screen: the unique background tiles in
    /// view (as if streamed in while scrolling) plus all sprite and HUD tiles.
    /// Dev tools: blocks held for the map's ground (built near the screen).
    pub fn ground_blocks(&self) -> usize {
        self.tileset.resident()
    }

    pub fn vram_usage(&self) -> usize {
        let mut ids = std::collections::HashSet::new();
        let (x0, y0) = (self.camera.x.div_euclid(16), self.camera.y.div_euclid(16));
        for my in y0..=y0 + (SCREEN_H as i32 / 16) {
            for mx in x0..=x0 + (SCREEN_W as i32 / 16) {
                for layer in [&self.visual.ground as &dyn BgLayer, &self.visual.overhead] {
                    let c = layer.cell(mx, my);
                    if !c.is_empty() {
                        ids.insert(c.tile);
                    }
                }
            }
        }
        let vram = &self.assets.vram;
        ids.len() * METATILE_BYTES + vram.hud_tiles.vram_bytes() + vram.sprites.vram_bytes()
    }
}

/// Where the rescue's cuts show: a 256x128 band in the middle of the
/// screen, black above and below.
const CUT_TOP: i32 = (SCREEN_H as i32 - ending_art::CUT_H as i32) / 2;
/// The cut with the plane in it, and where its picture's top-left is on
/// the screen (its middle a little left of the cut's, over the sea).
const PLANE_CUT: usize = 2;
const PLANE_AT: (i32, i32) = (98 - ending_art::PLANE_MIDDLE.0 as i32, CUT_TOP + 50 - ending_art::PLANE_MIDDLE.1 as i32);

/// How a game ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ending {
    /// Health spent.
    Collapsed,
    /// The signal fire's smoke was seen.
    Rescued,
}

/// What the end tells of the game.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Records {
    /// The day it ended on, and the game minutes the castaway lasted.
    pub day: u32,
    pub lasted: u32,
    /// Kinds of things made, of all the kinds there are to make.
    pub made: usize,
    pub makeable: usize,
    /// Recipes thought of, of all there are.
    pub recipes_known: usize,
    pub recipes: usize,
}

/// The title's items: START, and CONTINUE below it when there is a saved
/// game.
fn title_items(saved: bool) -> Vec<TitleItem> {
    let mut items = vec![TitleItem::Start];
    if saved {
        items.push(TitleItem::Continue);
    }
    items
}

/// What the page is to do with the saved game.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SaveOp {
    /// Keep this (JSON), replacing any saved before.
    Write(String),
    Erase,
}

/// The title screen's state: the item chosen, and while asking whether to
/// begin anew over a saved game, whether YES is chosen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct TitleState {
    choice: usize,
    confirm: Option<bool>,
}

impl TitleState {
    /// As the title opens: on CONTINUE if there's a saved game.
    fn opening(saved: bool) -> Self {
        Self { choice: saved as usize, confirm: None }
    }
}

/// Something A can pick up.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Pickup {
    /// An item lying on the ground (index into the items).
    Item(usize),
    /// Something lying in a cell as a map feature: a shell on the sand, a
    /// pine cone (its cell).
    Ground((i32, i32)),
}

/// What a fire would burn: pine cones first (they're good for nothing
/// else), then branches (which make things).
fn fuel(bag: &Bag) -> Option<ItemKind> {
    [ItemKind::PineCone, ItemKind::Branch].into_iter().find(|&k| bag.has(k))
}

/// The item a feature lying on the ground gives when picked up.
fn ground_pickup(feature: Feature) -> Option<ItemKind> {
    match feature {
        Feature::Shell => Some(ItemKind::SeaShell),
        Feature::PineCone => Some(ItemKind::PineCone),
        _ => None,
    }
}

/// Canopy box (x, y, w, h) and the ground row leaves fall to, for the palm
/// whose top-left cell is `anchor`.
fn palm_canopy(anchor: (i32, i32)) -> ((i32, i32, i32, i32), i32) {
    let (x, y) = (anchor.0 * TILE_SIZE, anchor.1 * TILE_SIZE);
    ((x + 2, y + 4, 44, 32), y + 3 * TILE_SIZE + 4)
}

/// Whether the palm at `anchor` is drawn mirrored.
fn palm_flip(map: &WorldMap, anchor: (i32, i32)) -> bool {
    // Read from the trunk base, which outlives the rest of a felled tree.
    use crate::world::terrain::PalmPart;
    let (x, y) = (anchor.0 + PalmPart::W / 2, anchor.1 + PalmPart::H - 1);
    match map.feature(x, y) {
        crate::world::Feature::Palm(p) => p.flip,
        crate::world::Feature::Stump(s) => s.flip,
        _ => false,
    }
}

/// Whether the castaway stands in front of (south of) a palm or broadleaf
/// tree whose canopy or upper trunk tiles overlap their sprite, so they must be drawn above BG2.
/// Whether feet at `(fx, fy)` (world px) are behind one of the mill's
/// walls, whose face is drawn over them: just north of the north wall
/// (outside), or inside just north of the south wall (not in the
/// doorway).
fn feet_behind_mill_wall(map: &WorldMap, (fx, fy): (i32, i32)) -> bool {
    let Some(site) = map.spots.mill.filter(|_| map.has_mill_walls()) else { return false };
    let (x0, y0, x1, y1) = crate::world::map::mill_plot(site);
    let (x, y) = (fx.div_euclid(TILE_SIZE), fy.div_euclid(TILE_SIZE));
    let in_door = crate::world::map::mill_doorway(site).iter().any(|&(dx, _)| dx == x);
    (x0..=x1).contains(&x) && (y == y0 - 1 || (y1 - 2..y1).contains(&y) && !in_door)
}

/// Whether feet at world pixel `(fx, fy)` stand just in front of one of
/// the mill's walls, so close that the head reaches up into the rows where
/// the face is drawn on BG2 (over someone behind it): just south of the
/// south wall (outside), or inside just south of the north wall. The
/// castaway is then raised over BG2, as in front of a tree's canopy.
fn feet_before_mill_wall(map: &WorldMap, (fx, fy): (i32, i32)) -> bool {
    let Some(site) = map.spots.mill.filter(|_| map.has_mill_walls()) else { return false };
    let (x0, y0, x1, y1) = crate::world::map::mill_plot(site);
    let (x, y) = (fx.div_euclid(TILE_SIZE), fy.div_euclid(TILE_SIZE));
    // Under the roof, inside, no one is raised over it.
    (x0 - 1..=x1 + 1).contains(&x) && (y == y1 + 1 || y == y0 + 1 && !map.has_mill_roof())
}

/// Whether feet at world pixel `(fx, fy)` are inside the mill once its
/// roof is on (or in its doorway), hidden under the roof, casting no
/// shadow over it.
fn feet_under_mill_roof(map: &WorldMap, (fx, fy): (i32, i32)) -> bool {
    let Some(site) = map.spots.mill.filter(|_| map.has_mill_roof()) else { return false };
    let (x0, y0, x1, y1) = crate::world::map::mill_plot(site);
    let (x, y) = (fx.div_euclid(TILE_SIZE), fy.div_euclid(TILE_SIZE));
    (x0..=x1).contains(&x) && (y0..=y1).contains(&y)
}

fn player_in_front_of_canopy(map: &WorldMap, player: &Player) -> bool {
    use crate::assets::character::{CHAR_H, CHAR_ORIGIN, CHAR_W};
    use crate::world::terrain::{BroadleafPart, PalmPart};
    let (px, py) = player.body.px();
    let feet_row = player.body.feet().1.div_euclid(TILE_SIZE);
    let (x0, y0) = (px - CHAR_ORIGIN.0, py - CHAR_ORIGIN.1);
    let (x1, y1) = (x0 + CHAR_W as i32 - 1, y0 + CHAR_H as i32 - 1);
    for cy in y0.div_euclid(TILE_SIZE)..=y1.div_euclid(TILE_SIZE) {
        for cx in x0.div_euclid(TILE_SIZE)..=x1.div_euclid(TILE_SIZE) {
            let base_row = match map.feature(cx, cy) {
                crate::world::Feature::Palm(p) => cy - p.dy as i32 + PalmPart::H - 1,
                crate::world::Feature::Broadleaf(p) => cy - p.dy as i32 + BroadleafPart::H - 1,
                _ => continue,
            };
            if feet_row > base_row {
                return true;
            }
        }
    }
    false
}

/// Whether `p` lies within `r` px of the segment from `a` to `b`.
fn near_line(p: (i32, i32), a: (i32, i32), b: (i32, i32), r: f32) -> bool {
    let (px, py) = (p.0 as f32, p.1 as f32);
    let (ax, ay, bx, by) = (a.0 as f32, a.1 as f32, b.0 as f32, b.1 as f32);
    let (dx, dy) = (bx - ax, by - ay);
    let t = (((px - ax) * dx + (py - ay) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0);
    let (qx, qy) = (ax + dx * t - px, ay + dy * t - py);
    qx * qx + qy * qy <= r * r
}

/// The fishing line from the rod tip (world px) sagging down to the float,
/// and the float itself: riding, bobbing as a fish nibbles, or pulled under
/// with rings spreading.
fn draw_line(list: &mut DrawList, camera: &Camera, sprites: &SpriteIds, line: &Line, tip: (i32, i32), depth: i32, clock: u32) {
    let (fx, fy) = line.float;
    const DOTS: i32 = 7;
    for k in 1..DOTS {
        let t = k as f32 / DOTS as f32;
        let sag = 6.0 * t * (1.0 - t) * 4.0;
        let x = tip.0 as f32 + (fx - tip.0) as f32 * t;
        let y = tip.1 as f32 + (fy - 1 - tip.1) as f32 * t + sag;
        let (sx, sy) = camera.to_screen(x as i32, y as i32);
        list.actor(depth, ObjEntry::new(sx, sy, sprites.line_dot, OBJ_FX, ACTOR_PRIORITY));
    }
    let frame = match line.float() {
        Float::Riding => 0,
        Float::Bobbing => 1,
        Float::Under => 2,
    };
    // A gentle rise and fall on the swell while riding.
    let swell = if frame == 0 && (line.age / 30) % 2 == 1 { 1 } else { 0 };
    let (sx, sy) = camera.to_screen(fx - 4, fy - 4 + swell);
    list.actor(fy, ObjEntry::new(sx, sy, sprites.float[frame], OBJ_CRAB, ACTOR_PRIORITY));
    if frame == 2 {
        let ripple = sprites.ripples[(clock / 6) as usize % sprites.ripples.len()];
        let (rx, ry) = camera.to_screen(fx - 12, fy - 3);
        list.actor(fy - 1, ObjEntry::new(rx, ry, ripple, OBJ_WATER_FX, ACTOR_PRIORITY));
    }
}

/// How far (px) the side-on cart rolls for its wheel to turn round one
/// step (a sixth of the way round between spokes over `CART_WHEEL_STEPS`
/// pictures, on a wheel 11px across).
const CART_WHEEL_STEP_PX: i32 = 2;

/// How many frames the mill's door takes to slide a step.
const MILL_DOOR_FRAMES: u32 = 3;

/// The posts of the mill's frame, once it's up: sprites, sorted with the
/// castaway by their feet, so they stand in front of someone behind them
/// and behind someone in front (what spans between their heads is BG2, in
/// front of everyone). Once the walls are up, the south ones are hidden in
/// the south wall's face (it's drawn over them, so they're left out). At
/// night in the dark palette, as things on the ground are.
fn draw_mill_posts(list: &mut DrawList, camera: &Camera, sprites: &SpriteIds, map: &WorldMap, night: bool) {
    // Under the roof (on or being built), the posts and beams are hidden;
    // they're left out.
    let Some(site) = map.spots.mill.filter(|_| map.has_mill_frame() && !map.shows_mill_roof()) else { return };
    let (_, _, _, y1) = crate::world::map::mill_plot(site);
    let palette = if night { OBJ_CRAB_NIGHT } else { OBJ_CRAB };
    for stone in crate::world::map::foundation_stones(site) {
        let front = stone.1 == y1;
        if front && map.has_mill_walls() {
            continue;
        }
        let gfx = sprites.mill_post[front as usize];
        let rise = mill::post_rise(stone.1, y1);
        let (fx, fy) = mill::post_foot(stone);
        let tall = rise + mill::POST_FOOT;
        let h = (tall + 2 + 7) / 8 * 8;
        // The art's post stands against its bottom, a pixel above it.
        let (wx, wy) = (fx - 4, fy + 1 - h);
        if camera.sees(wx, wy, 8, h) {
            let (sx, sy) = camera.to_screen(wx, wy);
            list.actor(fy, ObjEntry::new(sx, sy, gfx, palette, 2));
        }
    }
    // The beams across between the plates, higher than anyone's head: in
    // front of everyone (over BG2 too, so over someone raised over BG2 in
    // front of the north wall's face).
    let (x0, y0, x1, _) = crate::world::map::mill_plot(site);
    let top = mill::post_head(y0, y1) - 1;
    let h = (mill::beam_len() + 2 + 7) / 8 * 8;
    for cx in [x0, (x0 + x1) / 2, x1] {
        let wx = cx * 16 + 8 - 4;
        if camera.sees(wx, top, 8, h) {
            let (sx, sy) = camera.to_screen(wx, top);
            list.sky(ObjEntry::new(sx, sy, sprites.mill_beam, palette, 3));
        }
    }
}

/// How long before the hearth's fuel has burnt away its fire dies down
/// (in-game minutes), as a campfire's does.
const HEARTH_DYING_MINUTES: u32 = 2 * 60;
/// Frames per flame animation step, for the hearth and the lamps.
const FLAME_FRAME: u32 = 5;

/// Draws the fire burning on the hearth in the mill, `at` its middle
/// (lively flames, as a campfire's; small once it's dying down). Unlit by
/// the night tint, as it lights itself.
fn draw_hearth_fire(list: &mut DrawList, camera: &Camera, sprites: &SpriteIds, (x, y): (i32, i32), dying: bool, clock: u32) {
    let (wx, wy) = (x - 8, y - 13);
    if !camera.sees(wx, wy, 16, 16) {
        return;
    }
    let flames = if dying { &sprites.fire.small } else { &sprites.fire.flames };
    let frame = (clock / FLAME_FRAME) as usize % flames.len();
    let (sx, sy) = camera.to_screen(wx, wy);
    list.actor(y + 8, ObjEntry::new(sx, sy, flames[frame], OBJ_CRAB, 2));
}

/// Draws the oil lamps hung high on the mill's west wall, one in each of
/// `cells` (lit, their flames flickering out of step).
fn draw_lamps(list: &mut DrawList, camera: &Camera, sprites: &SpriteIds, cells: &[(i32, i32)], lit: bool, clock: u32) {
    for (k, &(x, y)) in cells.iter().enumerate() {
        let (wx, wy) = (x * TILE_SIZE, y * TILE_SIZE - 10);
        if !camera.sees(wx, wy, 16, 16) {
            continue;
        }
        let gfx = if lit { sprites.lamp_lit[(clock / FLAME_FRAME + k as u32 * 2) as usize % 3] } else { sprites.lamp };
        let (sx, sy) = camera.to_screen(wx, wy);
        // Sorted by its cell's foot: behind anyone south of it, in front
        // of anyone north of it (no one stands in its cell).
        list.actor(y * TILE_SIZE + TILE_SIZE - 1, ObjEntry::new(sx, sy, gfx, OBJ_CRAB, 2));
    }
}

/// Draws the handcart where it stands, its poles pointing the way it was
/// last pulled (at night in the dark palette, as things on the ground are).
fn draw_cart(list: &mut DrawList, camera: &Camera, sprites: &SpriteIds, cart: &Cart, night: bool, handle_depth: Option<i32>) {
    let _ = night;
    // The picture for the way its poles point, where it goes from the
    // cart's middle (the wheel's hub side on; the middle of its ground end
    // on), and mirrored or not.
    // Side on, its wheel turns as it rolls: a step round for every
    // `CART_WHEEL_STEP_PX` along the ground (clockwise rolling right; the
    // picture is mirrored rolling left, so it's turned the other way first).
    let rolled = if cart.poles == Dir::Left { -cart.at.0 } else { cart.at.0 };
    let wheel = sprites.handcart_side[rolled.div_euclid(CART_WHEEL_STEP_PX).rem_euclid(sprites.handcart_side.len() as i32) as usize];
    let (gfx, (w, h), (bx, by), flip) = match cart.poles {
        Dir::Right => (wheel, (32, 24), (-12, -17), false),
        Dir::Left => (wheel, (32, 24), (-19, -17), true),
        Dir::Down => (sprites.handcart[0], (24, 24), (-12, -19), false),
        Dir::Up => (sprites.handcart[1], (24, 24), (-12, -23), false),
    };
    let (wx, wy) = (cart.at.0 + bx, cart.at.1 + by);
    if !camera.sees(wx, wy - 16, w, h + 32) {
        return;
    }
    let (sx, sy) = camera.to_screen(wx, wy);
    list.actor(cart.at.1, ObjEntry::new(sx, sy, gfx, OBJ_FX, 2).flipped(flip));
    // The handle, side on or from the front, runs from the bed past the
    // castaway pulling it to the crossbar before them (`handle_depth`: over
    // their body, under their hands); set down, it lies with the cart. From
    // behind it's part of the cart's picture (the crossbar out of sight).
    let handle = match cart.poles {
        Dir::Right => Some((sprites.handcart_handle[0], (12, -8), false)),
        Dir::Left => Some((sprites.handcart_handle[0], (-28, -8), true)),
        Dir::Down => Some((sprites.handcart_handle[1], (-12, -5), false)),
        Dir::Up => None,
    };
    if let Some((gfx, (hx, hy), flip)) = handle {
        let (sx, sy) = camera.to_screen(cart.at.0 + hx, cart.at.1 + hy);
        list.actor(handle_depth.unwrap_or(cart.at.1), ObjEntry::new(sx, sy, gfx, OBJ_FX, 2).flipped(flip));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn black_smoke_is_likelier_to_be_seen_than_pale() {
        assert!(rescue_chance(true) > rescue_chance(false));
    }
}

//! Top-level orchestration: owns every subsystem and runs one frame at a time.
//! `step` advances simulation; `render` produces the picture. They are kept
//! separate so the host can run fixed-rate updates independent of display.

pub mod bag;
pub mod fishing;
pub mod menu;
pub mod regions;
pub mod save;
pub mod state;

use crate::assets::{Assets, SpriteIds};
use crate::ppu::oam::ObjEntry;
use crate::assets::character::Outfit;
use crate::assets::palettes::{OBJ_CRAB, OBJ_FX, OBJ_WATER_FX, base_cgram, set_player_palettes};
use crate::common::input::{Buttons, InputState};
use crate::assets::props::{PALM_W, coconut_spots};
use crate::common::math::Rect;
use crate::common::rng::Rng;
use crate::entity::fish;
use crate::entity::player::ACTOR_PRIORITY;
use crate::entity::{Campfire, Crab, Deed, Fish, Gull, Item, Particles, Player, Situation};
use bag::Bag;
use fishing::{Float, Line};
use crate::ppu::effects::{ColorMath, MathOp, MathSource, WindowRegion, layer};
use crate::ppu::palette::Cgram;
use crate::ppu::{BgLayer, FrameState, Ppu, SCREEN_H, SCREEN_W};
use crate::render::camera::Camera;
use crate::crafting::{RECIPES, Recipe};
use crate::items::{ItemKind, Use};
use crate::render::bag_screen;
use crate::render::end_screen;
use crate::render::title_screen::{self, TitleItem};
use crate::render::hud::{self, HUD_LINES};
use crate::render::light_canvas;
use crate::render::lighting::{self, Lighting};
use crate::render::sprites::DrawList;
use crate::render::tileset::{Tileset, VisualMap, sway_palm};
use crate::world::harvest::{self, Harvest, Shaken, Strike};
use crate::world::{ChunkId, Feature, TILE_SIZE, Terrain, WorldMap, island};
use crate::world::map::BridgeSpan;
use menu::{BagMenu, Outcome};
use regions::Regions;
use save::{SAVE_VERSION, SaveData};
use state::{GameClock, Inventory, Rest, Vitals};

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
/// Frames the screen takes to fade out once the castaway collapses, and
/// before the end can be dismissed.
const COLLAPSE_FADE: u32 = 120;
const COLLAPSE_HOLD: u32 = 60;
/// Chance a shake brings a frond down, and fronds a felled palm sheds.
const FROND_CHANCE: f32 = 0.15;
const FELLED_FRONDS: u32 = 3;
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

/// A palm canopy wobbling after a hit: world-space rows and frames left.
#[derive(Clone, Copy, Debug)]
struct TreeShake {
    anchor: (i32, i32),
    frames: u32,
}

pub struct Game {
    assets: Assets,
    map: WorldMap,
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
    /// The day shells last washed up, and the most the beach holds (as
    /// many as it started with).
    shell_day: u32,
    shell_cap: usize,
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
    /// Frames since the castaway collapsed, once they have.
    collapsed: Option<u32>,
    gulls: Vec<Gull>,
    camera: Camera,
    clock: GameClock,
    vitals: Vitals,
    /// Frames of the minute going by spent running (it tires faster).
    ran_frames: u32,
    inventory: Inventory,
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
        if let Some(s) = save {
            for &((x, y), f) in &s.island.features {
                map.set_feature(x, y, f);
            }
            map.restore_trees(&s.island.fruit, &s.island.bark);
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
        let gulls = vec![
            Gull::new((sx + 40.0, sy - 30.0), (120.0, 60.0), 0.006, 0.0),
            Gull::new((sx - 80.0, sy + 10.0), (90.0, 70.0), -0.008, 2.0),
        ];
        if let Some(s) = save {
            regions.restore(&s.island.asleep_since);
        }
        let mut game = Self {
            assets,
            map,
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
            shell_day: 0,
            shell_cap: 0,
            regions,
            seed,
            title: save.is_none().then(|| TitleState::opening(false)),
            saved: None,
            outbox: None,
            save_kept: false,
            collapsed: None,
            gulls,
            camera: Camera::new(0),
            clock,
            vitals: save.map_or_else(Vitals::new, |s| s.vitals.clone()),
            ran_frames: 0,
            inventory: Inventory::default(),
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
        game.camera.follow(game.player.center(), game.map.pixel_size());
        game.stream_ground();
        game.wake_regions();
        game
    }

    /// The game as it stands, to be saved.
    fn save_data(&self) -> SaveData {
        let made = island::generate(self.seed);
        let mut features = Vec::new();
        for y in 0..self.map.height() {
            for x in 0..self.map.width() {
                let f = self.map.feature(x, y);
                if f != made.feature(x, y) {
                    features.push(((x, y), f));
                }
            }
        }
        let (fruit, bark) = self.map.tree_changes(&made);
        let inv = &self.inventory;
        SaveData {
            version: SAVE_VERSION,
            seed: self.seed,
            clock: save::Clock::of(&self.clock),
            vitals: self.vitals.clone(),
            castaway: save::Castaway { at: self.player.body.px(), facing: self.player.facing, outfit: save::Castaway::outfit_index(&inv.outfit) },
            inventory: save::Carried { bag: inv.bag.clone(), torch: inv.torch, recipes: inv.recipes.clone(), gear: inv.gear },
            island: save::IslandState {
                features,
                fruit,
                bark,
                harvest: self.harvest.state(),
                items: self.items.iter().map(|i| save::Lying { kind: i.kind, count: i.count, at: i.ground() }).collect(),
                fires: self.fires.clone(),
                crabs: self.crabs.iter().map(|c| c.body.px()).collect(),
                fish: self.fish.iter().map(|f| f.pos()).collect(),
                fish_due: self.fish_due.clone(),
                shell_day: self.shell_day,
                shell_cap: self.shell_cap,
                asleep_since: self.regions.asleep_since(),
            },
        }
    }

    /// Saves the game: the page is handed it to keep.
    fn save(&mut self) {
        let data = self.save_data();
        self.outbox = Some(SaveOp::Write(data.to_json()));
        self.saved = Some(data);
        self.save_kept = true;
    }

    /// Erases the saved game (the castaway has collapsed).
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
        if let Some(t) = self.collapsed {
            // The end: the screen fades, then START begins again.
            self.frame_count = self.frame_count.wrapping_add(1);
            self.collapsed = Some(t + 1);
            if t >= COLLAPSE_FADE + COLLAPSE_HOLD && self.input.pressed(Buttons::START) {
                // Back to the title, to begin again.
                self.start_over();
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
        let near_fire = self.near_fire();
        if let Some(menu) = &mut self.menu {
            menu.near_fire = near_fire;
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
        if self.input.pressed(Buttons::X) && self.player.is_idle() {
            self.menu = Some(BagMenu::browse());
            return;
        }
        if self.player.is_asleep() {
            self.player.sheltered = self.inventory.bag.has(ItemKind::Shelter);
            self.sleep_on();
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
        self.player.weak = self.vitals.starving();
        let now = self.clock.total_minutes();
        self.inventory.tend_torch(now);
        if self.clock.day != self.shell_day {
            self.shell_day = self.clock.day;
            self.wash_up_shells();
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
        let situation = Situation {
            tool: self.inventory.gear.hand,
            has_leaf: self.inventory.bag.has(ItemKind::Leaf),
            has_knife: self.inventory.bag.has(ItemKind::StoneKnife),
            item_in_reach: self.pickup_target().map(|t| self.pickup_point(t)),
            fish_near: !self.inventory.bag.has(ItemKind::FishingRod) && self.fish_near(self.player.cast_point()),
        };
        let was_asleep = self.player.is_asleep();
        match self.player.update(&self.input, &self.map, situation) {
            Some(Deed::Chop(point)) => self.resolve_axe(point),
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
            None => {}
        }
        if was_asleep && !self.player.is_asleep() {
            // Each waking is saved.
            self.save();
        }
        self.tend_line();
        let now = self.clock.total_minutes();
        self.fires.retain(|f| !f.burnt_out(now));
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
        self.camera.follow(pc, self.map.pixel_size());
    }

    /// Changes what the castaway wears: re-renders their frames into VRAM and
    /// updates their palettes.
    pub fn equip(&mut self, outfit: Outfit) {
        self.inventory.outfit = outfit;
        self.assets.sprites.player.rebuild(&mut self.assets.vram.sprites, &outfit);
        set_player_palettes(&mut self.base_cgram, &outfit);
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
        let (w, h) = self.map.pixel_size();
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
        fish::swimmable(&self.map, (x, y)) && self.fish.iter().any(|f| {
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
        (0..self.map.height())
            .flat_map(|y| (0..self.map.width()).map(move |x| (x, y)))
            .filter(|&(x, y)| self.map.feature(x, y) == Feature::Shell)
            .collect()
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
            let open = self.map.terrain(x, y) == Terrain::Sand && !self.map.is_high(x, y) && self.map.feature(x, y) == Feature::None && !self.map.is_riverside(x, y);
            if open && near_water(&self.map, x, y) {
                self.map.set_feature(x, y, Feature::Shell);
                changed.push((x, y));
            }
        }
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

    /// Scoops water up in a leaf folded into a cup.
    fn draw_water(&mut self) {
        if self.inventory.bag.take_kind(ItemKind::Leaf) {
            self.take(ItemKind::RawWater, 1);
        }
    }

    /// Puts `count` of something just obtained into the bag; if they don't
    /// all fit, the bag opens with the rest in hand so room can be made (or
    /// they can be left).
    fn take(&mut self, kind: ItemKind, count: u32) {
        if !kind.goes_in_bag() {
            // A torch lit at the fire is carried straight off.
            self.inventory.light_torch(self.clock.total_minutes());
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
            Outcome::Save => {
                self.save();
                if let Some(menu) = &mut self.menu {
                    menu.notice = Some("SAVED.");
                }
            }
        }
    }

    /// One frame of sleep: time hurries on (hunger and thirst with it, if
    /// more slowly) and sleep mends health (faster under a shelter; see
    /// `Vitals`). The castaway sleeps on until woken with a button.
    fn sleep_on(&mut self) {
        let rest = if self.player.sheltered { Rest::Sheltered } else { Rest::Asleep };
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
        self.collapsed = Some(0);
        // There's no going back: the saved game goes with them.
        self.erase_save();
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

    /// The campfire the castaway stands by, if any.
    fn nearest_fire(&self) -> Option<&Campfire> {
        let (fx, fy) = self.player.body.feet();
        self.fires
            .iter()
            .filter(|f| f.within((fx, fy), NEAR_FIRE))
            .min_by_key(|f| (f.x - fx).pow(2) + (f.y - fy).pow(2))
    }

    /// Takes a recipe's materials and kneels to cook it over the nearest
    /// fire; it goes in the bag when done.
    fn start_cooking(&mut self, recipe: &Recipe) {
        let Some(at) = self.nearest_fire().map(|f| (f.x, f.y)) else {
            return;
        };
        self.menu = None;
        self.inventory.bag.use_up(recipe);
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
        let mut camera = self.camera;
        if self.camera_shake > 0 {
            camera.y += if self.camera_shake % 4 < 2 { 1 } else { -1 };
        }
        let f = &mut self.frame;
        f.clock = self.frame_count;
        f.scroll_x = camera.x;
        f.scroll_y = camera.y;
        f.brightness = 15;
        if let Some(t) = self.collapsed {
            if t >= COLLAPSE_FADE {
                // The end screen, fading in on black.
                f.cgram.clone_from(&self.base_cgram);
                f.cgram.colors[0] = crate::ppu::palette::Rgb555::BLACK;
                f.brightness = ((t - COLLAPSE_FADE) / 2).min(15) as u8;
                f.lines.clear();
                f.math = ColorMath::OFF;
                f.main_layers = layer::BG3;
                f.bg3_priority = false;
                let start = GameClock::new(1, START_HOUR).total_minutes();
                let lasted = self.clock.total_minutes() - start;
                let prompt = t >= COLLAPSE_FADE + COLLAPSE_HOLD && (self.frame_count / 30).is_multiple_of(2);
                end_screen::draw(&mut f.bg3, &self.assets.hud, &self.clock, lasted, prompt);
                f.oam.clear();
                self.ppu.render(&self.assets.vram, &self.frame, &self.visual.ground, &self.visual.overhead);
                return;
            }
            // The world fades out around the fallen castaway.
            f.brightness = (15 - 15 * t / COLLAPSE_FADE) as u8;
        }

        // CGRAM: base palette → water cycling → time-of-day tint.
        f.cgram.clone_from(&self.base_cgram);
        lighting::cycle_water(&mut f.cgram, self.frame_count);
        lighting.apply(&mut f.cgram);

        if let Some(menu) = &self.menu {
            // The bag screen: world layers off, BG3 behind the icon sprites.
            f.cgram.clone_from(&self.base_cgram);
            f.lines.clear();
            f.math = ColorMath::OFF;
            f.main_layers = layer::BG3 | layer::OBJ;
            f.bg3_priority = false;
            hud::draw(&mut f.bg3, &self.assets.hud, &self.vitals, &self.clock, night);
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
        if night {
            // Every light: a torch carried and each campfire.
            let torch = self.inventory.torch_lit().then(|| self.player.center());
            let lights: Vec<(i32, i32)> = torch.into_iter().chain(self.fires.iter().map(|f| f.flame_point())).collect();
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
            None => hud::draw(&mut f.bg3, &self.assets.hud, &self.vitals, &self.clock, night),
        }

        // Sprites.
        let shadows = !night;
        let list = &mut self.draw_list;
        list.clear();
        let sprites = &self.assets.sprites;
        let in_front = player_in_front_of_canopy(&self.map, &self.player);
        self.player.draw(list, &camera, sprites, self.frame_count, shadows, in_front);
        let now = self.clock.total_minutes();
        for fire in &self.fires {
            fire.draw(list, &camera, sprites, self.frame_count, now);
        }
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
        self.particles.draw(list, &camera, sprites, self.frame_count);
        if !night {
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

    /// Dev tools: the most shells the tide leaves each morning (it grows
    /// with the island).
    pub fn debug_shells_per_morning(&self) -> usize {
        island::for_area(&self.map, SHELLS_PER_DAY)
    }

    /// Dev tools: the game as it stands, as a save (JSON).
    pub fn debug_save_json(&self) -> String {
        self.save_data().to_json()
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
        self.camera.follow(self.player.center(), self.map.pixel_size());
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
        self.camera.follow(self.player.center(), self.map.pixel_size());
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

    pub fn debug_give(&mut self, kind: ItemKind) {
        self.debug_give_n(kind, kind.max_stack());
    }

    /// Puts up to `n` of a kind in the bag; returns how many went in.
    pub fn debug_give_n(&mut self, kind: ItemKind, n: u32) -> u32 {
        if !kind.goes_in_bag() {
            self.inventory.light_torch(self.clock.total_minutes());
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

    pub fn player_tile(&self) -> (i32, i32) {
        let (x, y) = self.player.body.feet();
        (x.div_euclid(crate::world::TILE_SIZE), y.div_euclid(crate::world::TILE_SIZE))
    }

    pub fn clock(&self) -> &GameClock {
        &self.clock
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

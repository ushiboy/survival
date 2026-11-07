//! Top-level orchestration: owns every subsystem and runs one frame at a time.
//! `step` advances simulation; `render` produces the picture. They are kept
//! separate so the host can run fixed-rate updates independent of display.

pub mod bag;
pub mod menu;
pub mod state;

use crate::assets::Assets;
use crate::assets::character::Outfit;
use crate::assets::palettes::{HUD_LIGHT_MASK, base_cgram, set_player_palettes};
use crate::common::input::{Buttons, InputState};
use crate::assets::props::{PALM_W, coconut_spots};
use crate::common::rng::Rng;
use crate::entity::{Campfire, Crab, Deed, Gull, Item, Particles, Player, Situation};
use crate::ppu::effects::{ColorMath, MathOp, MathSource, WindowRegion, layer};
use crate::ppu::palette::Cgram;
use crate::ppu::{BgLayer, FrameState, Ppu, SCREEN_H, SCREEN_W};
use crate::render::camera::Camera;
use crate::crafting::{RECIPES, Recipe};
use crate::items::{ItemKind, Use};
use crate::render::bag_screen;
use crate::render::hud::{self, HUD_LINES};
use crate::render::lighting::{self, Light, Lighting};
use crate::render::sprites::DrawList;
use crate::render::tileset::{Tileset, VisualMap, sway_palm};
use crate::world::harvest::{self, Harvest, Shaken, Strike};
use crate::world::{TILE_SIZE, WorldMap, island};
use menu::{BagMenu, Outcome};
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
/// second), and the minutes of sleep that restore a heart.
const SLEEP_MINUTES_PER_FRAME: u32 = 2;
const SLEEP_HEAL_MINUTES: u32 = 60;
/// Minutes of sleep per heart restored under a shelter.
const SHELTER_HEAL_MINUTES: u32 = 30;
/// Chance a shake brings a frond down, and fronds a felled palm sheds.
const FROND_CHANCE: f32 = 0.15;
const FELLED_FRONDS: u32 = 3;

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
    fires: Vec<Campfire>,
    /// In-game minutes the castaway has been asleep.
    slept: u32,
    gulls: Vec<Gull>,
    camera: Camera,
    clock: GameClock,
    vitals: Vitals,
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
        let mut assets = Assets::load();
        let map = island::generate(seed);
        let items = map.loot.iter().map(|&(kind, (x, y))| Item::lying(kind, (x as f32, y as f32))).collect();
        let mut tileset = Tileset::default();
        let visual = tileset.build(&map, &mut assets);
        // The story opens with the castaway washed up on the beach.
        let player = Player::washed_up(map.spawn.0, map.spawn.1);
        let crabs = map
            .crab_spawns
            .iter()
            .enumerate()
            .map(|(i, &(x, y))| Crab::new(x, y, seed.wrapping_add(i as u32 * 31)))
            .collect();
        let (sx, sy) = (map.spawn.0 as f32, map.spawn.1 as f32);
        let gulls = vec![
            Gull::new((sx + 40.0, sy - 30.0), (120.0, 60.0), 0.006, 0.0),
            Gull::new((sx - 80.0, sy + 10.0), (90.0, 70.0), -0.008, 2.0),
        ];
        let mut game = Self {
            assets,
            map,
            tileset,
            visual,
            harvest: Harvest::default(),
            particles: Particles::new(seed ^ 0xFA57),
            items,
            rng: Rng::new(seed ^ 0xC0C0),
            hitstop: 0,
            tree_shake: None,
            camera_shake: 0,
            player,
            crabs,
            fires: Vec::new(),
            slept: 0,
            gulls,
            camera: Camera::new(0),
            clock: GameClock::new(1, START_HOUR),
            vitals: Vitals::new(),
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
        game.camera.follow(game.player.center(), game.map.pixel_size());
        game.place_light_mask();
        game
    }

    /// Advances the simulation by one frame (1/60 s).
    pub fn step(&mut self, buttons: u16) {
        self.input.update(buttons);
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
        if self.input.pressed(Buttons::L) {
            // Debug: give / take the lantern until items exist.
            self.inventory.lantern = !self.inventory.lantern;
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
            self.slept = 0;
            if self.clock.tick() {
                self.vitals.on_minute(Rest::Awake);
            }
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
            item_in_reach: self.item_in_reach().map(|i| self.items[i].ground()),
        };
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
            None => {}
        }
        let now = self.clock.total_minutes();
        self.fires.retain(|f| !f.burnt_out(now));
        for fire in &self.fires {
            if self.rng.range(EMBER_EVERY) == 0 {
                self.particles.ember(fire.flame_point());
            }
        }
        self.particles.update();
        for item in &mut self.items {
            item.update(&self.map);
        }
        let pc = self.player.center();
        let (feet, moving) = (self.player.body.feet(), self.player.moving);
        for crab in &mut self.crabs {
            crab.update(&self.map, feet, moving);
        }
        for gull in &mut self.gulls {
            gull.update();
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
            Strike::TreeFelled { anchor, changed, timber, wood, fruit } => {
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
                // And a few fronds off the crown.
                for _ in 0..FELLED_FRONDS {
                    self.drop_frond(anchor);
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
        // Now and then a dead frond comes away too.
        if self.rng.chance(FROND_CHANCE) {
            self.drop_frond(anchor);
        }
        self.start_sway(anchor, sway);
    }

    /// Sends the coconut hanging in `slot` of the palm at `anchor` falling.
    fn drop_coconut(&mut self, anchor: (i32, i32), slot: u8, flip: bool) {
        let (hx, hy) = coconut_spots()[slot as usize];
        let hx = if flip { PALM_W as f32 - hx } else { hx };
        let from = ((anchor.0 * TILE_SIZE) as f32 + hx, (anchor.1 * TILE_SIZE) as f32 + hy);
        self.drop_item(ItemKind::Coconut, anchor, from);
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

    /// Takes the item within reach (if it hasn't rolled away) into the bag.
    fn pick_up(&mut self) {
        let Some(i) = self.item_in_reach() else {
            return;
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
        }
    }

    /// One frame of sleep: time hurries on (hunger and thirst with it, if
    /// more slowly) and every hour slept restores a heart (every half hour,
    /// and hunger and thirst slower still, under a shelter). The castaway sleeps on until
    /// woken with a button.
    fn sleep_on(&mut self) {
        let sheltered = self.player.sheltered;
        let (rest, heal_every) = if sheltered { (Rest::Sheltered, SHELTER_HEAL_MINUTES) } else { (Rest::Asleep, SLEEP_HEAL_MINUTES) };
        for _ in 0..SLEEP_MINUTES_PER_FRAME {
            self.clock.advance_minutes(1);
            self.vitals.on_minute(rest);
            self.slept += 1;
            if self.slept % heal_every == 0 {
                self.vitals.heal(1);
            }
        }
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
        if !self.inventory.bag.has(ItemKind::Branch) {
            return Err("NEEDS A BRANCH TO BURN");
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

    /// The drill has caught: a branch goes on as fuel and the fire burns
    /// for a day.
    fn light_fire(&mut self, spot: (i32, i32)) {
        if !self.inventory.bag.take_kind(ItemKind::Branch) {
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

    /// Sets up PPU state for this frame and renders it to the RGBA buffer.
    pub fn render(&mut self) {
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
            hud::draw(&mut f.bg3, &self.assets.hud, &self.vitals, &self.inventory, &self.clock, night);
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
            // One light source: the lantern, or else the fire nearest the
            // middle of the screen.
            let (cx, cy) = self.player.center();
            let lantern = self.inventory.lantern.then_some((cx, cy));
            let mid = (camera.x + SCREEN_W as i32 / 2, camera.y + SCREEN_H as i32 / 2);
            let fire = self
                .fires
                .iter()
                .map(|f| f.flame_point())
                .min_by_key(|&(x, y)| (x - mid.0).pow(2) + (y - mid.1).pow(2));
            let light = lantern.or(fire).map(|p| Light { center: camera.to_screen(p.0, p.1) });
            let t = ((lighting.darkness - NIGHT_THRESHOLD) / (1.0 - NIGHT_THRESHOLD)).clamp(0.0, 1.0);
            let strength = t * t * (3.0 - 2.0 * t);
            lighting::night(&mut f.lines, &mut f.cgram, light, HUD_LINES, strength, self.frame_count);
            f.math = MATH_NIGHT;
        } else {
            f.lines.clear();
            f.math = MATH_TRANSLUCENT;
        }

        hud::draw(&mut f.bg3, &self.assets.hud, &self.vitals, &self.inventory, &self.clock, night);

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

    /// Writes the lantern light mask into an off-screen area of the BG3 map.
    fn place_light_mask(&mut self) {
        let n = crate::assets::hud_art::LIGHT_MASK_TILES;
        let (ox, oy) = (lighting::LIGHT_MASK_ORIGIN.0 as usize / 8, lighting::LIGHT_MASK_ORIGIN.1 as usize / 8);
        for (i, &tile) in self.assets.hud.light_mask.iter().enumerate() {
            self.frame.bg3.put(ox + i % n, oy + i / n, tile, HUD_LIGHT_MASK);
        }
    }

    /// Dev tools: puts the castaway, awake, with their feet in a cell.
    pub fn debug_place_player(&mut self, (x, y): (i32, i32)) {
        self.player = Player::at_feet(x * TILE_SIZE, y * TILE_SIZE);
        self.camera.follow(self.player.center(), self.map.pixel_size());
    }

    /// Debug: cycles outfit presets until equipment exists.
    pub fn debug_cycle_outfit(&mut self) {
        let presets = Outfit::PRESETS;
        let i = presets.iter().position(|o| *o == self.inventory.outfit).unwrap_or(0);
        self.equip(presets[(i + 1) % presets.len()]);
    }

    /// Dev tools: sets the castaway's health in hearts.
    pub fn debug_set_hp(&mut self, hearts: u8) {
        self.vitals.hp = hearts.min(self.vitals.max_hp);
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
    pub fn debug_give(&mut self, kind: ItemKind) {
        self.debug_give_n(kind, kind.max_stack());
    }

    /// Puts up to `n` of a kind in the bag; returns how many went in.
    pub fn debug_give_n(&mut self, kind: ItemKind, n: u32) -> u32 {
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

    /// VRAM needed for the current screen: the unique background tiles in
    /// view (as if streamed in while scrolling) plus all sprite and HUD tiles.
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

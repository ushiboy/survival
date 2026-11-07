use super::body::Body;
use crate::assets::palettes::{OBJ_CRAB, OBJ_FX, OBJ_PLAYER, OBJ_SHADOW, OBJ_WADE, OBJ_WATER_FX};
use crate::assets::{SpriteIds, VIEWS};
use crate::assets::character::pose::AxeArt;
use crate::assets::character::{CHAR_ORIGIN, CHAR_W};
use crate::ppu::banks::SpriteId;
use crate::common::input::{Buttons, InputState};
use crate::common::math::{Dir, Rect};
use crate::ppu::oam::ObjEntry;
use crate::render::camera::Camera;
use crate::render::sprites::DrawList;
use crate::world::WorldMap;
use crate::items::{ItemKind, Use};
use crate::world::harvest::{can_pull_vine, can_shake, has_bark, is_broadleaf_trunk, is_bush};

pub const SPRITE_W: i32 = 16;
pub const SPRITE_H: i32 = 24;
/// Feet-only collision box (top-down games collide at the feet so heads can
/// overlap things above them).
const HITBOX: Rect = Rect::new(3, 18, 10, 6);
/// Speeds in 8.8 fixed point per frame.
const SPEED_WALK: i32 = 336; // ~1.3 px
const SPEED_WALK_DIAG: i32 = 238;
/// Running (R held), not possible while wading.
const SPEED_RUN: i32 = 560; // ~2.2 px
const SPEED_RUN_DIAG: i32 = 396;
const SPEED_WADE: i32 = 184;
const SPEED_WADE_DIAG: i32 = 130;
const CORNER_ASSIST_PX: i32 = 6;
/// Frames per walk-animation step.
const STEP_FRAMES_WALK: u32 = 8;
const STEP_FRAMES_WADE: u32 = 12;
const STEP_FRAMES_RUN: u32 = 5;
/// Frames between puffs of dust kicked up by a runner's feet.
const DUST_EVERY: u32 = 8;
/// Sprite priority for actors: above BG1/BG2 low tiles, below BG2 high (canopies).
pub const ACTOR_PRIORITY: u8 = 2;
/// Above BG2 high tiles: used while standing in front of a canopy.
const FRONT_PRIORITY: u8 = 3;

/// Axe swing timing (frames): pull back to the side, sweep past the body
/// from `CHOP_SWEEP`, connect on `CHOP_STRIKE`, then recover.
const CHOP_SWEEP: u32 = 6;
const CHOP_STRIKE: u32 = 8;
const CHOP_TOTAL: u32 = 22;
/// How long the swoosh trail shows around the strike.
const SWOOSH_FRAMES: u32 = 6;

/// Spear thrust (frames): drawn back, then jabbing out from `THRUST_JAB`;
/// the point strikes on `THRUST_HIT` and is held out until `THRUST_BACK`.
const THRUST_JAB: u32 = 6;
const THRUST_HIT: u32 = 7;
const THRUST_BACK: u32 = 14;
const THRUST_TOTAL: u32 = 20;

/// Shaking a trunk (frames): push and pull alternate every `SHAKE_STEP`;
/// the tree registers one shake on `SHAKE_HIT` of each round.
const SHAKE_STEP: u32 = 5;
const SHAKE_HIT: u32 = 2;
const SHAKE_TOTAL: u32 = 4 * SHAKE_STEP;

/// Picking something up (frames): crouch, take hold on `PICK_GRAB`, rise.
const PICK_GRAB: u32 = 7;
const PICK_TOTAL: u32 = 16;

/// Drawing water (frames): crouch, scoop it up on `SCOOP_GRAB`, and rise
/// from `SCOOP_RISE`.
const SCOOP_GRAB: u32 = 11;
const SCOOP_RISE: u32 = 16;
const SCOOP_TOTAL: u32 = 22;
/// Drinking (frames): raise the water to the mouth until `DRINK_RAISE`,
/// it's gulped down on `DRINK_SIP`, then lower the hands.
const DRINK_RAISE: u32 = 6;
const DRINK_SIP: u32 = 22;
const DRINK_TOTAL: u32 = 36;

/// Eating (frames): bring the food up until `EAT_RAISE`, take a bite on
/// each of `EAT_BITES` (the last finishes it: `EAT_BITE`), chew, then drop
/// the empty shell.
const EAT_RAISE: u32 = 8;
const EAT_BITES: [u32; 3] = [16, 30, 44];
const EAT_BITE: u32 = EAT_BITES[2];
const EAT_TOTAL: u32 = 60;
/// Pondering (frames): the thought bubble's dots appear one by one every
/// `PONDER_DOT`, until it's given up on at `PONDER_TOTAL`.
const PONDER_DOT: u32 = 12;
const PONDER_TOTAL: u32 = 60;

/// Lighting a fire (frames): kneeling and sawing the drill's bow back and
/// forth (one stroke every `DRILL_STROKE`), smoke from `DRILL_SMOKE`, until
/// the tinder catches on `DRILL_CATCH`.
const DRILL_STROKE: u32 = 6;
const DRILL_SMOKE: u32 = 50;
const DRILL_CATCH: u32 = 100;
const DRILL_TOTAL: u32 = 112;

/// Cooking over a fire (frames): kneeling with the food held over the
/// flames (turned every `COOK_TURN`) until it's done on `COOK_DONE`.
const COOK_TURN: u32 = 24;
const COOK_DONE: u32 = 150;
const COOK_TOTAL: u32 = 162;

/// Pulling a vine off its stake (frames): take hold, then yank it free on
/// `PULL_YANK`.
const PULL_YANK: u32 = 14;
const PULL_TOTAL: u32 = 26;

/// Waking up on the beach (frames): lying still, stirring (eyes open and a
/// blink), sitting up, getting onto one knee, standing and looking around.
const WAKE_STIR: u32 = 110;
const WAKE_SIT: u32 = 160;
const WAKE_KNEEL: u32 = 205;
const WAKE_STAND: u32 = 232;
const WAKE_END: u32 = 330;
/// Getting up after sleep ends this soon after standing.
const WAKE_FROM_SLEEP_END: u32 = WAKE_STAND + 10;
/// Earliest frame a button press can skip the wake-up.
const WAKE_SKIPPABLE: u32 = 20;
/// Frames after lying down before a button press wakes the castaway (so
/// the press that lay them down doesn't).
const SLEEP_SETTLE: u32 = 30;
/// Frames between the Zs drifting up from a sleeper.
const SNORE_EVERY: u32 = 40;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Idle,
    /// Standing hand to chin, wondering how to go about something that
    /// needs a tool or material not carried; `t` counts frames.
    Ponder { t: u32 },
    /// Swinging the axe; `t` counts frames since the swing started.
    Chop { t: u32 },
    /// Jabbing with the spear.
    Thrust { t: u32 },
    /// Coming to on the beach; `t` counts frames since the game started,
    /// and it's over at `end`. Waking from sleep picks this up from sitting
    /// up and ends once standing, without looking around.
    WakeUp { t: u32, end: u32 },
    /// Lying asleep; `t` counts frames since lying down.
    Sleep { t: u32 },
    /// Gripping a trunk and rocking it; `t` counts frames in this round.
    Shake { t: u32 },
    /// Crouching to pick something up.
    PickUp { t: u32 },
    /// Scooping up water to carry (raw water).
    Scoop { t: u32 },
    /// Drinking water from the bag.
    Drink { t: u32, water: ItemKind },
    /// Eating something from the bag.
    Eat { t: u32, food: ItemKind },
    /// Kneeling to drill a fire.
    Drill { t: u32 },
    /// Kneeling by the fire at `fire` (ground px), holding `food` over it
    /// until it becomes `dish`.
    Cook { t: u32, food: ItemKind, dish: ItemKind, fire: (i32, i32) },
    /// Tugging a vine off a climbing plant, or leaves off a bush.
    Pull { t: u32 },
}

/// What the castaway has to hand and what lies around them this frame.
#[derive(Clone, Copy, Debug, Default)]
pub struct Situation {
    /// The tool in hand, if any.
    pub tool: Option<ItemKind>,
    /// Where something lies close enough to pick up.
    pub item_in_reach: Option<(i32, i32)>,
    /// Carrying a leaf to scoop water into.
    pub has_leaf: bool,
    /// Carrying a knife to cut bark and vine with.
    pub has_knife: bool,
}

/// Something the castaway did to the world this frame, at a world pixel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Deed {
    /// An axe blow connected.
    Chop((i32, i32)),
    /// The spear point struck out along the line from the first point to
    /// the second (on the ground, world px).
    Thrust((i32, i32), (i32, i32)),
    /// A trunk was shaken.
    Shake((i32, i32)),
    /// Hands closed on whatever lies within reach.
    PickUp,
    /// Tugged at the plant at the point: a vine coming away, or a handful
    /// of leaves off a bush.
    Pull((i32, i32)),
    /// Scooped up water to carry.
    DrawWater,
    /// Drank some water taken from the bag.
    Drink(ItemKind),
    /// Finished eating something taken from the bag.
    Eat(ItemKind),
    /// The drill is smoking at the point (ground px).
    Smoulder((i32, i32)),
    /// Running feet kicked up dust at the point (ground px).
    KickDust((i32, i32)),
    /// A fire caught at the point (ground px).
    LightFire((i32, i32)),
    /// Finished cooking this.
    Cooked(ItemKind),
}

#[derive(Clone, Debug)]
pub struct Player {
    pub body: Body,
    pub facing: Dir,
    pub moving: bool,
    /// Moving at a run (R held, on dry ground).
    pub running: bool,
    pub wading: bool,
    pub action: Action,
    /// Sleeping under a pitched shelter (one is carried): only the
    /// shelter is drawn.
    pub sheltered: bool,
    walk_timer: u32,
}

impl Player {
    /// Places the player so their feet stand on the given cell's top-left.
    pub fn at_feet(px: i32, py: i32) -> Self {
        Self::new(px, py - (SPRITE_H - 16))
    }

    /// Lying unconscious on the sand with feet on the given cell, about to
    /// wake up.
    pub fn washed_up(px: i32, py: i32) -> Self {
        let mut p = Self::at_feet(px, py);
        p.action = Action::WakeUp { t: 0, end: WAKE_END };
        p.facing = Dir::Right;
        p
    }

    pub fn new(px: i32, py: i32) -> Self {
        Self {
            body: Body::new(px, py, HITBOX),
            facing: Dir::Down,
            sheltered: false,
            moving: false,
            running: false,
            wading: false,
            action: Action::Idle,
            walk_timer: 0,
        }
    }

    /// Advances one frame. Returns what the castaway did to the world, on
    /// the frame it happens (an axe blow landing, a trunk being shaken, an
    /// item taken). Picking up what's in reach takes priority over shaking
    /// for A.
    pub fn update(&mut self, input: &InputState, map: &WorldMap, situation: Situation) -> Option<Deed> {
        let Situation { tool, item_in_reach, has_leaf, has_knife } = situation;
        if let Action::WakeUp { t, end } = self.action {
            let skip = t >= WAKE_SKIPPABLE && (input.pressed(Buttons::A) || input.pressed(Buttons::B));
            let t = if skip { end } else { t + 1 };
            if t >= end {
                self.action = Action::Idle;
                self.facing = Dir::Down;
            } else {
                self.action = Action::WakeUp { t, end };
                // Look around once standing.
                self.facing = if (WAKE_STAND + 30..WAKE_STAND + 60).contains(&t) { Dir::Left } else { Dir::Right };
            }
            return None;
        }
        if let Action::Sleep { t } = self.action {
            let roused = [Buttons::A, Buttons::B, Buttons::Y].into_iter().any(|b| input.pressed(b));
            if t >= SLEEP_SETTLE && roused {
                self.wake();
            } else {
                self.action = Action::Sleep { t: t + 1 };
            }
            return None;
        }
        if let Action::Chop { t } = self.action {
            let t = t + 1;
            self.action = if t >= CHOP_TOTAL { Action::Idle } else { Action::Chop { t } };
            return (t == CHOP_STRIKE).then(|| Deed::Chop(self.reach_point()));
        }
        if let Action::Thrust { t } = self.action {
            let t = t + 1;
            self.action = if t >= THRUST_TOTAL { Action::Idle } else { Action::Thrust { t } };
            return (t == THRUST_HIT).then(|| {
                let (from, to) = self.spear_line();
                Deed::Thrust(from, to)
            });
        }
        if let Action::PickUp { t } = self.action {
            let t = t + 1;
            self.action = if t >= PICK_TOTAL { Action::Idle } else { Action::PickUp { t } };
            return (t == PICK_GRAB).then_some(Deed::PickUp);
        }
        if let Action::Pull { t } = self.action {
            let t = t + 1;
            self.action = if t >= PULL_TOTAL { Action::Idle } else { Action::Pull { t } };
            return (t == PULL_YANK).then(|| Deed::Pull(self.reach_point()));
        }
        if let Action::Eat { t, food } = self.action {
            let t = t + 1;
            self.action = if t >= EAT_TOTAL { Action::Idle } else { Action::Eat { t, food } };
            return (t == EAT_BITE).then_some(Deed::Eat(food));
        }
        if let Action::Drill { t } = self.action {
            let t = t + 1;
            self.action = if t >= DRILL_TOTAL { Action::Idle } else { Action::Drill { t } };
            let spot = self.fire_spot();
            if t == DRILL_CATCH {
                return Some(Deed::LightFire(spot));
            }
            return (t >= DRILL_SMOKE && t < DRILL_CATCH && t % 8 == 0).then_some(Deed::Smoulder(spot));
        }
        if let Action::Cook { t, food, dish, fire } = self.action {
            let t = t + 1;
            self.action = if t >= COOK_TOTAL { Action::Idle } else { Action::Cook { t, food, dish, fire } };
            if t == COOK_DONE {
                return Some(Deed::Cooked(dish));
            }
            // Smoke and sparks rise off it while it cooks.
            return (t < COOK_DONE && t % 10 == 0).then_some(Deed::Smoulder((fire.0, fire.1 - 4)));
        }
        if let Action::Ponder { t } = self.action {
            self.action = if t + 1 >= PONDER_TOTAL { Action::Idle } else { Action::Ponder { t: t + 1 } };
            return None;
        }
        if let Action::Scoop { t } = self.action {
            let t = t + 1;
            self.action = if t >= SCOOP_TOTAL { Action::Idle } else { Action::Scoop { t } };
            return (t == SCOOP_GRAB).then_some(Deed::DrawWater);
        }
        if let Action::Drink { t, water } = self.action {
            let t = t + 1;
            self.action = if t >= DRINK_TOTAL { Action::Idle } else { Action::Drink { t, water } };
            return (t == DRINK_SIP).then_some(Deed::Drink(water));
        }
        if let Action::Shake { t } = self.action {
            let t = t + 1;
            if t < SHAKE_TOTAL {
                self.action = Action::Shake { t };
                return (t == SHAKE_HIT).then(|| Deed::Shake(self.reach_point()));
            }
            // Keep rocking while A is held.
            let again = input.held(Buttons::A) && can_shake(map, self.reach_point());
            self.action = if again { Action::Shake { t: 0 } } else { Action::Idle };
            return None;
        }
        if !self.wading && input.pressed(Buttons::Y) {
            // Lie down where you stand and sleep.
            self.action = Action::Sleep { t: 0 };
            self.facing = Dir::Right;
            self.moving = false;
            self.walk_timer = 0;
            return None;
        }
        if let Some(tool) = tool.filter(|_| !self.wading && input.pressed(Buttons::B)) {
            // Using a tool roots you in place; change facing first if a
            // direction is held.
            let (ax, ay) = input.axis();
            self.facing = facing_for(self.facing, ax, ay);
            self.action = match tool {
                ItemKind::WoodSpear => Action::Thrust { t: 0 },
                _ => Action::Chop { t: 0 },
            };
            self.moving = false;
            self.walk_timer = 0;
            return None;
        }
        if let Some((ix, iy)) = item_in_reach.filter(|_| input.pressed(Buttons::A)) {
            // Turn toward it unless it's (nearly) underfoot.
            let (fx, fy) = self.body.feet();
            let (dx, dy) = (ix - fx, iy - fy);
            if dx.abs() > 4 || dy.abs() > 4 {
                self.facing = if dx.abs() > dy.abs() {
                    if dx < 0 { Dir::Left } else { Dir::Right }
                } else if dy < 0 {
                    Dir::Up
                } else {
                    Dir::Down
                };
            }
            self.action = Action::PickUp { t: 0 };
            self.moving = false;
            self.walk_timer = 0;
            return None;
        }
        if !self.wading && input.pressed(Buttons::A) {
            // Take hold of a trunk in front (turning toward a held direction).
            let (ax, ay) = input.axis();
            let before = self.facing;
            self.facing = facing_for(self.facing, ax, ay);
            if can_shake(map, self.reach_point()) {
                self.action = Action::Shake { t: 0 };
                self.moving = false;
                self.walk_timer = 0;
                return None;
            }
            let reach = self.reach_point();
            // Vine and bark need cutting: without a knife, stop and think.
            if !has_knife && (can_pull_vine(map, reach) || has_bark(map, reach)) {
                self.ponder();
                return None;
            }
            if can_pull_vine(map, reach) || is_bush(map, reach) || is_broadleaf_trunk(map, reach) {
                self.action = Action::Pull { t: 0 };
                self.moving = false;
                self.walk_timer = 0;
                return None;
            }
            self.facing = before;
        }
        if !self.wading && input.pressed(Buttons::A) {
            // Crouch at the water-drawing spot and scoop some up in a leaf
            // (with nothing to hold it in, stop and think).
            let (rx, ry) = self.reach_point();
            if map.is_drinking_water(rx, ry) && !has_leaf {
                self.ponder();
                return None;
            }
            if map.is_drinking_water(rx, ry) {
                self.action = Action::Scoop { t: 0 };
                self.moving = false;
                self.walk_timer = 0;
                return None;
            }
        }
        self.walk(input, map);
        (self.running && self.walk_timer % DUST_EVERY == 1).then(|| {
            // Just behind the heels, or beside them (one foot, then the
            // other) when running toward the camera, so the body doesn't
            // hide it.
            let (fx, fy) = self.body.feet();
            let side = if (self.walk_timer / DUST_EVERY) % 2 == 0 { -5 } else { 5 };
            Deed::KickDust(match self.facing {
                Dir::Right => (fx - 7, fy),
                Dir::Left => (fx + 7, fy),
                Dir::Up => (fx + side, fy + 4),
                Dir::Down => (fx + side, fy + 1),
            })
        })
    }

    /// The spot just in front of the feet that hands and tools reach: where
    /// the axe blade lands, the trunk being shaken, things picked up.
    pub fn reach_point(&self) -> (i32, i32) {
        let (fx, fy) = self.body.feet();
        match self.facing {
            Dir::Up => (fx, fy - 12),
            Dir::Down => (fx, fy + 10),
            Dir::Left => (fx - 12, fy - 2),
            Dir::Right => (fx + 12, fy - 2),
        }
    }

    /// The ground the spear point sweeps through when it jabs out: from
    /// in front of the feet to the tip at full stretch.
    fn spear_line(&self) -> ((i32, i32), (i32, i32)) {
        let (fx, fy) = self.body.feet();
        match self.facing {
            Dir::Up => ((fx + 4, fy - 8), (fx + 4, fy - 24)),
            Dir::Down => ((fx + 4, fy + 4), (fx + 4, fy + 18)),
            Dir::Left => ((fx - 8, fy - 2), (fx - 30, fy - 2)),
            Dir::Right => ((fx + 8, fy - 2), (fx + 30, fy - 2)),
        }
    }

    fn walk(&mut self, input: &InputState, map: &WorldMap) {
        let (ax, ay) = input.axis();
        self.moving = ax != 0 || ay != 0;
        self.facing = facing_for(self.facing, ax, ay);

        let (fx, fy) = self.body.feet();
        self.wading = map.props_at_px(fx, fy).wading;

        // Water drags at the legs too much to run.
        let was_running = self.running;
        self.running = self.moving && !self.wading && input.held(Buttons::R);
        if self.running != was_running {
            self.walk_timer = 0;
        }
        let diagonal = ax != 0 && ay != 0;
        let speed = match (self.wading, self.running, diagonal) {
            (true, _, false) => SPEED_WADE,
            (true, _, true) => SPEED_WADE_DIAG,
            (false, true, false) => SPEED_RUN,
            (false, true, true) => SPEED_RUN_DIAG,
            (false, false, false) => SPEED_WALK,
            (false, false, true) => SPEED_WALK_DIAG,
        };
        self.body.move_by(ax * speed, ay * speed, CORNER_ASSIST_PX, &|r| map.is_solid_rect(r));

        // Keep walking in place against walls, like classic action RPGs.
        self.walk_timer = if self.moving { self.walk_timer + 1 } else { 0 };
    }

    pub fn is_asleep(&self) -> bool {
        matches!(self.action, Action::Sleep { .. })
    }

    /// Wakes from sleep: sitting up, getting up and looking round.
    pub fn wake(&mut self) {
        if self.is_asleep() {
            self.action = Action::WakeUp { t: WAKE_SIT, end: WAKE_FROM_SLEEP_END };
        }
    }

    /// Standing ready: not mid-action (so the bag can be opened).
    /// Stops to think (something here needs what isn't carried).
    fn ponder(&mut self) {
        self.action = Action::Ponder { t: 0 };
        self.moving = false;
        self.walk_timer = 0;
    }

    pub fn is_idle(&self) -> bool {
        self.action == Action::Idle
    }

    /// Starts eating or drinking something taken from the bag.
    pub fn consume(&mut self, kind: ItemKind) {
        self.action = match kind.usage() {
            Some(Use::Eat) => Action::Eat { t: 0, food: kind },
            Some(Use::Drink) => Action::Drink { t: 0, water: kind },
            Some(Use::Light) => Action::Drill { t: 0 },
            None => return,
        };
        self.moving = false;
        self.walk_timer = 0;
    }

    /// Kneels facing the fire at `fire` (ground px) to cook `food` into
    /// `dish`.
    pub fn cook(&mut self, food: ItemKind, dish: ItemKind, fire: (i32, i32)) {
        let (fx, fy) = self.body.feet();
        let (dx, dy) = (fire.0 - fx, fire.1 - fy);
        self.facing = if dx.abs() > dy.abs() {
            if dx < 0 { Dir::Left } else { Dir::Right }
        } else if dy < 0 {
            Dir::Up
        } else {
            Dir::Down
        };
        self.action = Action::Cook { t: 0, food, dish, fire };
        self.moving = false;
        self.walk_timer = 0;
    }

    /// Where a fire is lit: a step ahead, clear of the castaway.
    pub fn fire_spot(&self) -> (i32, i32) {
        let (fx, fy) = self.body.feet();
        match self.facing {
            Dir::Up => (fx, fy - 14),
            Dir::Down => (fx, fy + 12),
            Dir::Left => (fx - 16, fy),
            Dir::Right => (fx + 16, fy),
        }
    }

    /// Where something put down lands: just in front of the feet.
    pub fn front_of_feet(&self) -> (i32, i32) {
        let (fx, fy) = self.body.feet();
        match self.facing {
            Dir::Up => (fx, fy - 6),
            Dir::Down => (fx, fy + 6),
            Dir::Left => (fx - 9, fy),
            Dir::Right => (fx + 9, fy),
        }
    }

    /// Point the camera follows (roughly the torso).
    pub fn center(&self) -> (i32, i32) {
        let (x, y) = self.body.px();
        (x + SPRITE_W / 2, y + SPRITE_H / 2 + 2)
    }

    pub fn depth(&self) -> i32 {
        self.body.px().1 + SPRITE_H
    }

    /// `in_front` raises the castaway above high-priority BG2 tiles (palm
    /// canopies) when standing in front of them; otherwise canopies cover them.
    pub fn draw(&self, list: &mut DrawList, camera: &Camera, sprites: &SpriteIds, clock: u32, shadows: bool, in_front: bool) {
        let prio = if in_front { FRONT_PRIORITY } else { ACTOR_PRIORITY };
        let (wx, wy) = self.body.px();
        let (sx, sy) = camera.to_screen(wx, wy);
        let depth = self.depth();
        let step_frames = if self.wading {
            STEP_FRAMES_WADE
        } else if self.running {
            STEP_FRAMES_RUN
        } else {
            STEP_FRAMES_WALK
        };
        // Start on a step frame so movement reads instantly.
        let phase = if self.moving { (self.walk_timer / step_frames + 1) % 4 } else { 0 };
        let (view, flip) = view_for(self.facing);
        // The 24x32 character canvas is offset from the 16x24 logical frame.
        let (cx, cy) = (sx - CHAR_ORIGIN.0, sy - CHAR_ORIGIN.1);

        if let Action::WakeUp { t, .. } = self.action {
            if t < WAKE_KNEEL {
                self.draw_waking(list, (sx, sy), (cx, cy), depth, sprites, t, prio, shadows);
                return;
            }
        }

        if let Action::Sleep { t } = self.action {
            // Curled up asleep, Zs drifting up from the head; under a
            // shelter only the shelter shows, the Zs rising from its ridge.
            let (lx, ly) = (sx - 15, sy + 6);
            let (hx, hy) = if self.sheltered {
                list.actor(depth, ObjEntry::new(sx - 12, sy, sprites.lean_to, OBJ_FX, prio));
                (sx + 6, sy - 2)
            } else {
                list.actor(depth, ObjEntry::new(lx, ly, sprites.player.sleeping, OBJ_PLAYER, prio));
                (sx - 10, sy + 4)
            };
            if shadows {
                list.shadow(ObjEntry::new(lx, sy + 15, sprites.shadow_lying, OBJ_SHADOW, ACTOR_PRIORITY));
            }
            for k in 0..2u32 {
                let age = (t + k * SNORE_EVERY / 2) % SNORE_EVERY;
                if t < SLEEP_SETTLE + k * SNORE_EVERY / 2 {
                    continue;
                }
                let rise = (age / 3) as i32;
                let sway = if (age / 8) % 2 == 0 { 0 } else { 1 };
                list.actor(depth + 1, ObjEntry::new(hx + rise / 2 + sway, hy - rise, sprites.snore, OBJ_FX, prio));
            }
            return;
        }

        if let Action::Chop { t } = self.action {
            self.draw_chop(list, (sx, sy), (cx, cy), depth, sprites, t, prio);
            if shadows {
                list.shadow(ObjEntry::new(sx, sy + SPRITE_H - 5, sprites.shadow_player, OBJ_SHADOW, ACTOR_PRIORITY));
            }
            return;
        }

        if let Action::Thrust { t } = self.action {
            self.draw_thrust(list, (cx, cy), depth, sprites, t, prio);
            if shadows {
                list.shadow(ObjEntry::new(sx, sy + SPRITE_H - 5, sprites.shadow_player, OBJ_SHADOW, ACTOR_PRIORITY));
            }
            return;
        }

        if let Action::PickUp { t } = self.action {
            // Crouched for the middle of the move, standing at either end.
            let low = (2..PICK_TOTAL - 3).contains(&t);
            let gfx = if low { sprites.player.crouch[view] } else { sprites.player.walk[view][0].full };
            list.actor(depth, ObjEntry::new(cx, cy, gfx, OBJ_PLAYER, prio).flipped(flip));
            if shadows {
                list.shadow(ObjEntry::new(sx, sy + SPRITE_H - 5, sprites.shadow_player, OBJ_SHADOW, ACTOR_PRIORITY));
            }
            return;
        }

        if let Action::Pull { t } = self.action {
            // Lean in and grip, then throw the weight back as it tears free.
            let stage = (t >= PULL_YANK) as usize;
            list.actor(depth, ObjEntry::new(cx, cy, sprites.player.shake[view][stage], OBJ_PLAYER, prio).flipped(flip));
            if shadows {
                list.shadow(ObjEntry::new(sx, sy + SPRITE_H - 5, sprites.shadow_player, OBJ_SHADOW, ACTOR_PRIORITY));
            }
            return;
        }

        if let Action::Eat { t, food } = self.action {
            self.draw_eating(list, (sx, sy), (cx, cy), depth, sprites, t, food, prio, shadows);
            return;
        }

        if let Action::Drink { t, .. } = self.action {
            self.draw_drinking(list, (cx, cy), depth, sprites, t, prio);
            if shadows {
                list.shadow(ObjEntry::new(sx, sy + SPRITE_H - 5, sprites.shadow_player, OBJ_SHADOW, ACTOR_PRIORITY));
            }
            return;
        }

        if let Action::Drill { t } = self.action {
            // Kneeling over the drill, sawing its bow from side to side;
            // it's behind the body when facing away.
            let body = ObjEntry::new(cx, cy, sprites.player.crouch[view], OBJ_PLAYER, prio).flipped(flip);
            let (dx, dy) = self.front_of_feet();
            let (fx, fy) = self.body.feet();
            let saw = if t < DRILL_CATCH && t / DRILL_STROKE % 2 == 0 { 1 } else { -1 };
            let drill = ObjEntry::new(sx + 8 + (dx - fx) - 8 + saw, sy + SPRITE_H + (dy - fy) - 13, sprites.fire_drill, OBJ_FX, prio);
            if self.facing == Dir::Up {
                list.actor(depth, body);
                list.actor(depth, drill);
            } else {
                list.actor(depth, drill);
                list.actor(depth, body);
            }
            if shadows {
                list.shadow(ObjEntry::new(sx, sy + SPRITE_H - 5, sprites.shadow_player, OBJ_SHADOW, ACTOR_PRIORITY));
            }
            return;
        }

        if let Action::Cook { t, food, fire, .. } = self.action {
            self.draw_cooking(list, camera, (cx, cy), depth, sprites, (t, food, fire), prio);
            if shadows {
                list.shadow(ObjEntry::new(sx, sy + SPRITE_H - 5, sprites.shadow_player, OBJ_SHADOW, ACTOR_PRIORITY));
            }
            return;
        }

        if let Action::Ponder { t } = self.action {
            // Hand to chin, a thought bubble filling with dots above.
            list.actor(depth, ObjEntry::new(cx, cy, sprites.player.drink[view], OBJ_PLAYER, prio).flipped(flip));
            let dots = ((t / PONDER_DOT) as usize).min(2);
            let bx = if flip { sx - 12 } else { sx + 12 };
            list.actor(depth + 1, ObjEntry::new(bx, sy - 18, sprites.thought[dots], OBJ_FX, prio).flipped(flip));
            if shadows && !self.wading {
                list.shadow(ObjEntry::new(sx, sy + SPRITE_H - 5, sprites.shadow_player, OBJ_SHADOW, ACTOR_PRIORITY));
            }
            return;
        }

        if let Action::Scoop { t } = self.action {
            let p = &sprites.player;
            let gfx = if (3..SCOOP_RISE).contains(&t) { p.crouch[view] } else { p.walk[view][0].full };
            list.actor(depth, ObjEntry::new(cx, cy, gfx, OBJ_PLAYER, prio).flipped(flip));
            if shadows && !self.wading {
                list.shadow(ObjEntry::new(sx, sy + SPRITE_H - 5, sprites.shadow_player, OBJ_SHADOW, ACTOR_PRIORITY));
            }
            return;
        }

        if let Action::Shake { t } = self.action {
            let stage = (t / SHAKE_STEP % 2) as usize;
            list.actor(depth, ObjEntry::new(cx, cy, sprites.player.shake[view][stage], OBJ_PLAYER, prio).flipped(flip));
            if shadows {
                list.shadow(ObjEntry::new(sx, sy + SPRITE_H - 5, sprites.shadow_player, OBJ_SHADOW, ACTOR_PRIORITY));
            }
            return;
        }

        if matches!(self.action, Action::WakeUp { t, .. } if t < WAKE_STAND) {
            list.actor(depth, ObjEntry::new(cx, cy, sprites.player.kneeling, OBJ_PLAYER, prio));
            if shadows {
                list.shadow(ObjEntry::new(sx, sy + SPRITE_H - 5, sprites.shadow_player, OBJ_SHADOW, ACTOR_PRIORITY));
            }
            return;
        }
        let frame = sprites.player.walk[view][phase as usize];
        if self.wading {
            // Foam ring first (in front), then the body above the waterline,
            // then the submerged legs in a blended palette.
            let ripple = sprites.ripples[(clock / 10) as usize % sprites.ripples.len()];
            list.actor(depth, ObjEntry::new(sx - 4, sy + 14, ripple, OBJ_WATER_FX, prio));
            list.actor(depth, ObjEntry::new(cx, cy, frame.top, OBJ_PLAYER, prio).flipped(flip));
            list.actor(depth, ObjEntry::new(cx, cy, frame.legs, OBJ_WADE, prio).flipped(flip));
        } else {
            list.actor(depth, ObjEntry::new(cx, cy, frame.full, OBJ_PLAYER, prio).flipped(flip));
            if shadows {
                list.shadow(ObjEntry::new(sx, sy + SPRITE_H - 5, sprites.shadow_player, OBJ_SHADOW, ACTOR_PRIORITY));
            }
        }
    }
}

impl Player {
    /// Lying, stirring and sitting up (kneeling and standing are drawn as
    /// regular frames once the castaway is on their feet).
    #[allow(clippy::too_many_arguments)]
    fn draw_waking(&self, list: &mut DrawList, (sx, sy): (i32, i32), (cx, cy): (i32, i32), depth: i32, sprites: &SpriteIds, t: u32, prio: u8, shadows: bool) {
        let p = &sprites.player;
        // The lying body is placed so its hips line up with the sitting pose.
        let (lx, ly) = (sx - 15, sy + 6);
        if t < WAKE_SIT {
            // Eyes flutter open, blink once, then stay open.
            let eyes_open = t >= WAKE_STIR && !(WAKE_STIR + 15..WAKE_STIR + 23).contains(&t);
            list.actor(depth, ObjEntry::new(lx, ly, p.lying[eyes_open as usize], OBJ_PLAYER, prio));
        } else {
            list.actor(depth, ObjEntry::new(cx, cy, p.sitting, OBJ_PLAYER, prio));
        }
        if shadows {
            list.shadow(ObjEntry::new(lx, sy + 15, sprites.shadow_lying, OBJ_SHADOW, ACTOR_PRIORITY));
        }
    }

    /// Raising a handful of water to the mouth and gulping it down. The
    /// water is a translucent sprite in the hands until it's drunk.
    fn draw_drinking(&self, list: &mut DrawList, (cx, cy): (i32, i32), depth: i32, sprites: &SpriteIds, t: u32, prio: u8) {
        let (view, flip) = view_for(self.facing);
        let raised = (DRINK_RAISE..DRINK_TOTAL - 5).contains(&t);
        let gfx = if raised { sprites.player.drink[view] } else { sprites.player.walk[view][0].full };
        let body = ObjEntry::new(cx, cy, gfx, OBJ_PLAYER, prio).flipped(flip);
        let behind = self.facing == Dir::Up;
        if behind {
            list.actor(depth, body);
        }
        if raised && t < DRINK_SIP {
            let (gx, gy) = crate::assets::character::pose::drink(VIEWS[view]).grip();
            let gx = if flip { CHAR_W as f32 - gx } else { gx };
            // Spills over as it's tipped back.
            let frame = (t >= DRINK_SIP - 6) as usize;
            list.actor(depth, ObjEntry::new(cx + gx.round() as i32 - 4, cy + gy.round() as i32 - 4, sprites.handful[frame], OBJ_WATER_FX, prio).flipped(flip));
        }
        if !behind {
            list.actor(depth, body);
        }
    }

    /// Holding food up to the mouth and gnawing at it: the food sprite sits
    /// in the hands of the drinking pose and bobs as they chew (a coconut
    /// shrinks a mouthful at a time). From behind, it is hidden by the head.
    #[allow(clippy::too_many_arguments)]
    fn draw_eating(&self, list: &mut DrawList, (sx, sy): (i32, i32), (cx, cy): (i32, i32), depth: i32, sprites: &SpriteIds, t: u32, food: ItemKind, prio: u8, shadows: bool) {
        let (view, flip) = view_for(self.facing);
        let raised = (EAT_RAISE..EAT_TOTAL - 6).contains(&t);
        let (body, pose) = if raised {
            (sprites.player.drink[view], crate::assets::character::pose::drink(VIEWS[view]))
        } else {
            (sprites.player.walk[view][0].full, crate::assets::character::pose::walk(VIEWS[view], 0))
        };
        let (gx, gy) = pose.grip();
        let gx = if flip { CHAR_W as f32 - gx } else { gx };
        // Each bite pulls the coconut away from the mouth for a moment,
        // and it shrinks a mouthful at a time.
        let bites = EAT_BITES.iter().filter(|&&b| t >= b).count();
        let tug = EAT_BITES.iter().any(|&b| (b..b + 4).contains(&t)) as i32;
        let (gfx, half_w) = match food {
            ItemKind::Crab => (sprites.crab_caught, 8),
            ItemKind::GrilledCrab => (sprites.crab_grilled, 8),
            _ if bites == 0 => (sprites.opened_coconut, 4),
            _ => (sprites.coconut_bitten[bites - 1], 4),
        };
        let nut = ObjEntry::new(cx + gx.round() as i32 - half_w, cy + gy.round() as i32 - 5 + tug, gfx, OBJ_CRAB, prio).flipped(flip);
        let nut = raised.then_some(nut);
        let body = ObjEntry::new(cx, cy, body, OBJ_PLAYER, prio).flipped(flip);
        let behind = self.facing == Dir::Up;
        if behind {
            list.actor(depth, body);
        }
        if let Some(nut) = nut {
            list.actor(depth, nut);
        }
        if !behind {
            list.actor(depth, body);
        }
        if shadows {
            list.shadow(ObjEntry::new(sx, sy + SPRITE_H - 5, sprites.shadow_player, OBJ_SHADOW, ACTOR_PRIORITY));
        }
    }

    /// A horizontal swing at trunk height: wound up to one side, sweeping
    /// past the body, then striking forward. The axe is a separate sprite
    /// placed so its handle sits in the pose's hands; when it is in front of
    /// the body the hands are overlaid on the handle.
    #[allow(clippy::too_many_arguments)]
    fn draw_chop(&self, list: &mut DrawList, (sx, sy): (i32, i32), (cx, cy): (i32, i32), depth: i32, sprites: &SpriteIds, t: u32, prio: u8) {
        let stage = if t >= CHOP_STRIKE {
            2
        } else if t >= CHOP_SWEEP {
            1
        } else {
            0
        };
        let (view, flip) = view_for(self.facing);
        let frame = sprites.player.chop[view][stage];
        let pose = &sprites.player.chop_poses[view][stage];

        if (CHOP_SWEEP..CHOP_STRIKE + SWOOSH_FRAMES).contains(&t) {
            let (ox, oy) = match self.facing {
                Dir::Right => (-6, 8),
                Dir::Left => (-10, 8),
                Dir::Down => (-8, 10),
                Dir::Up => (-8, -4),
            };
            let swoosh = sprites.swoosh[self.facing as usize];
            list.actor(depth, ObjEntry::new(sx + ox, sy + oy, swoosh, OBJ_WATER_FX, prio));
        }

        let body = ObjEntry::new(cx, cy, frame.full, OBJ_PLAYER, prio).flipped(flip);
        let Some(hold) = pose.tool else {
            list.actor(depth, body);
            return;
        };
        let (gfx, end) = axe_art(sprites, hold.art);
        let (fh, fv) = (hold.flip_h ^ flip, hold.flip_v);
        // Handle end in the axe image, after flips.
        let end = (if fh { 15.0 - end.0 } else { end.0 }, if fv { 15.0 - end.1 } else { end.1 });
        let (gx, gy) = pose.grip();
        let gx = if flip { CHAR_W as f32 - gx } else { gx };
        let mut axe = ObjEntry::new(cx + (gx - end.0).round() as i32, cy + (gy - end.1).round() as i32, gfx, OBJ_FX, prio).flipped(fh);
        axe.flip_v = fv;

        if hold.front {
            list.actor(depth, ObjEntry::new(cx, cy, frame.hands, OBJ_PLAYER, prio).flipped(flip));
            list.actor(depth, axe);
            list.actor(depth, body);
        } else {
            list.actor(depth, body);
            list.actor(depth, axe);
        }
    }
}

impl Player {
    /// Kneeling by the fire with the food held out toward the flames at
    /// arm's length, bobbing as it's turned. Crabs are turned over now and
    /// then; a pot is held steady. Facing away, it's hidden by the body.
    #[allow(clippy::too_many_arguments)]
    fn draw_cooking(&self, list: &mut DrawList, camera: &Camera, (cx, cy): (i32, i32), depth: i32, sprites: &SpriteIds, (t, food, fire): (u32, ItemKind, (i32, i32)), prio: u8) {
        let (view, flip) = view_for(self.facing);
        list.actor(depth, ObjEntry::new(cx, cy, sprites.player.crouch[view], OBJ_PLAYER, prio).flipped(flip));
        let (fx, fy) = self.body.feet();
        let (dx, dy) = match self.facing {
            Dir::Up => (0, -10),
            Dir::Down => (0, 8),
            Dir::Left => (-13, 0),
            Dir::Right => (13, 0),
        };
        let (ix, iy) = (fx + dx, fy + dy - 11);
        let bob = ((t / 6) % 2) as i32;
        let (gfx, w, palette) = match food {
            ItemKind::Crab => (sprites.crab_caught, 16, OBJ_CRAB),
            _ => (sprites.bark_pot, 8, OBJ_FX),
        };
        let turned = food == ItemKind::Crab && (t / COOK_TURN) % 2 == 1;
        let (sx, sy) = camera.to_screen(ix - w / 2, iy - 4 + bob);
        // In front of the fire, and of the body unless facing away.
        let item = ObjEntry::new(sx, sy, gfx, palette, prio).flipped(turned);
        let depth = if self.facing == Dir::Up { depth - 1 } else { depth.max(fire.1) + 1 };
        list.actor(depth, item);
    }

    /// A jab with the spear: drawn back, then lunging with the point out.
    /// The spear sprite is placed so the hands hold it 8px from the butt;
    /// facing away it's behind the body, otherwise in front with the hands
    /// drawn over it.
    fn draw_thrust(&self, list: &mut DrawList, (cx, cy): (i32, i32), depth: i32, sprites: &SpriteIds, t: u32, prio: u8) {
        let stage = (THRUST_JAB..THRUST_BACK).contains(&t) as usize;
        let (view, flip) = view_for(self.facing);
        let frame = sprites.player.thrust[view][stage];
        let (gx, gy) = sprites.player.thrust_poses[view][stage].grip();
        let gx = if flip { CHAR_W as f32 - gx } else { gx };
        // Spear image and where the hands hold it in that image.
        let (gfx, (hx, hy)) = match self.facing {
            Dir::Right => (sprites.spear_side, (8.0, 3.5)),
            Dir::Left => (sprites.spear_side, (23.0, 3.5)),
            Dir::Down => (sprites.spear_down, (3.5, 8.0)),
            Dir::Up => (sprites.spear_up, (3.5, 23.0)),
        };
        let spear = ObjEntry::new(cx + (gx - hx).round() as i32, cy + (gy - hy).round() as i32, gfx, OBJ_FX, prio).flipped(self.facing == Dir::Left);
        let body = ObjEntry::new(cx, cy, frame.full, OBJ_PLAYER, prio).flipped(flip);
        if self.facing == Dir::Up {
            list.actor(depth, body);
            list.actor(depth, spear);
        } else {
            list.actor(depth, ObjEntry::new(cx, cy, frame.hands, OBJ_PLAYER, prio).flipped(flip));
            list.actor(depth, spear);
            list.actor(depth, body);
        }
    }
}

/// The axe image for a hold, and where its handle end sits in it (unflipped).
fn axe_art(sprites: &SpriteIds, art: AxeArt) -> (SpriteId, (f32, f32)) {
    match art {
        AxeArt::Side => (sprites.axe_side, (1.5, 9.0)),
        AxeArt::Up => (sprites.axe_up, (9.0, 14.0)),
        AxeArt::Diag => (sprites.axe_diag, (4.0, 12.0)),
    }
}

/// Character view index (Front, Back, Side) and whether to mirror it.
fn view_for(facing: Dir) -> (usize, bool) {
    match facing {
        Dir::Down => (0, false),
        Dir::Up => (1, false),
        Dir::Right => (2, false),
        Dir::Left => (2, true),
    }
}

/// Keeps the current facing while it's still one of the held directions,
/// so diagonal movement doesn't make the sprite jitter.
fn facing_for(current: Dir, ax: i32, ay: i32) -> Dir {
    let held = |d: Dir| match d {
        Dir::Up => ay < 0,
        Dir::Down => ay > 0,
        Dir::Left => ax < 0,
        Dir::Right => ax > 0,
    };
    if held(current) {
        return current;
    }
    match (ax, ay) {
        (x, _) if x < 0 => Dir::Left,
        (x, _) if x > 0 => Dir::Right,
        (_, y) if y < 0 => Dir::Up,
        (_, y) if y > 0 => Dir::Down,
        _ => current,
    }
}

use super::body::Body;
use crate::assets::character::pose::AxeArt;
use crate::assets::character::{CHAR_ORIGIN, CHAR_W};
use crate::assets::palettes::{OBJ_CRAB, OBJ_FX, OBJ_PLAYER, OBJ_SHADOW, OBJ_WADE, OBJ_WATER_FX};
use crate::assets::sprite_art::TORCH_GRIP;
use crate::assets::{SpriteIds, VIEWS};
use crate::common::input::{Buttons, InputState};
use crate::common::math::{Dir, Rect};
use crate::items::{ItemKind, Use};
use crate::ppu::banks::SpriteId;
use crate::ppu::oam::ObjEntry;
use crate::render::camera::Camera;
use crate::render::sprites::DrawList;
use crate::world::harvest::{Diggable, can_pull_vine, can_shake, diggable, has_bark, is_bamboo, is_broadleaf_trunk, is_bush};
use crate::world::map::{BridgeSpan, LadderFit};
use crate::world::{TILE_SIZE, WorldMap};

pub const SPRITE_W: i32 = 16;
pub const SPRITE_H: i32 = 24;
/// Feet-only collision box (top-down games collide at the feet so heads can
/// overlap things above them).
const HITBOX: Rect = Rect::new(3, 18, 10, 6);
/// Speeds in 8.8 fixed point per frame.
const SPEED_WALK: i32 = 336; // ~1.3 px
const SPEED_WALK_DIAG: i32 = 238;
/// Up and down a ladder: half walking pace.
const SPEED_CLIMB: i32 = 168;
/// Running (R held), not possible while wading.
const SPEED_RUN: i32 = 560; // ~2.2 px
const SPEED_RUN_DIAG: i32 = 396;
const SPEED_WADE: i32 = 184;
const SPEED_WADE_DIAG: i32 = 130;
/// Too weak from hunger: the pace slowed to this share (in quarters), and
/// the steps with it.
const WEAK_PACE_QUARTERS: i32 = 3;
const CORNER_ASSIST_PX: i32 = 6;
/// Frames per walk-animation step.
const STEP_FRAMES_WALK: u32 = 8;
const STEP_FRAMES_WADE: u32 = 12;
const STEP_FRAMES_RUN: u32 = 5;
/// Frames per hand-over-hand step up a ladder.
const STEP_FRAMES_CLIMB: u32 = 10;
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
/// Digging (frames): crouching to drive the tool in twice (dirt flying at
/// each), lifting out what's loosened, and standing.
const DIG_STABS: [u32; 2] = [8, 22];
const DIG_LIFT: u32 = 32;
const DIG_TOTAL: u32 = 44;

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
/// Casting (frames): drawn back, then the line flies out at
/// `CAST_RELEASE`; and how far out the float lands (px).
const CAST_RELEASE: u32 = 8;
const CAST_TOTAL: u32 = 14;
const CAST_REACH: i32 = 30;
/// How long the "!" of a recipe coming to mind shows, and the frames it
/// takes to pop up.
const EUREKA_FRAMES: u32 = 50;
const EUREKA_POP: u32 = 4;
/// Frames per flicker of a held torch's flame.
const TORCH_FLAME_FRAME: u32 = 6;
/// How far out from the hand a held torch stands (px, screen-right facing
/// toward or away, forward from the side), by view.
const TORCH_REACH: [f32; 3] = [3.0, 3.0, 8.0];
/// How much lower than the hand's grip a held torch sits, by view: from
/// the side it's carried out in front at shoulder height.
const TORCH_DROP: [f32; 3] = [0.0, 0.0, 4.0];

/// Lighting a fire (frames): kneeling and sawing the drill's bow back and
/// forth (one stroke every `DRILL_STROKE`), smoke from `DRILL_SMOKE`, until
/// the tinder catches on `DRILL_CATCH`.
const DRILL_STROKE: u32 = 6;
const DRILL_SMOKE: u32 = 50;
const DRILL_CATCH: u32 = 100;
const DRILL_TOTAL: u32 = 112;
/// How near (px) the reach point must come to the middle of a signal
/// fire's stones to light it (about the ring as drawn, which is wider than
/// its cell), or the feet when standing over it.
const SIGNAL_REACH: i32 = 16;
const SIGNAL_UNDERFOOT: i32 = 12;

/// The middle of the stones of a signal fire in the cell (world px): a
/// little below the cell's middle, where the ring is drawn.
fn signal_fire_middle((x, y): (i32, i32)) -> (i32, i32) {
    (x * TILE_SIZE + TILE_SIZE / 2, y * TILE_SIZE + TILE_SIZE / 2 + 2)
}

/// Cooking over a fire (frames): kneeling with the food held over the
/// flames (turned every `COOK_TURN`) until it's done on `COOK_DONE`.
const COOK_TURN: u32 = 24;
const COOK_DONE: u32 = 150;
const COOK_TOTAL: u32 = 162;

/// Pulling a vine off its stake (frames): take hold, then yank it free on
/// `PULL_YANK`.
const PULL_YANK: u32 = 14;
const PULL_TOTAL: u32 = 26;

/// Standing a ladder against a cliff (frames): lift it, and it's set
/// against the rock on `PITCH_SET`.
/// Longest (frames) stepping over to where a ladder goes may take.
const TO_LADDER_FRAMES: u32 = 90;
const PITCH_SET: u32 = 12;
/// Longest the walk over to set a wood lathe may take (frames).
const TO_LATHE_FRAMES: u32 = 240;
const PITCH_TOTAL: u32 = 24;
/// Trying to scramble up a rock face too tall to climb bare-handed
/// (frames): two goes, each `SCRAMBLE_TRY` long, hauling up and sliding
/// back; then stopping to think.
const SCRAMBLE_TRY: u32 = 24;
/// Coming upon a place (the mill site, the river's narrow): standing struck
/// by an idea (a "!" over the head) until `SURVEY_STRUCK`, then walking
/// straight to the bank for at most `SURVEY_WALK`.
const SURVEY_STRUCK: u32 = 30;
const SURVEY_WALK: u32 = 240;
/// Noticing something (frames): looking up until `NOTICE_LEFT`, left until
/// `NOTICE_RIGHT`, right until `NOTICE_BACK`, up again until
/// `NOTICE_LOOKED`; then at most `NOTICE_APPROACH` walking up to the rock.
const NOTICE_LEFT: u32 = 30;
const NOTICE_RIGHT: u32 = 60;
const NOTICE_BACK: u32 = 90;
const NOTICE_LOOKED: u32 = 110;
const NOTICE_APPROACH: u32 = 60;
const SCRAMBLE_TOTAL: u32 = 2 * SCRAMBLE_TRY;

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
    Ponder {
        t: u32,
    },
    /// Swinging the axe; `t` counts frames since the swing started.
    Chop {
        t: u32,
    },
    /// Jabbing with the spear.
    Thrust {
        t: u32,
    },
    /// Crouched over the ground at `at` (world px), digging with the tool
    /// in hand.
    Dig {
        t: u32,
        at: (i32, i32),
    },
    /// Coming to on the beach; `t` counts frames since the game started,
    /// and it's over at `end`. Waking from sleep picks this up from sitting
    /// up and ends once standing, without looking around.
    WakeUp {
        t: u32,
        end: u32,
    },
    /// Lying asleep; `t` counts frames since lying down.
    Sleep {
        t: u32,
    },
    /// Gripping a trunk and rocking it; `t` counts frames in this round.
    Shake {
        t: u32,
    },
    /// Crouching to pick something up.
    PickUp {
        t: u32,
    },
    /// Scooping up water to carry (raw water).
    Scoop {
        t: u32,
    },
    /// Drinking water from the bag.
    Drink {
        t: u32,
        water: ItemKind,
    },
    /// Eating something from the bag.
    Eat {
        t: u32,
        food: ItemKind,
    },
    /// Kneeling to drill a fire: on the ground ahead, or (`signal`, its
    /// cell) in the signal fire there.
    Drill {
        t: u32,
        signal: Option<(i32, i32)>,
    },
    /// Kneeling by the fire at `fire` (ground px), holding `food` over it
    /// until it becomes `dish`.
    Cook {
        t: u32,
        food: ItemKind,
        dish: ItemKind,
        fire: (i32, i32),
    },
    /// Tugging a vine off a climbing plant, or leaves off a bush.
    Pull {
        t: u32,
    },
    /// Casting the line out over the water.
    Cast {
        t: u32,
    },
    /// Holding the rod out with the float on the water, waiting for a bite.
    Fishing {
        t: u32,
    },
    /// Grabbing at a low rock face and trying to haul up it, sliding back
    /// down; it ends in thought.
    Scramble {
        t: u32,
    },
    /// Stepping sideways to stand under the cliff face whose foot is at
    /// `foot` (a cell), to pitch a ladder there (`Pitch`); `t` counts
    /// frames.
    ToLadder {
        t: u32,
        foot: (i32, i32),
        rows: i32,
    },
    /// Standing a ladder against the cliff face whose foot is at `foot`
    /// (a cell), `rows` cells tall.
    Pitch {
        t: u32,
        foot: (i32, i32),
        rows: i32,
    },
    /// Stepping along the bank to the row a log bridge goes across
    /// (`span`), to lay it there (`Lay`), facing `across`.
    ToBridge {
        t: u32,
        span: BridgeSpan,
        across: Dir,
    },
    /// Laying a log bridge across the river along `span`.
    Lay {
        t: u32,
        span: BridgeSpan,
        across: Dir,
    },
    /// Stepping along the bank to the mill site's row (`site`, a cell),
    /// to set the water wheel in the river there (`Mount`).
    ToWheel {
        t: u32,
        site: (i32, i32),
    },
    /// Setting the water wheel in the river off the mill site's bank.
    Mount {
        t: u32,
        site: (i32, i32),
    },
    /// Walking over to the place to stand to set the wood lathe beside
    /// the water wheel off the mill site's bank `site` (`Install`).
    ToLathe {
        t: u32,
        site: (i32, i32),
    },
    /// Setting the wood lathe on the bank beside the water wheel.
    Install {
        t: u32,
        site: (i32, i32),
    },
    /// Setting a signal fire down on the ground ahead (its cell).
    Build {
        t: u32,
        cell: (i32, i32),
    },
    /// Come upon a place by the river (the mill site, the narrow): struck
    /// by an idea, then walking straight over to its bank (`to`,
    /// a cell) and turning `across` the river; then, if `think`, stopping
    /// to think.
    Survey {
        t: u32,
        to: (i32, i32),
        across: Dir,
        think: bool,
    },
    /// Something found has caught the eye (the pine cone fallen at the
    /// foot of the cliff's recess): looking up, then round about as on
    /// first coming to; then, if `climb`, stepping up to the rock to try
    /// scrambling up it.
    Notice {
        t: u32,
        climb: bool,
    },
    /// Fallen where they stood, health spent; there's no getting up.
    Collapsed,
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
    /// Fish are swimming in the water just ahead, and there's no rod to
    /// catch them with.
    pub fish_near: bool,
    /// A goes to the handcart (it's being pulled, or set down just ahead):
    /// a hint about the place doesn't take it.
    pub cart_ahead: bool,
    /// Carrying what it takes to light a signal fire: the fire drill, a
    /// branch and a palm frond.
    pub kindling: bool,
}

/// Something the castaway did to the world this frame, at a world pixel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Deed {
    /// An axe blow connected.
    Chop((i32, i32)),
    /// What the tool in hand loosened at the point is lifted out.
    Dig((i32, i32)),
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
    /// The float landed on the water at the point (world px).
    Cast((i32, i32)),
    /// The line was struck (B while fishing): a fish biting is caught.
    Strike,
    /// The line was drawn in without striking (moving off).
    ReelIn,
    /// A ladder was stood against the face whose foot is at the cell,
    /// that many cells tall.
    StandLadder((i32, i32), i32),
    /// A log bridge was laid across the river there.
    LayBridge(BridgeSpan),
    /// The water wheel was set in the river off the mill site's bank
    /// (the cell).
    SetWheel((i32, i32)),
    /// The wood lathe was set on the bank beside the water wheel off the
    /// mill site's bank (the cell).
    SetLathe((i32, i32)),
    /// A signal fire was set down in the cell.
    SetSignalFire((i32, i32)),
    /// The signal fire in the cell caught.
    LightSignalFire((i32, i32)),
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
    /// A lit torch is carried (`Some(true)` once it's burning low): it's
    /// held up while standing and walking.
    pub torch: Option<bool>,
    /// Weak with hunger: no running, and walking slower.
    pub weak: bool,
    /// Fit to run (not starving, nor kept from it by an ailment).
    pub can_run: bool,
    /// An upset stomach: pale, a hand held to the belly.
    pub queasy: bool,
    /// On a ladder: moving only up and down it, slowly, facing the rock.
    pub climbing: bool,
    /// Pulling the handcart: no running (walking is no slower), and not
    /// into the shallows or onto a ladder.
    pub pulling: bool,
    /// Frames left of the "!" over the head after a recipe came to mind.
    eureka: u32,
    walk_timer: u32,
    /// What's been picked up has caught the eye: once risen, `Notice`
    /// (trying to climb, if true).
    noticing: Option<bool>,
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
            torch: None,
            weak: false,
            can_run: true,
            queasy: false,
            climbing: false,
            pulling: false,
            noticing: None,
            eureka: 0,
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
        let Situation { tool, item_in_reach, has_leaf, has_knife, fish_near, cart_ahead, kindling } = situation;
        self.eureka = self.eureka.saturating_sub(1);
        if self.action == Action::Collapsed {
            return None;
        }
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
            let roused = [Buttons::A, Buttons::B, Buttons::L].into_iter().any(|b| input.pressed(b));
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
        if let Action::Dig { t, at } = self.action {
            let t = t + 1;
            self.action = if t >= DIG_TOTAL { Action::Idle } else { Action::Dig { t, at } };
            if DIG_STABS.contains(&t) {
                return Some(Deed::KickDust(at));
            }
            return (t == DIG_LIFT).then_some(Deed::Dig(at));
        }
        if let Action::Thrust { t } = self.action {
            let t = t + 1;
            self.action = if t >= THRUST_TOTAL { Action::Idle } else { Action::Thrust { t } };
            return (t == THRUST_HIT).then(|| {
                let (from, to) = self.spear_line();
                Deed::Thrust(from, to)
            });
        }
        if let Action::Cast { t } = self.action {
            let t = t + 1;
            self.action = if t >= CAST_TOTAL { Action::Fishing { t: 0 } } else { Action::Cast { t } };
            return (t == CAST_RELEASE).then(|| Deed::Cast(self.cast_point()));
        }
        if let Action::Fishing { t } = self.action {
            // B strikes; moving off (or A) draws the line in.
            if input.pressed(Buttons::B) {
                self.action = Action::Idle;
                return Some(Deed::Strike);
            }
            let (ax, ay) = input.axis();
            if ax != 0 || ay != 0 || input.pressed(Buttons::A) {
                self.action = Action::Idle;
                return Some(Deed::ReelIn);
            }
            self.action = Action::Fishing { t: t + 1 };
            return None;
        }
        if let Action::PickUp { t } = self.action {
            let t = t + 1;
            self.action = if t < PICK_TOTAL {
                Action::PickUp { t }
            } else if let Some(climb) = self.noticing.take() {
                // Something has caught the eye: a "!" over the head first.
                self.eureka = EUREKA_FRAMES;
                Action::Notice { t: 0, climb }
            } else {
                Action::Idle
            };
            return (t == PICK_GRAB).then_some(Deed::PickUp);
        }
        if let Action::Notice { t, climb } = self.action {
            self.notice(t + 1, climb, map);
            return None;
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
        if let Action::Drill { t, signal } = self.action {
            let t = t + 1;
            self.action = if t >= DRILL_TOTAL { Action::Idle } else { Action::Drill { t, signal } };
            let spot = match signal {
                // On top of the signal fire's stack.
                Some((x, y)) => (x * TILE_SIZE + TILE_SIZE / 2, y * TILE_SIZE - 4),
                None => self.fire_spot(),
            };
            if t == DRILL_CATCH {
                return Some(signal.map_or(Deed::LightFire(spot), Deed::LightSignalFire));
            }
            return ((DRILL_SMOKE..DRILL_CATCH).contains(&t) && t % 8 == 0).then_some(Deed::Smoulder(spot));
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
        if let Action::Scramble { t } = self.action {
            let t = t + 1;
            if t >= SCRAMBLE_TOTAL {
                self.ponder();
            } else {
                self.action = Action::Scramble { t };
            }
            return None;
        }
        if let Action::ToLadder { t, foot, rows } = self.action {
            self.step_to_ladder(t + 1, foot, rows, map);
            return None;
        }
        if let Action::Pitch { t, foot, rows } = self.action {
            let t = t + 1;
            self.action = if t >= PITCH_TOTAL { Action::Idle } else { Action::Pitch { t, foot, rows } };
            return (t == PITCH_SET).then_some(Deed::StandLadder(foot, rows));
        }
        if let Action::ToBridge { t, span, across } = self.action {
            self.action = match self.step_to_row(t + 1, span.row, across, map) {
                Some(true) => Action::Lay { t: 0, span, across },
                Some(false) => Action::ToBridge { t: t + 1, span, across },
                None => Action::Idle,
            };
            return None;
        }
        if let Action::Lay { t, span, across } = self.action {
            let t = t + 1;
            self.action = if t >= PITCH_TOTAL { Action::Idle } else { Action::Lay { t, span, across } };
            return (t == PITCH_SET).then_some(Deed::LayBridge(span));
        }
        if let Action::ToWheel { t, site } = self.action {
            match self.step_to_row(t + 1, site.1, Dir::Right, map) {
                Some(true) => self.action = Action::Mount { t: 0, site },
                Some(false) => self.action = Action::ToWheel { t: t + 1, site },
                None => self.action = Action::Idle,
            }
            return None;
        }
        if let Action::Mount { t, site } = self.action {
            let t = t + 1;
            self.action = if t >= PITCH_TOTAL { Action::Idle } else { Action::Mount { t, site } };
            return (t == PITCH_SET).then_some(Deed::SetWheel(site));
        }
        if let Action::ToLathe { t, site } = self.action {
            self.step_to_lathe(t + 1, site, map);
            return None;
        }
        if let Action::Install { t, site } = self.action {
            let t = t + 1;
            self.action = if t >= PITCH_TOTAL { Action::Idle } else { Action::Install { t, site } };
            return (t == PITCH_SET).then_some(Deed::SetLathe(site));
        }
        if let Action::Build { t, cell } = self.action {
            let t = t + 1;
            self.action = if t >= PITCH_TOTAL { Action::Idle } else { Action::Build { t, cell } };
            return (t == PITCH_SET).then_some(Deed::SetSignalFire(cell));
        }
        if let Action::Survey { t, to, across, think } = self.action {
            self.survey_step(t + 1, to, across, think, map);
            return None;
        }
        // On a ladder there's nothing to do but climb it.
        self.climbing = self.on_ladder(map);
        if self.climbing {
            self.climb(input, map);
            return None;
        }
        if !self.wading && input.pressed(Buttons::L) {
            // Lie down where you stand and sleep (on L, out of the way of the
            // face buttons: waking saves the game, so it shouldn't be hit by mistake).
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
                // The line goes out only over water.
                ItemKind::FishingRod if !is_water(map, self.cast_point()) => return None,
                ItemKind::FishingRod => Action::Cast { t: 0 },
                // A ladder is stood against the rock face ahead, where it
                // reaches the top.
                // Held anywhere along a low stretch of rock, it goes where it
                // reaches: the castaway steps over there first.
                ItemKind::Ladder => {
                    let (rx, ry) = self.reach_point();
                    let fit = if self.facing == Dir::Up { map.ladder_spot(rx.div_euclid(TILE_SIZE), ry.div_euclid(TILE_SIZE)) } else { LadderFit::No };
                    match fit {
                        LadderFit::Reaches { foot, rows } => Action::ToLadder { t: 0, foot, rows },
                        LadderFit::No => return None,
                    }
                }
                // A log bridge goes across the river ahead where it runs
                // narrow: the castaway steps along to its middle first.
                ItemKind::LogBridge => Action::ToBridge { t: 0, span: self.bridge_ahead(map)?, across: self.facing },
                // A water wheel goes into the river at the mill site, the
                // castaway stepping along the bank to it first.
                ItemKind::WaterWheel => Action::ToWheel { t: 0, site: self.wheel_ahead(map)? },
                // A wood lathe goes on the bank beside the turning wheel:
                // held anywhere near, the castaway walks over to set it.
                ItemKind::WoodLathe => Action::ToLathe { t: 0, site: self.lathe_near(map)? },
                // A signal fire goes down on open ground just ahead.
                ItemKind::SignalFire => {
                    let (x, y) = self.fire_spot();
                    let cell = (x.div_euclid(TILE_SIZE), y.div_euclid(TILE_SIZE));
                    if cell == self.feet_cell() || !map.signal_fire_spot(cell.0, cell.1) {
                        return None;
                    }
                    Action::Build { t: 0, cell }
                }
                // A digging stick loosens hard earth but won't lift sticky
                // clay; a bamboo spade lifts clay but would crack on hard
                // earth. The wrong one gives pause.
                ItemKind::DiggingStick | ItemKind::BambooSpade => {
                    let (cell, what) = self.dig_target(map)?;
                    let at = (cell.0 * TILE_SIZE + TILE_SIZE / 2, cell.1 * TILE_SIZE + TILE_SIZE / 2);
                    match (tool, what) {
                        (ItemKind::DiggingStick, Diggable::Earth) | (ItemKind::BambooSpade, Diggable::Clay { dug: false }) => Action::Dig { t: 0, at },
                        (_, Diggable::Clay { dug: true }) => return None,
                        _ => {
                            self.ponder();
                            return None;
                        }
                    }
                }
                _ => Action::Chop { t: 0 },
            };
            self.moving = false;
            self.walk_timer = 0;
            return None;
        }
        if let Some(item) = item_in_reach.filter(|_| input.pressed(Buttons::A)) {
            self.turn_toward(item);
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
            // A signal fire not yet lit: kneel to drill a fire into it, or
            // without what that takes, stop and think.
            if let Some(cell) = self.cold_signal_fire_ahead(map) {
                self.turn_toward(signal_fire_middle(cell));
                if kindling {
                    self.action = Action::Drill { t: 0, signal: Some(cell) };
                    self.moving = false;
                    self.walk_timer = 0;
                } else {
                    self.ponder();
                }
                return None;
            }
            let reach = self.reach_point();
            // Earth and clay take a tool to dig (clay dug out today has
            // nothing more to give).
            if matches!(self.dig_target(map), Some((_, Diggable::Earth | Diggable::Clay { dug: false }))) {
                self.ponder();
                return None;
            }
            // Bamboo won't give to bare hands or a knife: it takes an axe.
            if is_bamboo(map, reach) {
                self.ponder();
                return None;
            }
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
            // A rock face a ladder would reach: try to scramble up it
            // bare-handed (there's no ladder in hand), and fail.
            let face = (rx.div_euclid(TILE_SIZE), ry.div_euclid(TILE_SIZE));
            if self.facing == Dir::Up && tool != Some(ItemKind::Ladder) && matches!(map.ladder_spot(face.0, face.1), LadderFit::Reaches { .. }) {
                self.action = Action::Scramble { t: 0 };
                self.moving = false;
                self.walk_timer = 0;
                return None;
            }
            // Looking across the narrow with no bridge in hand.
            if tool != Some(ItemKind::LogBridge) && self.bridge_ahead(map).is_some() {
                self.ponder();
                return None;
            }
            // Looking at the river off the mill site with no wheel in hand.
            if tool != Some(ItemKind::WaterWheel) && self.wheel_ahead(map).is_some() {
                self.ponder();
                return None;
            }
            if map.is_drinking_water(rx, ry) {
                self.action = Action::Scoop { t: 0 };
                self.moving = false;
                self.walk_timer = 0;
                return None;
            }
            // Near the bank beside the turning wheel with no lathe in hand.
            if tool != Some(ItemKind::WoodLathe) && !cart_ahead && self.lathe_near(map).is_some() {
                self.ponder();
                return None;
            }
            // Fish swimming just there, and nothing to catch them with.
            if fish_near {
                self.ponder();
                return None;
            }
        }
        self.walk(input, map);
        (self.running && self.walk_timer % DUST_EVERY == 1).then(|| {
            // Just behind the heels, or beside them (one foot, then the
            // other) when running toward the camera, so the body doesn't
            // hide it.
            let (fx, fy) = self.body.feet();
            let side = if (self.walk_timer / DUST_EVERY).is_multiple_of(2) { -5 } else { 5 };
            Deed::KickDust(match self.facing {
                Dir::Right => (fx - 7, fy),
                Dir::Left => (fx + 7, fy),
                Dir::Up => (fx + side, fy + 4),
                Dir::Down => (fx + side, fy + 1),
            })
        })
    }

    /// Where a cast lands the float: out in front, a rod and line's length
    /// away.
    pub fn cast_point(&self) -> (i32, i32) {
        let (fx, fy) = self.body.feet();
        match self.facing {
            Dir::Up => (fx, fy - CAST_REACH - 16),
            Dir::Down => (fx, fy + CAST_REACH + 14),
            Dir::Left => (fx - CAST_REACH - 4, fy - 2),
            Dir::Right => (fx + CAST_REACH + 4, fy - 2),
        }
    }

    /// Pose while fishing: drawn back to cast, then arms out.
    fn rod_stage(&self) -> usize {
        match self.action {
            Action::Cast { t } if t < CAST_RELEASE => 0,
            _ => 1,
        }
    }

    /// The rod held out while casting or fishing: its image, where its
    /// top-left is drawn (world px), whether mirrored, and its tip (world
    /// px), placed so the hands hold it just above the butt.
    fn rod_hold(&self, sprites: &SpriteIds) -> (SpriteId, (i32, i32), bool, (i32, i32)) {
        let (view, flip) = view_for(self.facing);
        let (gx, gy) = sprites.player.thrust_poses[view][self.rod_stage()].grip();
        let gx = if flip { CHAR_W as f32 - gx } else { gx };
        let (wx, wy) = self.body.px();
        let (cx, cy) = (wx - CHAR_ORIGIN.0, wy - CHAR_ORIGIN.1);
        // Image, hands in it, tip in it.
        let (gfx, (hx, hy), (tx, ty)) = match self.facing {
            Dir::Right => (sprites.rod_side, (5.0, 5.0), (30, 3)),
            Dir::Left => (sprites.rod_side, (26.0, 5.0), (1, 3)),
            Dir::Down => (sprites.rod_down, (5.0, 5.0), (3, 30)),
            Dir::Up => (sprites.rod_up, (5.0, 26.0), (3, 1)),
        };
        let (x, y) = (cx + (gx - hx).round() as i32, cy + (gy - hy).round() as i32);
        (gfx, (x, y), self.facing == Dir::Left, (x + tx, y + ty))
    }

    /// The tip of the rod held out (world px), where the line leaves it.
    pub fn rod_tip(&self, sprites: &SpriteIds) -> (i32, i32) {
        self.rod_hold(sprites).3
    }

    /// Whether the line is out (or going out).
    pub fn is_fishing(&self) -> bool {
        matches!(self.action, Action::Cast { .. } | Action::Fishing { .. })
    }

    /// The spot just in front of the feet that hands and tools reach: where
    /// the axe blade lands, the trunk being shaken, things picked up.
    /// The ground to dig, and what it is: the cell just ahead, or else the
    /// one underfoot.
    fn dig_target(&self, map: &WorldMap) -> Option<((i32, i32), Diggable)> {
        let cell = |(px, py): (i32, i32)| (px.div_euclid(TILE_SIZE), py.div_euclid(TILE_SIZE));
        [cell(self.reach_point()), cell(self.body.feet())].into_iter().find_map(|c| diggable(map, c).map(|d| (c, d)))
    }

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

        // Water drags at the legs too much to run, and so do hunger and an
        // upset stomach.
        let was_running = self.running;
        self.running = self.moving && !self.wading && self.can_run && !self.pulling && input.held(Buttons::R);
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
        let speed = if self.weak { speed * WEAK_PACE_QUARTERS / 4 } else { speed };
        // The cart takes no part in what's in the way: the castaway bumps
        // into things as ever, pulling it or not (only where it can't be
        // pulled keeps them out).
        let pulling = self.pulling;
        let blocked = |r: Rect| map.is_solid_rect(r) || (pulling && map.stops_a_cart(r));
        self.body.move_by(ax * speed, ay * speed, CORNER_ASSIST_PX, &blocked);

        // Keep walking in place against walls, like classic action RPGs.
        self.walk_timer = if self.moving { self.walk_timer + 1 } else { 0 };
    }

    /// Whether the feet are on a ladder.
    fn on_ladder(&self, map: &WorldMap) -> bool {
        let (fx, fy) = self.body.feet();
        map.is_ladder(fx.div_euclid(TILE_SIZE), fy.div_euclid(TILE_SIZE))
    }

    /// Climbs up or down the ladder underfoot, facing the rock, drawn in
    /// to the middle of it.
    fn climb(&mut self, input: &InputState, map: &WorldMap) {
        let (_, ay) = input.axis();
        self.moving = ay != 0;
        self.running = false;
        self.wading = false;
        self.facing = Dir::Up;
        let (fx, _) = self.body.feet();
        let middle = fx.div_euclid(TILE_SIZE) * TILE_SIZE + TILE_SIZE / 2;
        let dx = (middle - fx).signum() * 256;
        self.body.move_by(dx, ay * SPEED_CLIMB, 0, &|r| map.is_solid_rect(r));
        self.walk_timer = if self.moving { self.walk_timer + 1 } else { 0 };
    }

    pub fn is_asleep(&self) -> bool {
        matches!(self.action, Action::Sleep { .. })
    }

    /// Falls where they stand, health spent.
    pub fn collapse(&mut self) {
        self.action = Action::Collapsed;
        self.facing = Dir::Right;
        self.moving = false;
        self.running = false;
        self.walk_timer = 0;
        self.eureka = 0;
        self.torch = None;
    }

    /// Wakes from sleep: sitting up, getting up and looking round.
    pub fn wake(&mut self) {
        if self.is_asleep() {
            self.action = Action::WakeUp { t: WAKE_SIT, end: WAKE_FROM_SLEEP_END };
        }
    }

    /// Once risen from picking it up, looks up and round at what's been
    /// noticed (and tries to climb, if `climb`).
    pub fn notice_after_pickup(&mut self, climb: bool) {
        self.noticing = Some(climb);
    }

    /// A frame of `Action::Notice`: looking up at the cliff, left and right
    /// round about, up again; then walking up to the rock until it's in
    /// reach, and trying to scramble up it.
    fn notice(&mut self, t: u32, climb: bool, map: &WorldMap) {
        self.moving = false;
        self.running = false;
        if t < NOTICE_LOOKED {
            self.facing = match t {
                0..NOTICE_LEFT => Dir::Up,
                NOTICE_LEFT..NOTICE_RIGHT => Dir::Left,
                NOTICE_RIGHT..NOTICE_BACK => Dir::Right,
                _ => Dir::Up,
            };
            self.walk_timer = 0;
            self.action = Action::Notice { t, climb };
            return;
        }
        self.facing = Dir::Up;
        if !climb {
            self.action = Action::Idle;
            return;
        }
        let (rx, ry) = self.reach_point();
        if matches!(map.ladder_spot(rx.div_euclid(TILE_SIZE), ry.div_euclid(TILE_SIZE)), LadderFit::Reaches { .. }) {
            self.walk_timer = 0;
            self.action = Action::Scramble { t: 0 };
            return;
        }
        // Step up to the rock, drawn in to the middle of the cell.
        let (fx, _) = self.body.feet();
        let middle = fx.div_euclid(TILE_SIZE) * TILE_SIZE + TILE_SIZE / 2;
        let moved = self.body.move_by((middle - fx).signum() * SPEED_WALK / 2, -SPEED_WALK, 0, &|r| map.is_solid_rect(r));
        if !moved || t >= NOTICE_LOOKED + NOTICE_APPROACH {
            self.walk_timer = 0;
            self.action = Action::Idle;
            return;
        }
        self.moving = true;
        self.walk_timer += 1;
        self.action = Action::Notice { t, climb };
    }

    /// A frame of `Action::ToLadder`: walking sideways until the feet are
    /// in the middle of the column of the face at `foot`, then turning to
    /// the rock to pitch the ladder. Stopped by something in the way (or
    /// taking too long), the ladder isn't pitched.
    fn step_to_ladder(&mut self, t: u32, foot: (i32, i32), rows: i32, map: &WorldMap) {
        let (fx, _) = self.body.feet();
        let middle = foot.0 * TILE_SIZE + TILE_SIZE / 2;
        if fx == middle {
            self.facing = Dir::Up;
            self.moving = false;
            self.walk_timer = 0;
            self.action = Action::Pitch { t: 0, foot, rows };
            return;
        }
        let step = (middle - fx).clamp(-1, 1);
        // The last pixel exactly, so the feet land on the middle.
        let dx = if (middle - fx).abs() * 256 < SPEED_WALK { (middle - fx) * 256 - self.body.x.rem_euclid(256) } else { step * SPEED_WALK };
        let moved = self.body.move_by(dx, 0, 0, &|r| map.is_solid_rect(r));
        if !moved || t > TO_LADDER_FRAMES {
            self.facing = Dir::Up;
            self.moving = false;
            self.walk_timer = 0;
            self.action = Action::Idle;
            return;
        }
        self.facing = if step < 0 { Dir::Left } else { Dir::Right };
        self.moving = true;
        self.walk_timer += 1;
        self.action = Action::ToLadder { t, foot, rows };
    }

    /// Where a log bridge would go across the river just ahead, if
    /// facing across it (it runs north to south).
    fn bridge_ahead(&self, map: &WorldMap) -> Option<BridgeSpan> {
        if !matches!(self.facing, Dir::Left | Dir::Right) {
            return None;
        }
        let (rx, ry) = self.reach_point();
        map.bridge_spot(rx.div_euclid(TILE_SIZE), ry.div_euclid(TILE_SIZE))
    }

    /// The mill site, if a water wheel would go into the river just ahead
    /// from it (facing the river east from its west bank).
    fn wheel_ahead(&self, map: &WorldMap) -> Option<(i32, i32)> {
        if self.facing != Dir::Right {
            return None;
        }
        let (rx, ry) = self.reach_point();
        map.wheel_spot(rx.div_euclid(TILE_SIZE), ry.div_euclid(TILE_SIZE))
    }

    /// The cell the feet are in.
    fn feet_cell(&self) -> (i32, i32) {
        let (fx, fy) = self.body.feet();
        (fx.div_euclid(TILE_SIZE), fy.div_euclid(TILE_SIZE))
    }

    /// The cell of a signal fire close at hand that isn't burning, if there
    /// is one: within reach of its stones as drawn (wider than its cell),
    /// or underfoot (it can be walked over). The nearest, if more.
    fn cold_signal_fire_ahead(&self, map: &WorldMap) -> Option<(i32, i32)> {
        let (reach, feet) = (self.reach_point(), self.body.feet());
        let d2 = |(ax, ay): (i32, i32), (bx, by): (i32, i32)| (ax - bx).pow(2) + (ay - by).pow(2);
        let (cx, cy) = self.feet_cell();
        (cy - 2..=cy + 2)
            .flat_map(|y| (cx - 2..=cx + 2).map(move |x| (x, y)))
            .filter(|&(x, y)| map.feature(x, y) == crate::world::Feature::SignalFire(false))
            .map(|cell| (cell, signal_fire_middle(cell)))
            .filter(|&(_, m)| d2(reach, m) <= SIGNAL_REACH * SIGNAL_REACH || d2(feet, m) <= SIGNAL_UNDERFOOT * SIGNAL_UNDERFOOT)
            .min_by_key(|&(_, m)| d2(reach, m))
            .map(|(cell, _)| cell)
    }

    /// Turns to face the point (world px), unless it's (nearly) underfoot.
    fn turn_toward(&mut self, (ix, iy): (i32, i32)) {
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
    }

    /// The mill site, if a wood lathe held here would be set on the bank
    /// beside the turning water wheel (the castaway is near enough).
    fn lathe_near(&self, map: &WorldMap) -> Option<(i32, i32)> {
        let (fx, fy) = self.body.feet();
        map.lathe_spot(fx.div_euclid(TILE_SIZE), fy.div_euclid(TILE_SIZE))
    }

    /// A frame of `Action::ToLathe`: walking (across, then along) until the
    /// feet are in the middle of the place to stand to set the lathe, then
    /// turning east to it. Stopped by something in the way (or taking too
    /// long), it isn't set.
    fn step_to_lathe(&mut self, t: u32, site: (i32, i32), map: &WorldMap) {
        let (sx, sy) = crate::world::map::lathe_stand(site);
        let to = (sx * TILE_SIZE + TILE_SIZE / 2, sy * TILE_SIZE + TILE_SIZE / 2);
        let (fx, fy) = self.body.feet();
        if (fx, fy) == to {
            self.facing = Dir::Right;
            self.moving = false;
            self.walk_timer = 0;
            self.action = Action::Install { t: 0, site };
            return;
        }
        // One axis at a time (the longer way first), the last pixel exactly.
        let step = |d: i32, sub: i32| if d.abs() * 256 < SPEED_WALK { d * 256 - sub } else { d.signum() * SPEED_WALK };
        let (ddx, ddy) = (to.0 - fx, to.1 - fy);
        let along_x = ddx != 0 && (ddx.abs() >= ddy.abs() || ddy == 0);
        let (dx, dy) = if along_x { (step(ddx, self.body.x.rem_euclid(256)), 0) } else { (0, step(ddy, self.body.y.rem_euclid(256))) };
        let moved = self.body.move_by(dx, dy, CORNER_ASSIST_PX, &|r| map.is_solid_rect(r));
        if !moved || t > TO_LATHE_FRAMES {
            self.moving = false;
            self.walk_timer = 0;
            self.action = Action::Idle;
            return;
        }
        self.facing = match (dx.signum(), dy.signum()) {
            (1, _) => Dir::Right,
            (-1, _) => Dir::Left,
            (_, -1) => Dir::Up,
            _ => Dir::Down,
        };
        self.moving = true;
        self.walk_timer += 1;
        self.action = Action::ToLathe { t, site };
    }

    /// Walks along the bank (up or down) until the feet are in the middle
    /// of `row`, then turns `across` the river: `Some(true)` once there,
    /// `Some(false)` on the way, and `None` if something is in the way (or
    /// it takes too long).
    fn step_to_row(&mut self, t: u32, row: i32, across: Dir, map: &WorldMap) -> Option<bool> {
        let (_, fy) = self.body.feet();
        let middle = row * TILE_SIZE + TILE_SIZE / 2;
        if fy == middle {
            self.facing = across;
            self.moving = false;
            self.walk_timer = 0;
            return Some(true);
        }
        let step = (middle - fy).clamp(-1, 1);
        // The last pixel exactly, so the feet land on the middle.
        let dy = if (middle - fy).abs() * 256 < SPEED_WALK { (middle - fy) * 256 - self.body.y.rem_euclid(256) } else { step * SPEED_WALK };
        let moved = self.body.move_by(0, dy, 0, &|r| map.is_solid_rect(r));
        if !moved || t > TO_LADDER_FRAMES {
            self.facing = across;
            self.moving = false;
            self.walk_timer = 0;
            return None;
        }
        self.facing = if step < 0 { Dir::Up } else { Dir::Down };
        self.moving = true;
        self.walk_timer += 1;
        Some(false)
    }

    /// Comes upon a place by the river: an idea strikes ("!"), then walks
    /// straight over to its bank `to` (a cell) and turns `across` the
    /// river, then stops to think if `think`.
    pub fn survey(&mut self, to: (i32, i32), across: Dir, think: bool) {
        self.action = Action::Survey { t: 0, to, across, think };
        self.eureka();
        self.moving = false;
        self.running = false;
        self.walk_timer = 0;
    }

    /// A frame of `Action::Survey`: standing struck by an idea, then
    /// walking (across, then along) to the middle of the bank cell, facing `across` the river there and, if `think`,
    /// stopping to think. Stopped by something in the way (or taking too
    /// long), it ends where it is.
    fn survey_step(&mut self, t: u32, to: (i32, i32), across: Dir, think: bool, map: &WorldMap) {
        self.running = false;
        if t < SURVEY_STRUCK {
            self.moving = false;
            self.walk_timer = 0;
            self.action = Action::Survey { t, to, across, think };
            return;
        }
        let (fx, fy) = self.body.feet();
        let (mx, my) = (to.0 * TILE_SIZE + TILE_SIZE / 2, to.1 * TILE_SIZE + TILE_SIZE / 2);
        if (fx, fy) == (mx, my) {
            self.facing = across;
            self.moving = false;
            self.walk_timer = 0;
            if think {
                self.ponder();
            } else {
                self.action = Action::Idle;
            }
            return;
        }
        // One axis at a time, the last pixel exactly, as along a bank.
        let toward =
            |from: i32, to: i32, sub: i32| if (to - from).abs() * 256 < SPEED_WALK { (to - from) * 256 - sub } else { (to - from).signum() * SPEED_WALK };
        let (dx, dy) = if fx != mx { (toward(fx, mx, self.body.x.rem_euclid(256)), 0) } else { (0, toward(fy, my, self.body.y.rem_euclid(256))) };
        let moved = self.body.move_by(dx, dy, 0, &|r| map.is_solid_rect(r));
        if !moved || t > SURVEY_STRUCK + SURVEY_WALK {
            self.facing = across;
            self.moving = false;
            self.walk_timer = 0;
            self.action = Action::Idle;
            return;
        }
        self.facing = match (dx.signum(), dy.signum()) {
            (1, _) => Dir::Right,
            (-1, _) => Dir::Left,
            (_, -1) => Dir::Up,
            _ => Dir::Down,
        };
        self.moving = true;
        self.walk_timer += 1;
        self.action = Action::Survey { t, to, across, think };
    }

    /// Stops to think (something here needs what isn't carried).
    fn ponder(&mut self) {
        self.action = Action::Ponder { t: 0 };
        self.moving = false;
        self.walk_timer = 0;
    }

    /// A recipe has come to mind: a "!" pops up over the head for a moment,
    /// without stopping whatever's being done.
    pub fn eureka(&mut self) {
        self.eureka = EUREKA_FRAMES;
    }

    /// A "!" is over the head (an idea has struck).
    pub fn is_struck_by_an_idea(&self) -> bool {
        self.eureka > 0
    }

    /// Stopped to think.
    pub fn is_pondering(&self) -> bool {
        matches!(self.action, Action::Ponder { .. })
    }

    /// Standing ready: not mid-action (so the bag can be opened).
    /// Whether the castaway is wading in the shallows.
    pub fn is_wading(&self) -> bool {
        self.wading
    }

    pub fn is_idle(&self) -> bool {
        self.action == Action::Idle
    }

    /// Starts eating or drinking something taken from the bag.
    pub fn consume(&mut self, kind: ItemKind) {
        self.action = match kind.usage() {
            Some(Use::Eat) => Action::Eat { t: 0, food: kind },
            Some(Use::Drink) => Action::Drink { t: 0, water: kind },
            Some(Use::Light) => Action::Drill { t: 0, signal: None },
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
        let step_frames = if self.weak { step_frames * 4 / WEAK_PACE_QUARTERS as u32 } else { step_frames };
        // Start on a step frame so movement reads instantly.
        let phase = if self.moving { (self.walk_timer / step_frames + 1) % 4 } else { 0 };
        let (view, flip) = view_for(self.facing);
        // The 24x32 character canvas is offset from the 16x24 logical frame.
        let (cx, cy) = (sx - CHAR_ORIGIN.0, sy - CHAR_ORIGIN.1);

        if self.eureka > 0 && !self.is_asleep() {
            // Over the head, springing up into place.
            let age = EUREKA_FRAMES - self.eureka;
            let rise = EUREKA_POP.saturating_sub(age) as i32;
            let bx = if flip { sx - 12 } else { sx + 12 };
            list.actor(depth + 1, ObjEntry::new(bx, sy - 18 + rise, sprites.eureka, OBJ_FX, prio).flipped(flip));
        }

        if let Action::WakeUp { t, .. } = self.action
            && t < WAKE_KNEEL
        {
            self.draw_waking(list, (sx, sy), (cx, cy), depth, sprites, t, prio, shadows);
            return;
        }

        if self.action == Action::Collapsed {
            list.actor(depth, ObjEntry::new(sx - 15, sy + 6, sprites.player.sleeping, OBJ_PLAYER, prio));
            if shadows {
                list.shadow(ObjEntry::new(sx - 15, sy + 15, sprites.shadow_lying, OBJ_SHADOW, ACTOR_PRIORITY));
            }
            return;
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
                let sway = if (age / 8).is_multiple_of(2) { 0 } else { 1 };
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

        if self.is_fishing() {
            // Holding the rod out over the water (the line and float are
            // drawn with the line). Facing away it's behind the body.
            let frame = sprites.player.thrust[view][self.rod_stage()];
            let (gfx, (rx, ry), mirror, _) = self.rod_hold(sprites);
            let (rx, ry) = camera.to_screen(rx, ry);
            let rod = ObjEntry::new(rx, ry, gfx, OBJ_FX, prio).flipped(mirror);
            let body = ObjEntry::new(cx, cy, frame.full, OBJ_PLAYER, prio).flipped(flip);
            if self.facing == Dir::Up {
                list.actor(depth, body);
                list.actor(depth, rod);
            } else {
                list.actor(depth, ObjEntry::new(cx, cy, frame.hands, OBJ_PLAYER, prio).flipped(flip));
                list.actor(depth, rod);
                list.actor(depth, body);
            }
            if shadows && !self.wading {
                list.shadow(ObjEntry::new(sx, sy + SPRITE_H - 5, sprites.shadow_player, OBJ_SHADOW, ACTOR_PRIORITY));
            }
            return;
        }

        if let Action::Dig { t, .. } = self.action {
            // Crouched to drive the tool in, rising between the stabs.
            let low = DIG_STABS.iter().any(|&s| (s.saturating_sub(4)..s + 6).contains(&t)) || (DIG_LIFT - 4..DIG_LIFT + 4).contains(&t);
            let gfx = if low { sprites.player.crouch[view] } else { sprites.player.walk[view][0].full };
            list.actor(depth, ObjEntry::new(cx, cy, gfx, OBJ_PLAYER, prio).flipped(flip));
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

        if let Action::Drill { t, .. } = self.action {
            // Kneeling over the drill, sawing its bow from side to side;
            // it's behind the body when facing away.
            let body = ObjEntry::new(cx, cy, sprites.player.crouch[view], OBJ_PLAYER, prio).flipped(flip);
            let (dx, dy) = self.front_of_feet();
            let (fx, fy) = self.body.feet();
            let saw = if t < DRILL_CATCH && (t / DRILL_STROKE).is_multiple_of(2) { 1 } else { -1 };
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

        if let Action::Scramble { t } = self.action {
            // Hands up on the rock, hauling up (hand over hand) and
            // sliding back down, twice.
            let k = t % SCRAMBLE_TRY;
            let lift = match k {
                0..=3 => 0,
                4..=13 => ((k - 3) / 2).min(4) as i32,
                14..=17 => 4 - ((k - 13) as i32),
                _ => 0,
            };
            let step = ((k / 5) % 2) as usize;
            list.actor(depth, ObjEntry::new(cx, cy - lift, sprites.player.climb[step], OBJ_PLAYER, prio));
            if shadows {
                list.shadow(ObjEntry::new(sx, sy + SPRITE_H - 5, sprites.shadow_player, OBJ_SHADOW, ACTOR_PRIORITY));
            }
            return;
        }

        if let Action::Pitch { t, .. } | Action::Lay { t, .. } | Action::Mount { t, .. } | Action::Install { t, .. } | Action::Build { t, .. } = self.action {
            // Lifting the ladder (or bridge), then leaning in to set it
            // against the rock (or down across the river).
            let stage = (t >= PITCH_SET) as usize;
            list.actor(depth, ObjEntry::new(cx, cy, sprites.player.shake[view][stage], OBJ_PLAYER, prio).flipped(flip));
            if shadows {
                list.shadow(ObjEntry::new(sx, sy + SPRITE_H - 5, sprites.shadow_player, OBJ_SHADOW, ACTOR_PRIORITY));
            }
            return;
        }

        if self.climbing {
            // Hand over hand, facing the rock; no shadow on the face.
            let step = if self.moving { (self.walk_timer / STEP_FRAMES_CLIMB) as usize % 2 } else { 0 };
            list.actor(depth, ObjEntry::new(cx, cy, sprites.player.climb[step], OBJ_PLAYER, prio));
            return;
        }

        if matches!(self.action, Action::WakeUp { t, .. } if t < WAKE_STAND) {
            list.actor(depth, ObjEntry::new(cx, cy, sprites.player.kneeling, OBJ_PLAYER, prio));
            if shadows {
                list.shadow(ObjEntry::new(sx, sy + SPRITE_H - 5, sprites.shadow_player, OBJ_SHADOW, ACTOR_PRIORITY));
            }
            return;
        }
        let frame = self.walk_set(sprites)[view][phase as usize];
        if let Some(low) = self.torch {
            self.draw_torch(list, (cx, cy), depth, sprites, (view, flip, phase as usize), (clock, low), prio);
        }
        if self.wading {
            // Foam ring first (in front), then the body above the waterline,
            // then the submerged legs in a blended palette.
            let ripple = sprites.ripples[(clock / 10) as usize % sprites.ripples.len()];
            list.actor(depth, ObjEntry::new(sx - 4, sy + 14, ripple, OBJ_WATER_FX, prio));
            list.actor(depth, ObjEntry::new(cx, cy, frame.top, OBJ_PLAYER, prio).flipped(flip));
            list.actor(depth, ObjEntry::new(cx, cy, frame.legs, OBJ_WADE, prio).flipped(flip));
        } else {
            list.actor(depth, ObjEntry::new(cx, cy, frame.full, OBJ_PLAYER, prio).flipped(flip));
            if self.pulling && view != 1 {
                // The hands over the cart's handle, held in front of them
                // (from behind, they're out of sight).
                list.actor(depth + 2, ObjEntry::new(cx, cy, frame.hands, OBJ_PLAYER, prio).flipped(flip));
            }
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
    fn draw_eating(
        &self,
        list: &mut DrawList,
        (sx, sy): (i32, i32),
        (cx, cy): (i32, i32),
        depth: i32,
        sprites: &SpriteIds,
        t: u32,
        food: ItemKind,
        prio: u8,
        shadows: bool,
    ) {
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
    /// The walk (and standing) frames to draw: holding the belly with an
    /// upset stomach.
    fn walk_set<'a>(&self, sprites: &'a SpriteIds) -> &'a [[crate::assets::CharFrame; 4]; 3] {
        if self.pulling {
            &sprites.player.pull
        } else if self.queasy {
            &sprites.player.queasy
        } else {
            &sprites.player.walk
        }
    }

    /// The torch held in the near hand (the screen-right one facing toward
    /// or away). It's behind the body facing away;
    /// otherwise in front, with the hands drawn over its handle.
    #[allow(clippy::too_many_arguments)]
    fn draw_torch(
        &self,
        list: &mut DrawList,
        (cx, cy): (i32, i32),
        depth: i32,
        sprites: &SpriteIds,
        (view, flip, phase): (usize, bool, usize),
        (clock, low): (u32, bool),
        prio: u8,
    ) {
        let poses = if self.pulling {
            &sprites.player.pull_poses
        } else if self.queasy {
            &sprites.player.queasy_poses
        } else {
            &sprites.player.walk_poses
        };
        let pose = &poses[view][phase];
        // Held a little out from the body, so the flame stands clear of
        // the face, and steady: the arm holding it doesn't swing, it only
        // bobs with the step.
        let hx = poses[view][0].arms[1].c.0;
        let hy = pose.arms[1].c.1 + TORCH_DROP[view];
        let hx = hx + TORCH_REACH[view];
        let hx = if flip { CHAR_W as f32 - hx } else { hx };
        let (gx, gy) = TORCH_GRIP;
        let frames = if low { &sprites.torch_low } else { &sprites.torch };
        let flame = (clock / TORCH_FLAME_FRAME) as usize % frames.len();
        let torch = ObjEntry::new(cx + (hx - gx).round() as i32, cy + (hy - gy).round() as i32, frames[flame], OBJ_CRAB, prio).flipped(flip);
        if self.facing == Dir::Up {
            list.actor(depth - 1, torch);
        } else {
            list.actor(depth + 1, ObjEntry::new(cx, cy, self.walk_set(sprites)[view][phase].hands, OBJ_PLAYER, prio).flipped(flip));
            list.actor(depth + 1, torch);
        }
    }

    /// Kneeling by the fire with the food held out toward the flames at
    /// arm's length, bobbing as it's turned. Crabs are turned over now and
    /// then; a pot is held steady. Facing away, it's hidden by the body.
    #[allow(clippy::too_many_arguments)]
    fn draw_cooking(
        &self,
        list: &mut DrawList,
        camera: &Camera,
        (cx, cy): (i32, i32),
        depth: i32,
        sprites: &SpriteIds,
        (t, food, fire): (u32, ItemKind, (i32, i32)),
        prio: u8,
    ) {
        let (view, flip) = view_for(self.facing);
        list.actor(depth, ObjEntry::new(cx, cy, sprites.player.crouch[view], OBJ_PLAYER, prio).flipped(flip));
        if food == ItemKind::Branch {
            // A torch: its head thrust into the flames until it catches,
            // leaning back toward the hands.
            let lean = match self.facing {
                Dir::Left => 4,
                Dir::Right => -4,
                _ => 0,
            };
            let (hx, hy) = (8, 14);
            let (sx, sy) = camera.to_screen(fire.0 + lean - hx, fire.1 - 8 - hy);
            let torch = sprites.torch[(t / TORCH_FLAME_FRAME) as usize % sprites.torch.len()];
            list.actor(fire.1 + 1, ObjEntry::new(sx, sy, torch, OBJ_CRAB, prio));
            return;
        }
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

/// Whether there's water (sea or river) at the world pixel.
fn is_water(map: &WorldMap, (x, y): (i32, i32)) -> bool {
    let (cx, cy) = (x.div_euclid(crate::world::TILE_SIZE), y.div_euclid(crate::world::TILE_SIZE));
    (0..map.width()).contains(&cx) && (0..map.height()).contains(&cy) && map.terrain(cx, cy).holds_water()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::island;
    use crate::world::map::BRIDGE_REACH;

    /// The island, and the ground cell at the foot of its low cliff's
    /// middle (where a ladder stands).
    fn island_and_foot() -> (WorldMap, (i32, i32)) {
        let map = island::generate(1);
        let foot = map.spots.ladder.expect("the island has a low cliff");
        (map, foot)
    }

    /// A castaway standing in the cell `(x, y)`, facing the rock above.
    fn facing_the_rock_at(x: i32, y: i32) -> Player {
        let mut p = Player::at_feet(x * TILE_SIZE, y * TILE_SIZE);
        // Into the middle of the cell, wherever the hitbox sits.
        let (fx, fy) = p.body.feet();
        p.body.move_by((x * TILE_SIZE + 8 - fx) * 256, (y * TILE_SIZE + 10 - fy) * 256, 0, &|_| false);
        assert_eq!(p.body.feet().0.div_euclid(TILE_SIZE), x);
        assert_eq!(p.body.feet().1.div_euclid(TILE_SIZE), y);
        p.facing = Dir::Up;
        p
    }

    /// Presses `button` once with `tool` in hand, then lets the castaway
    /// carry on for up to `frames` frames; what was done to the world.
    fn press(p: &mut Player, map: &WorldMap, button: u16, tool: Option<ItemKind>, frames: u32) -> Vec<Deed> {
        let mut input = InputState::default();
        let situation = Situation { tool, ..Situation::default() };
        let mut deeds = Vec::new();
        for f in 0..frames {
            input.update(if f == 0 { button } else { 0 });
            deeds.extend(p.update(&input, map, situation));
        }
        deeds
    }

    #[test]
    fn a_ladder_pitched_anywhere_under_the_recess_stands_in_its_middle() {
        let (map, (x, y)) = island_and_foot();
        for dx in -1..=1 {
            let mut p = facing_the_rock_at(x + dx, y);
            let deeds = press(&mut p, &map, Buttons::B, Some(ItemKind::Ladder), 240);
            let stood: Vec<_> = deeds.iter().filter_map(|d| if let Deed::StandLadder(foot, _) = d { Some(*foot) } else { None }).collect();
            assert_eq!(stood, vec![(x, y - 1)], "pitched from {dx} cells off the middle");
            assert_eq!(p.body.feet().0.div_euclid(TILE_SIZE), x, "the castaway steps over to the middle to pitch it");
        }
    }

    #[test]
    fn a_ladder_wont_go_against_the_full_height_rock_beside_the_recess() {
        let (map, (x, y)) = island_and_foot();
        for dx in [-2, 2] {
            let mut p = facing_the_rock_at(x + dx, y);
            let deeds = press(&mut p, &map, Buttons::B, Some(ItemKind::Ladder), 240);
            assert!(!deeds.iter().any(|d| matches!(d, Deed::StandLadder(..))), "{dx} cells off the middle");
            assert_eq!(p.action, Action::Idle);
        }
    }

    #[test]
    fn trying_to_climb_anywhere_under_the_recess_without_a_ladder_scrambles() {
        let (map, (x, y)) = island_and_foot();
        for dx in -1..=1 {
            let mut p = facing_the_rock_at(x + dx, y);
            press(&mut p, &map, Buttons::A, None, 1);
            assert!(matches!(p.action, Action::Scramble { .. }), "{dx} cells off the middle");
        }
    }

    /// A castaway standing in the cell beside the river at row `y` by the
    /// narrow, facing `across` it (`Left` from the east bank, `Right` from
    /// the west).
    fn facing_the_narrow_at(map: &WorldMap, y: i32, across: Dir) -> Player {
        let (bx, _) = map.spots.bridge.expect("the river has a narrow");
        let step = if across == Dir::Left { 1 } else { -1 };
        let x = (0..).map(|k| bx + k * step).find(|&x| map.terrain(x, y) != crate::world::Terrain::River).unwrap();
        let mut p = facing_the_rock_at(x, y);
        p.facing = across;
        p
    }

    #[test]
    fn a_bridge_laid_anywhere_along_the_narrow_goes_across_its_middle() {
        let map = island::generate(1);
        let (_, by) = map.spots.bridge.expect("the river has a narrow");
        for across in [Dir::Left, Dir::Right] {
            for y in by - BRIDGE_REACH..=by + BRIDGE_REACH {
                let mut p = facing_the_narrow_at(&map, y, across);
                let deeds = press(&mut p, &map, Buttons::B, Some(ItemKind::LogBridge), 240);
                let laid: Vec<_> = deeds.iter().filter_map(|d| if let Deed::LayBridge(span) = d { Some(span.row) } else { None }).collect();
                assert_eq!(laid, vec![by], "laid from row {y} facing {across:?}");
                assert_eq!(p.body.feet().1.div_euclid(TILE_SIZE), by, "the castaway steps along to the middle to lay it");
            }
        }
    }

    #[test]
    fn a_bridge_wont_go_across_the_river_beyond_the_narrow() {
        let map = island::generate(1);
        let (_, by) = map.spots.bridge.expect("the river has a narrow");
        for y in [by - BRIDGE_REACH - 2, by + BRIDGE_REACH + 2] {
            let mut p = facing_the_narrow_at(&map, y, Dir::Left);
            let deeds = press(&mut p, &map, Buttons::B, Some(ItemKind::LogBridge), 240);
            assert!(!deeds.iter().any(|d| matches!(d, Deed::LayBridge(..))), "{y}");
        }
    }

    #[test]
    fn reaching_across_the_narrow_without_a_bridge_makes_the_castaway_think() {
        let map = island::generate(1);
        let (_, by) = map.spots.bridge.expect("the river has a narrow");
        for y in by - BRIDGE_REACH..=by + BRIDGE_REACH {
            let mut p = facing_the_narrow_at(&map, y, Dir::Left);
            press(&mut p, &map, Buttons::A, None, 1);
            assert!(matches!(p.action, Action::Ponder { .. }), "row {y}");
        }
    }

    #[test]
    fn reaching_for_bamboo_with_no_axe_makes_the_castaway_think() {
        use crate::world::Feature;
        use crate::world::terrain::BambooPart;
        let mut map = WorldMap::new(8, 8, crate::world::Terrain::Grass);
        for dy in 0..BambooPart::H {
            map.set_feature(3, dy, Feature::Bamboo(BambooPart { dy: dy as u8, v: 0, flip: false }));
        }
        // Standing just below the culm's foot, facing it.
        let mut p = facing_the_rock_at(3, BambooPart::H);
        press(&mut p, &map, Buttons::A, Some(ItemKind::StoneKnife), 1);
        assert!(matches!(p.action, Action::Ponder { .. }));
    }
}

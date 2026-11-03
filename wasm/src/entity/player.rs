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
use crate::world::harvest::can_shake;

pub const SPRITE_W: i32 = 16;
pub const SPRITE_H: i32 = 24;
/// Feet-only collision box (top-down games collide at the feet so heads can
/// overlap things above them).
const HITBOX: Rect = Rect::new(3, 18, 10, 6);
/// Speeds in 8.8 fixed point per frame.
const SPEED_WALK: i32 = 336; // ~1.3 px
const SPEED_WALK_DIAG: i32 = 238;
const SPEED_WADE: i32 = 184;
const SPEED_WADE_DIAG: i32 = 130;
const CORNER_ASSIST_PX: i32 = 6;
/// Frames per walk-animation step.
const STEP_FRAMES_WALK: u32 = 8;
const STEP_FRAMES_WADE: u32 = 12;
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

/// Waking up on the beach (frames): lying still, stirring (eyes open and a
/// blink), sitting up, getting onto one knee, standing and looking around.
const WAKE_STIR: u32 = 110;
const WAKE_SIT: u32 = 160;
const WAKE_KNEEL: u32 = 205;
const WAKE_STAND: u32 = 232;
const WAKE_END: u32 = 330;
/// Earliest frame a button press can skip the wake-up.
const WAKE_SKIPPABLE: u32 = 20;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Idle,
    /// Swinging the axe; `t` counts frames since the swing started.
    Chop { t: u32 },
    /// Coming to on the beach; `t` counts frames since the game started.
    WakeUp { t: u32 },
    /// Gripping a trunk and rocking it; `t` counts frames in this round.
    Shake { t: u32 },
    /// Crouching to pick something up.
    PickUp { t: u32 },
    /// Scooping up water to carry (raw water).
    Scoop { t: u32 },
    /// Drinking water from the bag.
    Drink { t: u32 },
    /// Eating something from the bag (a coconut).
    Eat { t: u32 },
}

/// What the castaway has to hand and what lies around them this frame.
#[derive(Clone, Copy, Debug, Default)]
pub struct Situation {
    pub has_axe: bool,
    /// Where something lies close enough to pick up.
    pub item_in_reach: Option<(i32, i32)>,
    /// Something to eat in the bag.
    pub has_food: bool,
    /// Water to drink in the bag.
    pub has_water: bool,
}

/// Something the castaway did to the world this frame, at a world pixel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Deed {
    /// An axe blow connected.
    Chop((i32, i32)),
    /// A trunk was shaken.
    Shake((i32, i32)),
    /// Hands closed on whatever lies within reach.
    PickUp,
    /// Scooped up water to carry.
    DrawWater,
    /// Drank some carried water (it is used up).
    Drink,
    /// Ate a bite (the food is used up).
    Eat,
}

#[derive(Clone, Debug)]
pub struct Player {
    pub body: Body,
    pub facing: Dir,
    pub moving: bool,
    pub wading: bool,
    pub action: Action,
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
        p.action = Action::WakeUp { t: 0 };
        p.facing = Dir::Right;
        p
    }

    pub fn new(px: i32, py: i32) -> Self {
        Self {
            body: Body::new(px, py, HITBOX),
            facing: Dir::Down,
            moving: false,
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
        let Situation { has_axe, item_in_reach, has_food, has_water } = situation;
        if let Action::WakeUp { t } = self.action {
            let skip = t >= WAKE_SKIPPABLE && (input.pressed(Buttons::A) || input.pressed(Buttons::B));
            let t = if skip { WAKE_END } else { t + 1 };
            if t >= WAKE_END {
                self.action = Action::Idle;
                self.facing = Dir::Down;
            } else {
                self.action = Action::WakeUp { t };
                // Look around once standing.
                self.facing = if (WAKE_STAND + 30..WAKE_STAND + 60).contains(&t) { Dir::Left } else { Dir::Right };
            }
            return None;
        }
        if let Action::Chop { t } = self.action {
            let t = t + 1;
            self.action = if t >= CHOP_TOTAL { Action::Idle } else { Action::Chop { t } };
            return (t == CHOP_STRIKE).then(|| Deed::Chop(self.reach_point()));
        }
        if let Action::PickUp { t } = self.action {
            let t = t + 1;
            self.action = if t >= PICK_TOTAL { Action::Idle } else { Action::PickUp { t } };
            return (t == PICK_GRAB).then_some(Deed::PickUp);
        }
        if let Action::Eat { t } = self.action {
            let t = t + 1;
            self.action = if t >= EAT_TOTAL { Action::Idle } else { Action::Eat { t } };
            return (t == EAT_BITE).then_some(Deed::Eat);
        }
        if let Action::Scoop { t } = self.action {
            let t = t + 1;
            self.action = if t >= SCOOP_TOTAL { Action::Idle } else { Action::Scoop { t } };
            return (t == SCOOP_GRAB).then_some(Deed::DrawWater);
        }
        if let Action::Drink { t } = self.action {
            let t = t + 1;
            self.action = if t >= DRINK_TOTAL { Action::Idle } else { Action::Drink { t } };
            return (t == DRINK_SIP).then_some(Deed::Drink);
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
        if has_axe && !self.wading && input.pressed(Buttons::B) {
            // Swinging roots you in place; change facing first if a direction is held.
            let (ax, ay) = input.axis();
            self.facing = facing_for(self.facing, ax, ay);
            self.action = Action::Chop { t: 0 };
            self.moving = false;
            self.walk_timer = 0;
            return None;
        }
        if has_food && !self.wading && input.pressed(Buttons::Y) {
            self.action = Action::Eat { t: 0 };
            self.moving = false;
            self.walk_timer = 0;
            return None;
        }
        if has_water && !self.wading && input.pressed(Buttons::R) {
            self.action = Action::Drink { t: 0 };
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
            self.facing = before;
        }
        if !self.wading && input.pressed(Buttons::A) {
            // Crouch at the water-drawing spot and scoop some up.
            let (rx, ry) = self.reach_point();
            if map.is_drinking_water(rx, ry) {
                self.action = Action::Scoop { t: 0 };
                self.moving = false;
                self.walk_timer = 0;
                return None;
            }
        }
        self.walk(input, map);
        None
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

    fn walk(&mut self, input: &InputState, map: &WorldMap) {
        let (ax, ay) = input.axis();
        self.moving = ax != 0 || ay != 0;
        self.facing = facing_for(self.facing, ax, ay);

        let (fx, fy) = self.body.feet();
        self.wading = map.props_at_px(fx, fy).wading;

        let diagonal = ax != 0 && ay != 0;
        let speed = match (self.wading, diagonal) {
            (false, false) => SPEED_WALK,
            (false, true) => SPEED_WALK_DIAG,
            (true, false) => SPEED_WADE,
            (true, true) => SPEED_WADE_DIAG,
        };
        self.body.move_by(ax * speed, ay * speed, CORNER_ASSIST_PX, &|r| map.is_solid_rect(r));

        // Keep walking in place against walls, like classic action RPGs.
        self.walk_timer = if self.moving { self.walk_timer + 1 } else { 0 };
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
        let step_frames = if self.wading { STEP_FRAMES_WADE } else { STEP_FRAMES_WALK };
        // Start on a step frame so movement reads instantly.
        let phase = if self.moving { (self.walk_timer / step_frames + 1) % 4 } else { 0 };
        let (view, flip) = view_for(self.facing);
        // The 24x32 character canvas is offset from the 16x24 logical frame.
        let (cx, cy) = (sx - CHAR_ORIGIN.0, sy - CHAR_ORIGIN.1);

        if let Action::WakeUp { t } = self.action {
            if t < WAKE_KNEEL {
                self.draw_waking(list, (sx, sy), (cx, cy), depth, sprites, t, prio, shadows);
                return;
            }
        }

        if let Action::Chop { t } = self.action {
            self.draw_chop(list, (sx, sy), (cx, cy), depth, sprites, t, prio);
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

        if let Action::Eat { t } = self.action {
            self.draw_eating(list, (sx, sy), (cx, cy), depth, sprites, t, prio, shadows);
            return;
        }

        if let Action::Drink { t } = self.action {
            self.draw_drinking(list, (cx, cy), depth, sprites, t, prio);
            if shadows {
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

        if matches!(self.action, Action::WakeUp { t } if t < WAKE_STAND) {
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

    /// Holding a coconut up to the mouth and gnawing at it: the coconut
    /// sprite sits in the hands of the drinking pose and bobs as they chew.
    /// From behind, it is hidden by the head.
    #[allow(clippy::too_many_arguments)]
    fn draw_eating(&self, list: &mut DrawList, (sx, sy): (i32, i32), (cx, cy): (i32, i32), depth: i32, sprites: &SpriteIds, t: u32, prio: u8, shadows: bool) {
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
        let gfx = if bites == 0 { sprites.coconut[0] } else { sprites.coconut_bitten[bites - 1] };
        let nut = ObjEntry::new(cx + gx.round() as i32 - 4, cy + gy.round() as i32 - 5 + tug, gfx, OBJ_CRAB, prio).flipped(flip);
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

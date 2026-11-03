use super::body::Body;
use crate::assets::palettes::{OBJ_FX, OBJ_PLAYER, OBJ_SHADOW, OBJ_WADE, OBJ_WATER_FX};
use crate::assets::SpriteIds;
use crate::assets::character::pose::AxeArt;
use crate::assets::character::{CHAR_ORIGIN, CHAR_W};
use crate::ppu::banks::SpriteId;
use crate::common::input::{Buttons, InputState};
use crate::common::math::{Dir, Rect};
use crate::ppu::oam::ObjEntry;
use crate::render::camera::Camera;
use crate::render::sprites::DrawList;
use crate::world::WorldMap;

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Idle,
    /// Swinging the axe; `t` counts frames since the swing started.
    Chop { t: u32 },
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

    /// Advances one frame. Returns the world point the axe hits on the frame
    /// a swing connects.
    pub fn update(&mut self, input: &InputState, map: &WorldMap, has_axe: bool) -> Option<(i32, i32)> {
        if let Action::Chop { t } = self.action {
            let t = t + 1;
            self.action = if t >= CHOP_TOTAL { Action::Idle } else { Action::Chop { t } };
            return (t == CHOP_STRIKE).then(|| self.strike_point());
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
        self.walk(input, map);
        None
    }

    /// Where the axe blade lands, just in front of the feet.
    fn strike_point(&self) -> (i32, i32) {
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

        if let Action::Chop { t } = self.action {
            self.draw_chop(list, (sx, sy), (cx, cy), depth, sprites, t, prio);
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

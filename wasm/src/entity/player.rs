use super::body::Body;
use crate::assets::palettes::{OBJ_FX, OBJ_PLAYER, OBJ_SHADOW, OBJ_WADE, OBJ_WATER_FX};
use crate::assets::{PlayerFrame, PlayerSprites, SpriteIds};
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

    pub fn draw(&self, list: &mut DrawList, camera: &Camera, sprites: &SpriteIds, clock: u32, shadows: bool) {
        let (wx, wy) = self.body.px();
        let (sx, sy) = camera.to_screen(wx, wy);
        let depth = self.depth();
        let step_frames = if self.wading { STEP_FRAMES_WADE } else { STEP_FRAMES_WALK };
        // Start on a step frame so movement reads instantly.
        let phase = if self.moving { (self.walk_timer / step_frames + 1) % 4 } else { 0 };
        let (frame, flip) = pick_frame(&sprites.player, self.facing, phase);

        if let Action::Chop { t } = self.action {
            self.draw_chop(list, (sx, sy), depth, sprites, t);
            if shadows {
                list.shadow(ObjEntry::new(sx, sy + SPRITE_H - 5, sprites.shadow_player, OBJ_SHADOW, ACTOR_PRIORITY));
            }
            return;
        }

        if self.wading {
            // Foam ring first (in front), then the body above the waterline,
            // then the submerged legs in a blended palette.
            let ripple = sprites.ripples[(clock / 10) as usize % sprites.ripples.len()];
            list.actor(depth, ObjEntry::new(sx - 4, sy + 14, ripple, OBJ_WATER_FX, ACTOR_PRIORITY));
            list.actor(depth, ObjEntry::new(sx, sy, frame.top, OBJ_PLAYER, ACTOR_PRIORITY).flipped(flip));
            list.actor(depth, ObjEntry::new(sx, sy, frame.legs, OBJ_WADE, ACTOR_PRIORITY).flipped(flip));
        } else {
            list.actor(depth, ObjEntry::new(sx, sy, frame.full, OBJ_PLAYER, ACTOR_PRIORITY).flipped(flip));
            if shadows {
                list.shadow(ObjEntry::new(sx, sy + SPRITE_H - 5, sprites.shadow_player, OBJ_SHADOW, ACTOR_PRIORITY));
            }
        }
    }
}

impl Player {
    /// A horizontal swing at trunk height: the axe is pulled back to one side,
    /// sweeps past the body and ends pointing forward. Body pose and axe are
    /// separate sprites; parts pushed before the body appear in front of it.
    fn draw_chop(&self, list: &mut DrawList, (sx, sy): (i32, i32), depth: i32, sprites: &SpriteIds, t: u32) {
        #[derive(Clone, Copy, PartialEq)]
        enum Pose {
            Back,
            Sweep,
            Strike,
        }
        #[derive(Clone, Copy)]
        enum Axe {
            Side,
            Up,
            Diag,
        }
        let pose = if t >= CHOP_STRIKE {
            Pose::Strike
        } else if t >= CHOP_SWEEP {
            Pose::Sweep
        } else {
            Pose::Back
        };

        let chop = &sprites.player.chop;
        let i = (pose != Pose::Back) as usize;
        let (body, body_flip) = match self.facing {
            Dir::Down => (chop.down[i], false),
            Dir::Up => (chop.up[i], false),
            d => (chop.side[i], d == Dir::Left),
        };

        // (dx, dy, axe image, flip_h, flip_v, in front of body). The handle
        // end is placed under the pose's hands; when the axe is in front,
        // the hands overlay is drawn over it so they grip the handle.
        let (dx, dy, axe, fh, fv, front) = match (self.facing, pose) {
            // Side view: back (pointing behind) → toward the camera → forward.
            (Dir::Right, Pose::Back) => (-13, 6, Axe::Side, true, false, false),
            (Dir::Right, Pose::Sweep) => (5, 12, Axe::Up, false, true, true),
            (Dir::Right, Pose::Strike) => (12, 5, Axe::Side, false, false, false),
            (Dir::Left, Pose::Back) => (13, 6, Axe::Side, false, false, false),
            (Dir::Left, Pose::Sweep) => (-5, 12, Axe::Up, false, true, true),
            (Dir::Left, Pose::Strike) => (-12, 5, Axe::Side, true, false, false),
            // Facing the camera: from the right hand side (screen left) across the front.
            (Dir::Down, Pose::Back) => (-13, 7, Axe::Side, true, false, false),
            (Dir::Down, Pose::Sweep) => (-4, 11, Axe::Diag, true, true, true),
            (Dir::Down, Pose::Strike) => (2, 14, Axe::Up, false, true, true),
            // Facing away: from the right hand side (screen right) sweeping up behind.
            (Dir::Up, Pose::Back) => (12, 6, Axe::Side, false, false, false),
            (Dir::Up, Pose::Sweep) => (8, 1, Axe::Diag, false, false, false),
            (Dir::Up, Pose::Strike) => (2, -4, Axe::Up, false, false, false),
        };
        let gfx = match axe {
            Axe::Side => sprites.axe_side,
            Axe::Up => sprites.axe_up,
            Axe::Diag => sprites.axe_diag,
        };
        let mut axe = ObjEntry::new(sx + dx, sy + dy, gfx, OBJ_FX, ACTOR_PRIORITY).flipped(fh);
        axe.flip_v = fv;

        if (CHOP_SWEEP..CHOP_STRIKE + SWOOSH_FRAMES).contains(&t) {
            let (ox, oy) = match self.facing {
                Dir::Right => (-6, 8),
                Dir::Left => (-10, 8),
                Dir::Down => (-8, 10),
                Dir::Up => (-8, -4),
            };
            let swoosh = sprites.swoosh[self.facing as usize];
            list.actor(depth, ObjEntry::new(sx + ox, sy + oy, swoosh, OBJ_WATER_FX, ACTOR_PRIORITY));
        }
        if front {
            list.actor(depth, ObjEntry::new(sx, sy, body.hands, OBJ_PLAYER, ACTOR_PRIORITY).flipped(body_flip));
            list.actor(depth, axe);
        }
        list.actor(depth, ObjEntry::new(sx, sy, body.body, OBJ_PLAYER, ACTOR_PRIORITY).flipped(body_flip));
        if !front {
            list.actor(depth, axe);
        }
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

/// Front/back cycle: stand, step, stand, step mirrored.
/// Side cycle: stand, step A, stand, step B.
fn pick_frame(s: &PlayerSprites, facing: Dir, phase: u32) -> (PlayerFrame, bool) {
    match facing {
        Dir::Down | Dir::Up => {
            let set = if facing == Dir::Down { &s.down } else { &s.up };
            (set[(phase % 2) as usize], phase == 3)
        }
        Dir::Right | Dir::Left => {
            let f = match phase {
                1 => s.side[1],
                3 => s.side[2],
                _ => s.side[0],
            };
            (f, facing == Dir::Left)
        }
    }
}

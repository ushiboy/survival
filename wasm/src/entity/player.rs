use super::body::Body;
use crate::assets::palettes::{OBJ_PLAYER, OBJ_SHADOW, OBJ_WADE, OBJ_WATER_FX};
use crate::assets::{PlayerFrame, PlayerSprites, SpriteIds};
use crate::common::input::InputState;
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

#[derive(Clone, Debug)]
pub struct Player {
    pub body: Body,
    pub facing: Dir,
    pub moving: bool,
    pub wading: bool,
    walk_timer: u32,
}

impl Player {
    /// Places the player so their feet stand on the given cell's top-left.
    pub fn at_feet(px: i32, py: i32) -> Self {
        Self::new(px, py - (SPRITE_H - 16))
    }

    pub fn new(px: i32, py: i32) -> Self {
        Self { body: Body::new(px, py, HITBOX), facing: Dir::Down, moving: false, wading: false, walk_timer: 0 }
    }

    pub fn update(&mut self, input: &InputState, map: &WorldMap) {
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

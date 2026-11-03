use super::body::Body;
use crate::assets::PlayerSprites;
use crate::assets::SpriteIds;
use crate::assets::palettes::{SPR_PLAYER, SPR_WATER};
use crate::common::input::InputState;
use crate::common::math::{Dir, Rect};
use crate::ppu::oam::{Oam, attr};
use crate::render::camera::Camera;
use crate::render::sprites::{push_16x8, push_16x16};
use crate::world::WorldMap;

/// Feet-only collision box (top-down games collide at the feet so heads can
/// overlap walls above them).
const HITBOX: Rect = Rect::new(3, 10, 10, 6);
/// Speeds in 8.8 fixed point per frame.
const SPEED_WALK: i32 = 320; // 1.25 px
const SPEED_WALK_DIAG: i32 = 226; // ~1.25 / sqrt(2)
const SPEED_WADE: i32 = 176;
const SPEED_WADE_DIAG: i32 = 124;
const CORNER_ASSIST_PX: i32 = 6;
/// Frames per walk-animation step.
const STEP_FRAMES_WALK: u32 = 8;
const STEP_FRAMES_WADE: u32 = 12;
/// Where the ripple ring sits relative to the sprite.
const RIPPLE_OFFSET_Y: i32 = 11;

#[derive(Clone, Debug)]
pub struct Player {
    pub body: Body,
    pub facing: Dir,
    pub moving: bool,
    pub wading: bool,
    walk_timer: u32,
}

impl Player {
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

    pub fn center(&self) -> (i32, i32) {
        let (x, y) = self.body.px();
        (x + 8, y + 8)
    }

    pub fn sort_y(&self) -> i32 {
        self.body.px().1
    }

    pub fn draw(&self, oam: &mut Oam, camera: &Camera, sprites: &SpriteIds, clock: u32) {
        let (wx, wy) = self.body.px();
        let (sx, sy) = camera.to_screen(wx, wy);
        let step_frames = if self.wading { STEP_FRAMES_WADE } else { STEP_FRAMES_WALK };
        // Start on a step frame so movement reads instantly.
        let phase = if self.moving { (self.walk_timer / step_frames + 1) % 4 } else { 0 };
        let (frame, flip) = pick_frame(&sprites.player, self.facing, phase);
        let tiles = if self.wading { frame.wade } else { frame.normal };

        if self.wading {
            // Pushed first so it wins sprite priority over the body.
            let ripple = sprites.ripple[(clock / 16) as usize % 2];
            push_16x8(oam, ripple, sx, sy + RIPPLE_OFFSET_Y, SPR_WATER);
        }
        push_16x16(oam, tiles, sx, sy, SPR_PLAYER | flip);
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

/// Front/back walk cycle: stand, step, stand, step mirrored.
/// Side walk cycle: stand, step.
fn pick_frame(s: &PlayerSprites, facing: Dir, phase: u32) -> (crate::assets::PlayerFrame, u8) {
    let step = phase % 2 == 1;
    match facing {
        Dir::Down | Dir::Up => {
            let set = if facing == Dir::Down { &s.down } else { &s.up };
            let flip = if phase == 3 { attr::FLIP_H } else { 0 };
            (set[step as usize], flip)
        }
        Dir::Right => (s.side[step as usize], 0),
        Dir::Left => (s.side[step as usize], attr::FLIP_H),
    }
}

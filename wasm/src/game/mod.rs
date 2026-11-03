//! Top-level orchestration: owns every subsystem and runs one frame at a time.
//! `step` advances simulation; `render` produces the picture. They are kept
//! separate so the host can run fixed-rate updates independent of display.

pub mod state;

use crate::assets::Assets;
use crate::assets::palettes::base_palette;
use crate::common::input::{Buttons, InputState};
use crate::entity::{Crab, Player};
use crate::ppu::palette::{darken, nightify};
use crate::ppu::{FrameState, HudLayer, Ppu};
use crate::ppu::oam::Oam;
use crate::render::camera::Camera;
use crate::render::hud::{self, HUD_HEIGHT, HUD_ROWS};
use crate::render::tileset::{VisualMap, build_visual_map};
use crate::world::{WorldMap, island};
use state::{DayPhase, GameClock, Vitals};

const START_HOUR: u32 = 7;

pub struct Game {
    assets: Assets,
    map: WorldMap,
    visual: VisualMap,
    player: Player,
    crabs: Vec<Crab>,
    camera: Camera,
    clock: GameClock,
    vitals: Vitals,
    input: InputState,
    paused: bool,
    frame_count: u32,
    frame: FrameState,
    ppu: Ppu,
}

impl Game {
    pub fn new(seed: u32) -> Self {
        let mut assets = Assets::load();
        let map = island::generate(seed);
        let visual = build_visual_map(&map, &mut assets, seed);
        let player = Player::new(map.spawn.0, map.spawn.1);
        let crabs = map
            .crab_spawns
            .iter()
            .enumerate()
            .map(|(i, &(x, y))| Crab::new(x, y, seed.wrapping_add(i as u32 * 31)))
            .collect();
        let frame = FrameState {
            scroll_x: 0,
            scroll_y: 0,
            palette: base_palette(),
            oam: Oam::default(),
            hud: HudLayer::new(HUD_ROWS, assets.hud.blank),
            clock: 0,
        };
        let mut game = Self {
            assets,
            map,
            visual,
            player,
            crabs,
            camera: Camera::new(HUD_HEIGHT),
            clock: GameClock::new(1, START_HOUR),
            vitals: Vitals::new(),
            input: InputState::default(),
            paused: false,
            frame_count: 0,
            frame,
            ppu: Ppu::new(),
        };
        game.camera.follow(game.player.center(), game.map.pixel_size());
        game
    }

    /// Advances the simulation by one frame (1/60 s).
    pub fn step(&mut self, buttons: u8) {
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

        self.frame_count = self.frame_count.wrapping_add(1);
        if self.clock.tick() {
            self.vitals.on_minute(&self.clock);
        }
        self.player.update(&self.input, &self.map);
        let pc = self.player.center();
        for crab in &mut self.crabs {
            crab.update(&self.map, pc);
        }
        self.camera.follow(pc, self.map.pixel_size());
    }

    /// Composes PPU state for this frame and renders it to the RGBA buffer.
    pub fn render(&mut self) {
        let f = &mut self.frame;
        f.clock = self.frame_count;
        f.scroll_x = self.camera.x;
        f.scroll_y = self.camera.y;
        let base = base_palette();
        f.palette = match self.clock.phase() {
            DayPhase::Day => base,
            DayPhase::Twilight => base.map_colors(|c| darken(c, 1)),
            DayPhase::Night => base.map_colors(nightify),
        };

        f.oam.clear();
        hud::draw(&mut f.hud, &mut f.oam, &self.assets.hud, &self.assets.sprites, &self.vitals, &self.clock);
        let world_start = f.oam.len();

        // Nearer (lower on screen) actors first: lower OAM index = in front.
        enum Actor<'a> {
            Player(&'a Player),
            Crab(&'a Crab),
        }
        let mut actors: Vec<(i32, Actor)> = Vec::with_capacity(self.crabs.len() + 1);
        actors.push((self.player.sort_y(), Actor::Player(&self.player)));
        actors.extend(self.crabs.iter().map(|c| (c.sort_y(), Actor::Crab(c))));
        actors.sort_by(|a, b| b.0.cmp(&a.0));
        for (_, actor) in &actors {
            match actor {
                Actor::Player(p) => p.draw(&mut f.oam, &self.camera, &self.assets.sprites, self.frame_count),
                Actor::Crab(c) => c.draw(&mut f.oam, &self.camera, &self.assets.sprites),
            }
        }
        // Turn sprite dropout into flicker when the scanline limit was hit.
        if self.ppu.overflowed {
            f.oam.rotate_from(world_start, self.frame_count as usize * 4);
        }

        self.ppu.render(&self.assets.vram, &self.frame, &self.visual);
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
}

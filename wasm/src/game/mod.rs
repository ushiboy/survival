//! Top-level orchestration: owns every subsystem and runs one frame at a time.
//! `step` advances simulation; `render` produces the picture. They are kept
//! separate so the host can run fixed-rate updates independent of display.

pub mod state;

use crate::assets::Assets;
use crate::assets::palettes::{HUD_LIGHT_MASK, base_cgram};
use crate::common::input::{Buttons, InputState};
use crate::entity::{Crab, Gull, Player};
use crate::ppu::effects::{ColorMath, MathOp, MathSource, WindowRegion, layer};
use crate::ppu::palette::Cgram;
use crate::ppu::{BgLayer, FrameState, Ppu, SCREEN_H, SCREEN_W};
use crate::render::camera::Camera;
use crate::render::hud::{self, HUD_LINES};
use crate::render::lighting::{self, Light, Lighting};
use crate::render::sprites::DrawList;
use crate::render::tileset::{VisualMap, build_visual_map};
use crate::world::{WorldMap, island};
use state::{GameClock, Inventory, Vitals};

const START_HOUR: u32 = 7;
/// Above this darkness the sun casts no shadows and night darkness begins.
const NIGHT_THRESHOLD: f32 = 0.35;
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

pub struct Game {
    assets: Assets,
    map: WorldMap,
    visual: VisualMap,
    player: Player,
    crabs: Vec<Crab>,
    gulls: Vec<Gull>,
    camera: Camera,
    clock: GameClock,
    vitals: Vitals,
    inventory: Inventory,
    input: InputState,
    paused: bool,
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
        let visual = build_visual_map(&map, &mut assets);
        let player = Player::at_feet(map.spawn.0, map.spawn.1);
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
            visual,
            player,
            crabs,
            gulls,
            camera: Camera::new(0),
            clock: GameClock::new(1, START_HOUR),
            vitals: Vitals::new(),
            inventory: Inventory::default(),
            input: InputState::default(),
            paused: false,
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
        if self.input.pressed(Buttons::Y) {
            // Debug: give / take the lantern until items exist.
            self.inventory.lantern = !self.inventory.lantern;
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
        for gull in &mut self.gulls {
            gull.update();
        }
        self.camera.follow(pc, self.map.pixel_size());
    }

    /// Sets up PPU state for this frame and renders it to the RGBA buffer.
    pub fn render(&mut self) {
        let lighting = Lighting::at(self.clock.minute_f());
        let night = lighting.darkness > NIGHT_THRESHOLD;
        let f = &mut self.frame;
        f.clock = self.frame_count;
        f.scroll_x = self.camera.x;
        f.scroll_y = self.camera.y;

        // CGRAM: base palette → water cycling → time-of-day tint.
        f.cgram.clone_from(&self.base_cgram);
        lighting::cycle_water(&mut f.cgram, self.frame_count);
        lighting.apply(&mut f.cgram);

        // Colour math + HDMA tables.
        if night {
            let (cx, cy) = self.player.center();
            let light = self.inventory.lantern.then(|| Light { center: self.camera.to_screen(cx, cy) });
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
        self.player.draw(list, &self.camera, sprites, self.frame_count, shadows);
        for crab in &self.crabs {
            crab.draw(list, &self.camera, sprites, night);
        }
        if !night {
            for gull in &self.gulls {
                gull.draw(list, &self.camera, sprites, shadows);
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

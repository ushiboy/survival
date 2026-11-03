//! WebAssembly entry point. This file is only the JS-facing facade; all game
//! logic lives in plain Rust modules so it can be unit-tested natively.

pub mod assets;
pub mod common;
pub mod entity;
pub mod game;
pub mod items;
pub mod ppu;
pub mod render;
pub mod world;

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn screen_width() -> u32 {
    ppu::SCREEN_W as u32
}

#[wasm_bindgen]
pub fn screen_height() -> u32 {
    ppu::SCREEN_H as u32
}

#[wasm_bindgen]
pub struct GameHandle {
    game: game::Game,
}

#[wasm_bindgen]
impl GameHandle {
    #[wasm_bindgen(constructor)]
    pub fn new(seed: u32) -> GameHandle {
        console_error_panic_hook::set_once();
        GameHandle { game: game::Game::new(seed) }
    }

    /// Advance one frame with the given SNES joypad word.
    pub fn step(&mut self, buttons: u16) {
        self.game.step(buttons);
    }

    pub fn render(&mut self) {
        self.game.render();
    }

    /// Pointer to the 256x224 RGBA frame in wasm memory.
    pub fn frame_ptr(&self) -> *const u8 {
        self.game.rgba().as_ptr()
    }

    pub fn set_sprite_limit(&mut self, on: bool) {
        self.game.set_sprite_limit(on);
    }

    /// Compact status for the debug panel:
    /// `[tileX, tileY, day, minuteOfDay, paused, vramBytes]`.
    pub fn status(&self) -> Vec<i32> {
        let (tx, ty) = self.game.player_tile();
        let c = self.game.clock();
        vec![tx, ty, c.day as i32, c.minute as i32, self.game.is_paused() as i32, self.game.vram_usage() as i32]
    }
}

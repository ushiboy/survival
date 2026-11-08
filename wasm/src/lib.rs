//! WebAssembly entry point. This file is only the JS-facing facade; all game
//! logic lives in plain Rust modules so it can be unit-tested natively.

pub mod assets;
pub mod common;
pub mod crafting;
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

/// Debug: every item's name, indexed as `GameHandle::debug_give` takes them.
#[wasm_bindgen]
pub fn debug_item_names() -> Vec<String> {
    items::ItemKind::ALL.iter().map(|k| k.name().to_string()).collect()
}

/// Debug: how many of the item at that index fit in one stack.
#[wasm_bindgen]
pub fn debug_item_max_stack(index: usize) -> u32 {
    items::ItemKind::ALL.get(index).map_or(1, |k| k.max_stack())
}

#[wasm_bindgen]
pub struct GameHandle {
    game: game::Game,
}

#[wasm_bindgen]
impl GameHandle {
    /// A new game at the title screen. `save` is the saved game the page
    /// keeps (JSON), offered as CONTINUE if it can be read.
    #[wasm_bindgen(constructor)]
    pub fn new(seed: u32, save: Option<String>) -> GameHandle {
        console_error_panic_hook::set_once();
        GameHandle { game: game::Game::with_save(seed, save.as_deref()) }
    }

    /// Advance one frame with the given SNES joypad word.
    pub fn step(&mut self, buttons: u16) {
        self.game.step(buttons);
    }

    /// What to do with the saved game after this frame, if anything: the
    /// JSON to keep (replacing what was kept), or an empty string to erase
    /// it.
    pub fn take_save_op(&mut self) -> Option<String> {
        self.game.take_save_op().map(|op| match op {
            game::SaveOp::Write(json) => json,
            game::SaveOp::Erase => String::new(),
        })
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

    /// Debug: puts the castaway in the next outfit preset.
    pub fn debug_cycle_outfit(&mut self) {
        self.game.debug_cycle_outfit();
    }

    /// Debug: puts up to `count` of the item at `index` (in
    /// `debug_item_names`) in the bag; returns how many fitted.
    pub fn debug_give(&mut self, index: usize, count: u32) -> u32 {
        items::ItemKind::ALL.get(index).map_or(0, |&k| self.game.debug_give_n(k, count))
    }

    /// Debug: sets health, food and water (0..=64); a negative value leaves
    /// that one as it is.
    pub fn debug_set_vitals(&mut self, hp: i32, food: i32, water: i32) {
        let v = |n: i32| (n >= 0).then_some(n as u32);
        self.game.debug_set_vitals(v(hp), v(food), v(water));
    }

    /// Debug: makes every recipe known.
    pub fn debug_learn_recipes(&mut self) {
        self.game.debug_learn_recipes();
    }

    /// Compact status for the debug panel:
    /// `[tileX, tileY, day, minuteOfDay, paused, vramBytes]`.
    pub fn status(&self) -> Vec<i32> {
        let (tx, ty) = self.game.player_tile();
        let c = self.game.clock();
        vec![tx, ty, c.day as i32, c.minute as i32, self.game.is_paused() as i32, self.game.vram_usage() as i32]
    }
}

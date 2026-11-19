//! The end: the castaway has collapsed, or escaped the island. On a black
//! screen, BG3 tells the day it happened and how long they lasted (and,
//! escaped, what they made and found), then asks for START to begin
//! again.

use crate::assets::HudTiles;
use crate::assets::palettes::{HUD_DIM, HUD_TEXT};
use crate::game::Records;
use crate::game::state::GameClock;
use crate::ppu::Bg3Layer;
use crate::render::hud::HUD_COLS;

/// Rows of the BG3 map the screen covers (the HUD and the bag screen's).
const ROWS: usize = 28;

fn centred(bg3: &mut Bg3Layer, tiles: &HudTiles, row: usize, s: &str, palette: u8) {
    let col = (HUD_COLS - s.len()) / 2;
    for (i, c) in s.chars().enumerate() {
        bg3.put(col + i, row, tiles.glyph(c), palette);
    }
}

/// Draws the end screen: `lasted` is the game minutes survived, and
/// `prompt` whether to show PRESS START this frame.
pub fn draw(bg3: &mut Bg3Layer, tiles: &HudTiles, clock: &GameClock, lasted: u32, prompt: bool) {
    bg3.clear_rect(0, 0, HUD_COLS, ROWS);
    centred(bg3, tiles, 10, "COLLAPSED", HUD_TEXT);
    let (h, m) = clock.hour_minute();
    let (ampm, h12) = if h < 12 { ("AM", h) } else { ("PM", h - 12) };
    let h12 = if h12 == 0 { 12 } else { h12 };
    centred(bg3, tiles, 13, &format!("DAY {:02}  {ampm} {h12:02}:{m:02}", clock.day), HUD_DIM);
    let (days, hours) = (lasted / (24 * 60), lasted / 60 % 24);
    let plural = |n: u32, word: &str| format!("{n} {word}{}", if n == 1 { "" } else { "S" });
    centred(bg3, tiles, 15, &format!("LASTED {} {}", plural(days, "DAY"), plural(hours, "HOUR")), HUD_TEXT);
    if prompt {
        centred(bg3, tiles, 20, "PRESS START", HUD_TEXT);
    }
}

/// Draws the records every escape ends on: the day it came, the kinds of
/// things made of all there are, and the recipes found of all there are.
pub fn draw_escape(bg3: &mut Bg3Layer, tiles: &HudTiles, records: &Records, prompt: bool) {
    bg3.clear_rect(0, 0, HUD_COLS, ROWS);
    centred(bg3, tiles, 8, "RESCUED", HUD_TEXT);
    centred(bg3, tiles, 12, &format!("ESCAPED ON DAY {}", records.day), HUD_TEXT);
    centred(bg3, tiles, 14, &format!("ITEMS MADE    {:>2} / {:>2}", records.made, records.makeable), HUD_TEXT);
    centred(bg3, tiles, 16, &format!("RECIPES FOUND {:>2} / {:>2}", records.recipes_known, records.recipes), HUD_TEXT);
    if prompt {
        centred(bg3, tiles, 20, "PRESS START", HUD_TEXT);
    }
}

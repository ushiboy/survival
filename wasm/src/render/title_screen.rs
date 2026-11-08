//! The title screen: the castaway lying on the beach where the story
//! opens, the game's name over it in large letters, and a menu to begin
//! (START, and CONTINUE below it when there is a saved game).

use crate::assets::{HudTiles, TITLE};
use crate::assets::palettes::{HUD_SKY, HUD_TEXT};
use crate::ppu::Bg3Layer;
use crate::render::hud::HUD_COLS;

/// Rows of the BG3 map the screen covers (the HUD and the bag screen's).
const ROWS: usize = 28;
const TITLE_ROW: usize = 6;
const MENU_ROW: usize = 18;
/// What the menu offers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TitleItem {
    /// Begin a new game.
    Start,
    /// Carry on from the saved game.
    Continue,
}

impl TitleItem {
    fn label(self) -> &'static str {
        match self {
            TitleItem::Start => "START",
            TitleItem::Continue => "CONTINUE",
        }
    }
}

/// Draws the title and the menu (`items`, top to bottom) with `choice`
/// marked; while `confirm` is set, asks instead whether to begin anew over
/// the saved game, with YES marked if it holds true. The marker shows when
/// `blink` is on.
pub fn draw(bg3: &mut Bg3Layer, tiles: &HudTiles, items: &[TitleItem], choice: usize, confirm: Option<bool>, blink: bool) {
    bg3.clear_rect(0, 0, HUD_COLS, ROWS);
    let col = (HUD_COLS - TITLE.len() * 2) / 2;
    for (i, c) in TITLE.chars().enumerate() {
        let t = tiles.title_glyph(c);
        for (q, &tile) in t.iter().enumerate() {
            bg3.put(col + i * 2 + q % 2, TITLE_ROW + q / 2, tile, HUD_SKY);
        }
    }
    if let Some(yes) = confirm {
        centred(bg3, tiles, MENU_ROW, "START A NEW GAME?");
        centred(bg3, tiles, MENU_ROW + 1, "THE SAVED GAME WILL BE LOST.");
        let row = MENU_ROW + 3;
        let (yes_col, no_col) = (HUD_COLS / 2 - 5, HUD_COLS / 2 + 3);
        text(bg3, tiles, yes_col, row, "YES");
        text(bg3, tiles, no_col, row, "NO");
        if blink {
            bg3.put(if yes { yes_col } else { no_col } - 2, row, tiles.glyph('>'), HUD_TEXT);
        }
        return;
    }
    let width = items.iter().map(|s| s.label().len()).max().unwrap_or(0);
    let col = (HUD_COLS - width) / 2;
    for (i, item) in items.iter().enumerate() {
        let row = MENU_ROW + i * 2;
        text(bg3, tiles, col, row, item.label());
        if i == choice && blink {
            bg3.put(col - 2, row, tiles.glyph('>'), HUD_TEXT);
        }
    }
}

fn text(bg3: &mut Bg3Layer, tiles: &HudTiles, col: usize, row: usize, s: &str) {
    for (k, c) in s.chars().enumerate() {
        bg3.put(col + k, row, tiles.glyph(c), HUD_TEXT);
    }
}

fn centred(bg3: &mut Bg3Layer, tiles: &HudTiles, row: usize, s: &str) {
    text(bg3, tiles, (HUD_COLS - s.len()) / 2, row, s);
}

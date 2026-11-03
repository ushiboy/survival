//! Status bar: hearts, food & water gauges, day and clock.

use crate::assets::palettes::{BG_SAND, BG_SHORE, SPR_UI};
use crate::assets::{HUD_TEXT_PALETTE, HudTiles, SpriteIds};
use crate::game::state::{GameClock, Vitals};
use crate::ppu::HudLayer;
use crate::ppu::oam::{Oam, OamEntry, attr};

pub const HUD_ROWS: usize = 4;
pub const HUD_HEIGHT: i32 = (HUD_ROWS * 8) as i32;

fn text(hud: &mut HudLayer, tiles: &HudTiles, col: usize, row: usize, s: &str) {
    for (i, c) in s.chars().enumerate() {
        hud.put(col + i, row, tiles.glyph(c), HUD_TEXT_PALETTE);
    }
}

fn gauge(hud: &mut HudLayer, col: usize, row: usize, value: u8, max: u8, full: u16, empty: u16, palette: u8) {
    for i in 0..max {
        hud.put(col + i as usize, row, if i < value { full } else { empty }, palette);
    }
}

pub fn draw(hud: &mut HudLayer, oam: &mut Oam, tiles: &HudTiles, sprites: &SpriteIds, vitals: &Vitals, clock: &GameClock) {
    hud.fill(tiles.blank);

    // Hearts are sprites so they can be red; the HUD tiles only have one palette set.
    for i in 0..vitals.max_hp {
        let tile = if i < vitals.hp { sprites.heart_full } else { sprites.heart_empty };
        oam.push(OamEntry { x: 16 + i as i32 * 9, y: 8, tile, attr: SPR_UI | attr::HUD_LAYER });
    }

    hud.put(2, 2, tiles.food_icon, BG_SAND);
    gauge(hud, 3, 2, vitals.food, Vitals::GAUGE_MAX, tiles.food_full, tiles.food_empty, BG_SAND);
    hud.put(12, 2, tiles.water_icon, BG_SHORE);
    gauge(hud, 13, 2, vitals.water, Vitals::GAUGE_MAX, tiles.water_full, tiles.water_empty, BG_SHORE);

    text(hud, tiles, 23, 1, &format!("DAY {:02}", clock.day));
    let (h, m) = clock.hour_minute();
    let (ampm, h12) = if h < 12 { ("AM", h) } else { ("PM", h - 12) };
    let h12 = if h12 == 0 { 12 } else { h12 };
    text(hud, tiles, 22, 2, &format!("{ampm} {h12:02}:{m:02}"));
}

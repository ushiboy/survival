//! HUD overlay on BG3: health, food & water gauges, day and clock.
//! Drawn over the playfield with no backing panel, as 16-bit games did.

use crate::assets::HudTiles;
use crate::assets::palettes::{HUD_FOOD, HUD_HEART, HUD_SKY, HUD_TEXT, HUD_WATER};
use crate::game::state::{GameClock, Vitals};
use crate::ppu::Bg3Layer as HudLayer;

const GAUGE_TILES: usize = 8;
/// The HUD occupies the top-left of the BG3 map.
pub const HUD_COLS: usize = 32;
pub const HUD_ROWS: usize = 4;
/// Lines at the top of the screen where BG3 shows the HUD.
pub const HUD_LINES: usize = HUD_ROWS * 8;

fn text(hud: &mut HudLayer, tiles: &HudTiles, col: usize, row: usize, s: &str) {
    for (i, c) in s.chars().enumerate() {
        hud.put(col + i, row, tiles.glyph(c), HUD_TEXT);
    }
}

/// Pixel-precise gauge: `value` out of `max`, drawn across GAUGE_TILES tiles.
fn gauge(hud: &mut HudLayer, tiles: &HudTiles, col: usize, row: usize, value: u32, max: u32, palette: u8) {
    let px = (value * (GAUGE_TILES as u32 * 8) / max.max(1)) as usize;
    hud.put(col, row, tiles.gauge_cap_left, palette);
    for i in 0..GAUGE_TILES {
        let fill = px.saturating_sub(i * 8).min(8);
        hud.put(col + 1 + i, row, tiles.gauge[fill], palette);
    }
    hud.put(col + 1 + GAUGE_TILES, row, tiles.gauge_cap_right, palette);
}

pub fn draw(hud: &mut HudLayer, tiles: &HudTiles, vitals: &Vitals, clock: &GameClock, night: bool) {
    hud.clear_rect(0, 0, HUD_COLS, HUD_ROWS);
    hud.put(1, 1, tiles.heart, HUD_HEART);
    gauge(hud, tiles, 2, 1, vitals.hp, vitals.max_hp, HUD_HEART);
    hud.put(1, 2, tiles.food, HUD_FOOD);
    gauge(hud, tiles, 2, 2, vitals.food, Vitals::MAX, HUD_FOOD);
    hud.put(1, 3, tiles.water, HUD_WATER);
    gauge(hud, tiles, 2, 3, vitals.water, Vitals::MAX, HUD_WATER);

    hud.put(23, 1, if night { tiles.moon } else { tiles.sun }, HUD_SKY);
    text(hud, tiles, 25, 1, &format!("DAY {:02}", clock.day));
    let (h, m) = clock.hour_minute();
    let (ampm, h12) = if h < 12 { ("AM", h) } else { ("PM", h - 12) };
    let h12 = if h12 == 0 { 12 } else { h12 };
    text(hud, tiles, 23, 2, &format!("{ampm} {h12:02}:{m:02}"));
}

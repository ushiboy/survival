//! Draws the bag screen: a full-screen subscreen below the HUD. The world
//! layers are switched off; BG3 draws the slots and text, and sprites the
//! item icons and cursor (BG3's priority bit is cleared so they show in
//! front of it).

use crate::assets::palettes::{HUD_MENU, HUD_TEXT, OBJ_FX};
use crate::assets::{HudTiles, SpriteIds};
use crate::game::bag::{BAG_H, BAG_W, Bag, footprint};
use crate::game::menu::{BagMenu, Mode, commands_for};
use crate::ppu::Bg3Layer;
use crate::ppu::oam::ObjEntry;
use crate::render::hud::{HUD_COLS, HUD_ROWS};
use crate::render::sprites::DrawList;

/// The bag grid's top-left (BG3 tile) and its cells in pixels.
const GRID_COL: usize = 2;
const GRID_ROW: usize = 8;
const CELL: i32 = 16;
/// Rows of the BG3 map the screen uses.
const LAST_ROW: usize = 27;
const INFO_COL: usize = 17;

fn text(bg3: &mut Bg3Layer, tiles: &HudTiles, col: usize, row: usize, s: &str) {
    for (i, c) in s.chars().enumerate() {
        bg3.put(col + i, row, tiles.glyph(c), HUD_TEXT);
    }
}

/// Clears the screen's area of BG3 (when the bag closes).
pub fn clear(bg3: &mut Bg3Layer) {
    bg3.clear_rect(0, HUD_ROWS, HUD_COLS, LAST_ROW + 1 - HUD_ROWS);
}

fn cell_px(x: i32, y: i32) -> (i32, i32) {
    ((GRID_COL as i32) * 8 + x * CELL, (GRID_ROW as i32) * 8 + y * CELL)
}

pub fn draw(bg3: &mut Bg3Layer, list: &mut DrawList, tiles: &HudTiles, sprites: &SpriteIds, bag: &Bag, menu: &BagMenu, clock: u32) {
    clear(bg3);
    let used: i32 = bag.items().iter().map(|p| p.size().0 * p.size().1).sum();
    text(bg3, tiles, GRID_COL, GRID_ROW - 2, &format!("BAG  {used:02}/{}", BAG_W * BAG_H));

    for y in 0..BAG_H as usize {
        for x in 0..BAG_W as usize {
            for q in 0..4 {
                bg3.put(GRID_COL + x * 2 + q % 2, GRID_ROW + y * 2 + q / 2, tiles.bag_cell[q], HUD_MENU);
            }
        }
    }

    let icons = &sprites.bag;
    // The held item floats over the grid, lifted a little.
    let held = menu.held_placement();
    if let Some(h) = held {
        let (px, py) = cell_px(h.x, h.y);
        let (gfx, pal) = icons.icon(h.kind, h.turned);
        list.sky(ObjEntry::new(px - 2, py - 3, gfx, pal, 3));
    }
    for p in bag.items() {
        let (px, py) = cell_px(p.x, p.y);
        let (gfx, pal) = icons.icon(p.kind, p.turned);
        list.sky(ObjEntry::new(px, py, gfx, pal, 3));
    }

    // Cursor brackets around the item (or cell) in question; blinking when
    // a held item can't go down here.
    let (cx, cy) = menu.cursor;
    let (bx, by, bw, bh, blocked) = match held {
        Some(h) => {
            let (w, ht) = h.size();
            (h.x, h.y, w, ht, bag.overlapping(&h).len() > 1)
        }
        None => match bag.at(cx, cy).map(|i| bag.items()[i]) {
            Some(p) => (p.x, p.y, p.size().0, p.size().1, false),
            None => (cx, cy, 1, 1, false),
        },
    };
    if !blocked || clock / 6 % 2 == 0 {
        let (x0, y0) = cell_px(bx, by);
        let (x1, y1) = (x0 + bw * CELL - 8, y0 + bh * CELL - 8);
        for (x, y, fh, fv) in [(x0 - 1, y0 - 1, false, false), (x1 + 1, y0 - 1, true, false), (x0 - 1, y1 + 1, false, true), (x1 + 1, y1 + 1, true, true)] {
            let mut e = ObjEntry::new(x, y, icons.corner, OBJ_FX, 3).flipped(fh);
            e.flip_v = fv;
            list.sky(e);
        }
    }

    // What's under the cursor (or in hand), and what can be done with it.
    let focus = held.map(|h| h.kind).or_else(|| bag.at(cx, cy).map(|i| bag.items()[i].kind));
    if let Some(kind) = focus {
        let (w, h) = footprint(kind, false);
        text(bg3, tiles, INFO_COL, GRID_ROW, kind.name());
        text(bg3, tiles, INFO_COL, GRID_ROW + 1, &format!("SIZE {w}X{h}"));
    }
    if let Mode::Commands { item, choice } = menu.mode {
        let kind = bag.items()[item].kind;
        for (i, c) in commands_for(kind).iter().enumerate() {
            let row = GRID_ROW + 3 + i;
            if i == choice {
                text(bg3, tiles, INFO_COL, row, ">");
            }
            text(bg3, tiles, INFO_COL + 1, row, c.label(kind));
        }
    }
    if let Some(n) = menu.notice {
        text(bg3, tiles, GRID_COL, GRID_ROW + BAG_H as usize * 2 + 1, n);
    }
    let help = match menu.mode {
        Mode::Browse => "A CHOOSE   B CLOSE",
        Mode::Commands { .. } => "A OK   B BACK",
        Mode::Holding(_) => "A PUT  Y TURN  B CANCEL",
    };
    text(bg3, tiles, GRID_COL, LAST_ROW - 1, help);
}

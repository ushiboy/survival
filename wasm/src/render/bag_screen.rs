//! Draws the bag screen: a full-screen subscreen below the HUD. The world
//! layers are switched off; BG3 draws the slots and text, and sprites the
//! item icons and cursor (BG3's priority bit is cleared so they show in
//! front of it).

use crate::assets::palettes::{HUD_DIM, HUD_MENU, HUD_TEXT, OBJ_FX};
use crate::assets::{HudTiles, SpriteIds};
use crate::game::state::Inventory;
use crate::game::bag::{BAG_H, BAG_W, Bag, Placed, footprint};
use crate::crafting::RECIPES;
use crate::game::menu::{BagMenu, Mode, Page, commands_for};
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
/// The page tabs, and the columns between them.
const TAB_ROW: usize = 5;
const TAB_STRIDE: usize = 8;

fn text(bg3: &mut Bg3Layer, tiles: &HudTiles, col: usize, row: usize, s: &str) {
    text_in(bg3, tiles, col, row, s, HUD_TEXT);
}

fn text_in(bg3: &mut Bg3Layer, tiles: &HudTiles, col: usize, row: usize, s: &str, palette: u8) {
    for (i, c) in s.chars().enumerate() {
        bg3.put(col + i, row, tiles.glyph(c), palette);
    }
}

/// Clears the screen's area of BG3 (when the bag closes).
pub fn clear(bg3: &mut Bg3Layer) {
    bg3.clear_rect(0, HUD_ROWS, HUD_COLS, LAST_ROW + 1 - HUD_ROWS);
}

fn cell_px(x: i32, y: i32) -> (i32, i32) {
    ((GRID_COL as i32) * 8 + x * CELL, (GRID_ROW as i32) * 8 + y * CELL)
}

pub fn draw(bg3: &mut Bg3Layer, list: &mut DrawList, tiles: &HudTiles, sprites: &SpriteIds, inventory: &Inventory, menu: &BagMenu, clock: u32) {
    let bag = &inventory.bag;
    clear(bg3);
    // Tabs: the page shown is lit, the others greyed; `>` marks the tab
    // under the cursor while choosing one.
    for (i, page) in Page::ALL.into_iter().enumerate() {
        let col = GRID_COL + i * TAB_STRIDE;
        if menu.mode == Mode::Tabs(page) {
            text(bg3, tiles, col, TAB_ROW, ">");
        }
        text_in(bg3, tiles, col + 1, TAB_ROW, page.label(), if page == menu.page() { HUD_TEXT } else { HUD_DIM });
    }
    let used: i32 = bag.items().iter().map(|p| p.size().0 * p.size().1).sum();
    let room = format!("{used:02}/{}", BAG_W * BAG_H);
    text(bg3, tiles, GRID_COL + BAG_W as usize * 2 - room.len(), GRID_ROW + BAG_H as usize * 2, &room);

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
    // Counts go in first so they're drawn over the icons.
    if let Some(h) = held {
        let (px, py) = cell_px(h.x, h.y);
        draw_count(list, sprites, &h, (px - 2, py - 3));
        let (gfx, pal) = icons.icon(h.kind, h.turned);
        list.sky(ObjEntry::new(px - 2, py - 3, gfx, pal, 3));
    }
    for p in bag.items() {
        draw_count(list, sprites, p, cell_px(p.x, p.y));
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
    let in_grid = matches!(menu.mode, Mode::Browse | Mode::Commands { .. } | Mode::Holding(_));
    if in_grid && (!blocked || clock / 6 % 2 == 0) {
        let (x0, y0) = cell_px(bx, by);
        let (x1, y1) = (x0 + bw * CELL - 8, y0 + bh * CELL - 8);
        for (x, y, fh, fv) in [(x0 - 1, y0 - 1, false, false), (x1 + 1, y0 - 1, true, false), (x0 - 1, y1 + 1, false, true), (x1 + 1, y1 + 1, true, true)] {
            let mut e = ObjEntry::new(x, y, icons.corner, OBJ_FX, 3).flipped(fh);
            e.flip_v = fv;
            list.sky(e);
        }
    }

    // What's under the cursor (or in hand), and what can be done with it.
    let focus = held.map(|h| h.kind).or_else(|| bag.at(cx, cy).map(|i| bag.items()[i].kind)).filter(|_| in_grid);
    if let Some(kind) = focus {
        let (w, h) = footprint(kind, false);
        text(bg3, tiles, INFO_COL, GRID_ROW, kind.name());
        text(bg3, tiles, INFO_COL, GRID_ROW + 1, &format!("SIZE {w}X{h}"));
        if inventory.tool == Some(kind) {
            text(bg3, tiles, INFO_COL, GRID_ROW + 2, "IN HAND");
        }
        let count = held.map(|h| h.count).or_else(|| bag.at(cx, cy).map(|i| bag.items()[i].count));
        if let Some(n) = count.filter(|_| kind.max_stack() > 1) {
            text(bg3, tiles, INFO_COL, GRID_ROW + 2, &format!("{n} OF {}", kind.max_stack()));
        }
    }
    if let Mode::Commands { item, choice } = menu.mode {
        let kind = bag.items()[item].kind;
        for (i, c) in commands_for(kind, inventory.tool == Some(kind)).iter().enumerate() {
            let row = GRID_ROW + 4 + i;
            if i == choice {
                text(bg3, tiles, INFO_COL, row, ">");
            }
            text(bg3, tiles, INFO_COL + 1, row, c.label(kind));
        }
    }
    match menu.mode {
        Mode::Crafting { choice } => draw_crafting(bg3, tiles, list, sprites, bag, choice, true),
        Mode::Tabs(Page::Craft) => draw_crafting(bg3, tiles, list, sprites, bag, 0, false),
        _ => {}
    }
    if menu.page() == Page::Craft && menu.near_fire {
        text(bg3, tiles, INFO_COL, GRID_ROW + BAG_H as usize * 2, "BY THE FIRE");
    }
    if let Some(n) = menu.notice {
        text(bg3, tiles, GRID_COL, GRID_ROW + BAG_H as usize * 2 + 2, n);
    }
    let help = match menu.mode {
        Mode::Tabs(_) => "A OPEN   B CLOSE",
        Mode::Browse => "A CHOOSE   B CLOSE",
        Mode::Crafting { .. } => "A MAKE   B BACK",
        Mode::Commands { .. } => "A OK   B BACK",
        Mode::Holding(_) => "A PUT  Y TURN  B CANCEL",
    };
    text(bg3, tiles, GRID_COL, LAST_ROW - 1, help);
}

/// The number in a stack, over the bottom right of its icon (whose top
/// left is at `at`). Only things that stack are counted.
fn draw_count(list: &mut DrawList, sprites: &SpriteIds, p: &Placed, at: (i32, i32)) {
    if p.kind.max_stack() <= 1 {
        return;
    }
    let (w, h) = p.size();
    let digits = p.count.to_string();
    // Glyphs are 7px wide with their outline; overlap them a pixel.
    let mut x = at.0 + w * CELL - 7 - (digits.len() as i32 - 1) * 6;
    let y = at.1 + h * CELL - 8;
    for d in digits.chars().filter_map(|c| c.to_digit(10)) {
        list.sky(ObjEntry::new(x, y, sprites.bag.digits[d as usize], OBJ_FX, 3));
        x += 6;
    }
}

/// The crafting list beside the grid: recipes with the chosen one's
/// materials (have/need) and a picture of what it makes. `active` is false
/// while only previewing it from the tab row.
fn draw_crafting(bg3: &mut Bg3Layer, tiles: &HudTiles, list: &mut DrawList, sprites: &SpriteIds, bag: &Bag, choice: usize, active: bool) {
    for (i, r) in RECIPES.iter().enumerate() {
        let row = GRID_ROW + i;
        if active && i == choice {
            text(bg3, tiles, INFO_COL, row, ">");
        }
        text(bg3, tiles, INFO_COL + 1, row, r.makes.name());
    }
    let recipe = &RECIPES[choice];
    let top = GRID_ROW + 1 + RECIPES.len();
    text(bg3, tiles, INFO_COL, top, "NEEDS");
    for (i, &(kind, n)) in recipe.needs.iter().enumerate() {
        let have = bag.count(kind).min(9);
        text(bg3, tiles, INFO_COL, top + 1 + i, &format!("{} {have}/{n}", kind.name()));
    }
    if recipe.at_fire {
        text(bg3, tiles, INFO_COL, top + 1 + recipe.needs.len(), "AT A FIRE");
    }
    // The finished thing, at the right below the materials.
    let (w, _) = footprint(recipe.makes, false);
    let (gfx, pal) = sprites.bag.icon(recipe.makes, false);
    let y = (top + 2 + recipe.needs.len() + recipe.at_fire as usize) * 8;
    list.sky(ObjEntry::new(31 * 8 - w * CELL, y as i32, gfx, pal, 3));
}

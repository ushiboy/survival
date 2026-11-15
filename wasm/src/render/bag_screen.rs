//! Draws the bag screen: a full-screen subscreen below the HUD. The world
//! layers are switched off; BG3 draws the slots and text, and sprites the
//! item icons and cursor (BG3's priority bit is cleared so they show in
//! front of it).

use crate::assets::palettes::{HUD_DIM, HUD_FOOD, HUD_MENU, HUD_TEXT, OBJ_FX};
use crate::assets::{HudTiles, SpriteIds};
use crate::crafting::RECIPES;
use crate::game::bag::{BAG_W, Bag, Placed, footprint};
use crate::game::menu::{BagMenu, GearRow, Mode, Page, RECIPES_PER_PAGE, commands_for, gear_choices, gear_rows};
use crate::game::state::Inventory;
use crate::items::{ItemKind, Slot};
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
const TAB_STRIDE_FIVE: usize = 6;
/// The cart page: the cart's grid starts this column, right of the bag's.
const CART_COL: usize = GRID_COL + BAG_W as usize * 2 + 1;
/// The crafting page: the picture of the chosen recipe's result in the
/// list; in a recipe, its picture's right edge (tiles), the description's
/// width, the material count columns, and the MAKE / BACK rows.
const PREVIEW_COL: usize = 22;
const ICON_RIGHT: i32 = 30;
const DESC_WIDTH: usize = 24;
const NEED_COL: usize = 20;
const HAVE_COL: usize = 26;
const CHOICE_ROW: usize = 21;
const NOTICE_ROW: usize = 24;
/// The mark of the lathe (a cog), by recipes turned on it as `^` (a
/// flame) is by those made at a fire.
const LATHE_MARK: &str = "*";

fn text(bg3: &mut Bg3Layer, tiles: &HudTiles, col: usize, row: usize, s: &str) {
    text_in(bg3, tiles, col, row, s, HUD_TEXT);
}

/// Writes `s` from `col` to the screen's right edge at most, cutting a
/// name too long to fit and ending it with an ellipsis.
fn text_fit(bg3: &mut Bg3Layer, tiles: &HudTiles, col: usize, row: usize, s: &str) {
    text(bg3, tiles, col, row, &fit(s, HUD_COLS.saturating_sub(col)));
}

/// `s` cut to `width` characters, ending in `…` if anything was cut (and
/// no space is left before it).
fn fit(s: &str, width: usize) -> String {
    if s.chars().count() <= width {
        return s.to_string();
    }
    let kept: String = s.chars().take(width.saturating_sub(1)).collect();
    format!("{}…", kept.trim_end())
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
    let pages = menu.pages();
    // Closer together when the cart's page makes five.
    let stride = if pages.len() > 4 { TAB_STRIDE_FIVE } else { TAB_STRIDE };
    for (i, page) in pages.into_iter().enumerate() {
        let col = GRID_COL + i * stride;
        if menu.mode == Mode::Tabs(page) {
            text(bg3, tiles, col, TAB_ROW, ">");
        }
        text_in(bg3, tiles, col + 1, TAB_ROW, page.label(), if page == menu.page() { HUD_TEXT } else { HUD_DIM });
    }
    if menu.page() == Page::Craft {
        draw_craft_page(bg3, tiles, list, sprites, inventory, menu);
        return;
    }
    if menu.page() == Page::Save {
        draw_save_page(bg3, tiles, menu);
        return;
    }
    if menu.page() == Page::Cart {
        draw_cart_page(bg3, tiles, list, sprites, inventory, menu);
        return;
    }
    let used: i32 = bag.items().iter().map(|p| p.size().0 * p.size().1).sum();
    let room = format!("{used:02}/{}", BAG_W * bag.rows());
    text(bg3, tiles, GRID_COL + BAG_W as usize * 2 - room.len(), GRID_ROW + bag.rows() as usize * 2, &room);

    for y in 0..bag.rows() as usize {
        for x in 0..BAG_W as usize {
            for q in 0..4 {
                bg3.put(GRID_COL + x * 2 + q % 2, GRID_ROW + y * 2 + q / 2, tiles.bag_cell[q], HUD_MENU);
            }
        }
    }

    let icons = &sprites.bag;
    // The held item floats over the grid, lifted a little.
    let held = menu.held_placement();
    // Counts and the in-hand mark go in first so they're drawn over the
    // icons.
    let in_hand = |p: &Placed| inventory.gear.hand == Some(p.kind);
    if let Some(h) = held {
        let (px, py) = cell_px(h.x, h.y);
        draw_count(list, sprites, &h, (px - 2, py - 3));
        if in_hand(&h) {
            list.sky(ObjEntry::new(px - 2, py - 3, icons.equipped, OBJ_FX, 3));
        }
        let (gfx, pal) = icons.icon(h.kind, h.turned);
        list.sky(ObjEntry::new(px - 2, py - 3, gfx, pal, 3));
    }
    for p in bag.items() {
        draw_count(list, sprites, p, cell_px(p.x, p.y));
        if in_hand(p) {
            let (px, py) = cell_px(p.x, p.y);
            list.sky(ObjEntry::new(px, py, icons.equipped, OBJ_FX, 3));
        }
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
    if in_grid && (!blocked || (clock / 6).is_multiple_of(2)) {
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
        if inventory.gear.hand == Some(kind) {
            text(bg3, tiles, INFO_COL, GRID_ROW + 2, "IN HAND");
        }
        let count = held.map(|h| h.count).or_else(|| bag.at(cx, cy).map(|i| bag.items()[i].count));
        if let Some(n) = count.filter(|_| kind.max_stack() > 1) {
            text(bg3, tiles, INFO_COL, GRID_ROW + 2, &format!("{n} OF {}", kind.max_stack()));
        }
    }
    if let Mode::Commands { item, choice } = menu.mode {
        let kind = bag.items()[item].kind;
        for (i, c) in commands_for(kind, &inventory.gear).iter().enumerate() {
            let row = GRID_ROW + 4 + i;
            if i == choice {
                text(bg3, tiles, INFO_COL, row, ">");
            }
            text(bg3, tiles, INFO_COL + 1, row, c.label(kind));
        }
    }
    if menu.page() == Page::Gear {
        draw_gear(bg3, tiles, inventory, menu);
    }
    if let Some(n) = menu.notice {
        text(bg3, tiles, GRID_COL, GRID_ROW + bag.rows() as usize * 2 + 2, n);
    }
    draw_help(bg3, tiles, menu);
}

/// The save page: one choice, SAVE, and word of the last save.
fn draw_save_page(bg3: &mut Bg3Layer, tiles: &HudTiles, menu: &BagMenu) {
    if menu.mode == Mode::Saving {
        text(bg3, tiles, GRID_COL, GRID_ROW, ">");
    }
    text(bg3, tiles, GRID_COL + 1, GRID_ROW, "SAVE");
    text_in(bg3, tiles, GRID_COL + 1, GRID_ROW + 2, "KEEP HOW THINGS STAND NOW.", HUD_DIM);
    text_in(bg3, tiles, GRID_COL + 1, GRID_ROW + 3, "IT IS ALSO SAVED EACH TIME", HUD_DIM);
    text_in(bg3, tiles, GRID_COL + 1, GRID_ROW + 4, "YOU WAKE.", HUD_DIM);
    if let Some(n) = menu.notice {
        text(bg3, tiles, GRID_COL, NOTICE_ROW, n);
    }
    draw_help(bg3, tiles, menu);
}

fn draw_help(bg3: &mut Bg3Layer, tiles: &HudTiles, menu: &BagMenu) {
    let help = match menu.mode {
        Mode::Tabs(_) => "A OPEN   B CLOSE",
        Mode::Browse => "A CHOOSE   B CLOSE",
        Mode::Crafting { .. } => "A CHOOSE   B BACK",
        Mode::Recipe { .. } | Mode::Commands { .. } | Mode::GearPick { .. } => "A OK   B BACK",
        Mode::Holding(_) => "A PUT  Y TURN  B CANCEL",
        Mode::Gear { .. } => "A CHANGE   B BACK",
        Mode::Saving => "A SAVE   B CLOSE",
        Mode::Cart { .. } => "A MOVE ACROSS   B BACK",
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

/// The cart page: the bag's grid on the left, the cart's on the right,
/// each with the room it has used; the cursor on a cell of one of them,
/// and what's there named below.
fn draw_cart_page(bg3: &mut Bg3Layer, tiles: &HudTiles, list: &mut DrawList, sprites: &SpriteIds, inventory: &Inventory, menu: &BagMenu) {
    let Some(cart) = inventory.cart.as_ref() else { return };
    let bag = &inventory.bag;
    let hand = inventory.gear.hand;
    draw_grid(bg3, tiles, list, sprites, bag, GRID_COL, hand);
    draw_grid(bg3, tiles, list, sprites, cart, CART_COL, None);
    let in_cart = matches!(menu.mode, Mode::Cart { in_cart: true });
    let (on, col) = if in_cart { (cart, CART_COL) } else { (bag, GRID_COL) };
    let rows = bag.rows().max(cart.rows()) as usize;
    if let Mode::Cart { .. } = menu.mode {
        let (cx, cy) = menu.cursor;
        let item = on.at(cx, cy).map(|i| on.items()[i]);
        let (bx, by, bw, bh) = item.map_or((cx, cy, 1, 1), |p| (p.x, p.y, p.size().0, p.size().1));
        let x0 = col as i32 * 8 + bx * CELL;
        let y0 = GRID_ROW as i32 * 8 + by * CELL;
        let (x1, y1) = (x0 + bw * CELL - 8, y0 + bh * CELL - 8);
        for (x, y, fh, fv) in [(x0 - 1, y0 - 1, false, false), (x1 + 1, y0 - 1, true, false), (x0 - 1, y1 + 1, false, true), (x1 + 1, y1 + 1, true, true)] {
            let mut e = ObjEntry::new(x, y, sprites.bag.corner, OBJ_FX, 3).flipped(fh);
            e.flip_v = fv;
            list.sky(e);
        }
        if let Some(p) = item {
            let row = GRID_ROW + rows * 2 + 2;
            text(bg3, tiles, GRID_COL, row, p.kind.name());
            if p.kind.max_stack() > 1 {
                text(bg3, tiles, GRID_COL + p.kind.name().len() + 1, row, &format!("{} OF {}", p.count, p.kind.max_stack()));
            }
        }
    }
    text_in(bg3, tiles, CART_COL, GRID_ROW - 1, "CART", HUD_DIM);
    if let Some(n) = menu.notice {
        text(bg3, tiles, GRID_COL, NOTICE_ROW, n);
    }
    draw_help(bg3, tiles, menu);
}

/// A grid (the bag's, or the cart's) from column `col`: its cells, the
/// icons in it with their counts, a mark on the tool in hand, and the
/// room used under it.
fn draw_grid(bg3: &mut Bg3Layer, tiles: &HudTiles, list: &mut DrawList, sprites: &SpriteIds, grid: &Bag, col: usize, hand: Option<ItemKind>) {
    let (w, rows) = (grid.width() as usize, grid.rows() as usize);
    for y in 0..rows {
        for x in 0..w {
            for q in 0..4 {
                bg3.put(col + x * 2 + q % 2, GRID_ROW + y * 2 + q / 2, tiles.bag_cell[q], HUD_MENU);
            }
        }
    }
    let used: i32 = grid.items().iter().map(|p| p.size().0 * p.size().1).sum();
    let room = format!("{used:02}/{}", w * rows);
    text(bg3, tiles, col + w * 2 - room.len(), GRID_ROW + rows * 2, &room);
    let at = |p: &Placed| (col as i32 * 8 + p.x * CELL, GRID_ROW as i32 * 8 + p.y * CELL);
    for p in grid.items() {
        draw_count(list, sprites, p, at(p));
        if hand == Some(p.kind) {
            let (px, py) = at(p);
            list.sky(ObjEntry::new(px, py, sprites.bag.equipped, OBJ_FX, 3));
        }
    }
    for p in grid.items() {
        let (px, py) = at(p);
        let (gfx, pal) = sprites.bag.icon(p.kind, p.turned);
        list.sky(ObjEntry::new(px, py, gfx, pal, 3));
    }
}

/// The crafting page: the recipe list, or one recipe opened from it.
fn draw_craft_page(bg3: &mut Bg3Layer, tiles: &HudTiles, list: &mut DrawList, sprites: &SpriteIds, inventory: &Inventory, menu: &BagMenu) {
    // Where the castaway is, if it's somewhere things are made (the lathe
    // first: a fire can be lit by it, but the lathe is what's rare).
    if menu.near_lathe {
        text_in(bg3, tiles, INFO_COL + 2, TAB_ROW + 1, LATHE_MARK, HUD_FOOD);
        text(bg3, tiles, INFO_COL + 3, TAB_ROW + 1, "BY THE LATHE");
    } else if menu.near_fire {
        text_in(bg3, tiles, INFO_COL + 3, TAB_ROW + 1, "^", HUD_FOOD);
        text(bg3, tiles, INFO_COL + 4, TAB_ROW + 1, "BY THE FIRE");
    }
    match menu.mode {
        Mode::Recipe { recipe, choice } => draw_recipe(bg3, tiles, list, sprites, inventory, menu, recipe, choice),
        Mode::Crafting { choice } => draw_recipe_list(bg3, tiles, list, sprites, inventory, menu, Some(choice)),
        _ => draw_recipe_list(bg3, tiles, list, sprites, inventory, menu, None),
    }
    if let Some(n) = menu.notice {
        text(bg3, tiles, GRID_COL, NOTICE_ROW, n);
    }
    draw_help(bg3, tiles, menu);
}

/// A page of the recipes thought of so far: those that can be made now
/// lit, the rest greyed, a flame by those made at a fire, a "!" by those
/// not yet looked at; and a picture of the chosen one's result. `choice`
/// (a place in the list) is None while only previewing from the tab row.
/// Recipes not yet thought of aren't shown at all, not even as a count.
#[allow(clippy::too_many_arguments)]
fn draw_recipe_list(
    bg3: &mut Bg3Layer,
    tiles: &HudTiles,
    list: &mut DrawList,
    sprites: &SpriteIds,
    inventory: &Inventory,
    menu: &BagMenu,
    choice: Option<usize>,
) {
    let near_fire = menu.near_fire;
    let book = &inventory.recipes;
    let listed = book.listed();
    let at = choice.unwrap_or(0);
    let page = at / RECIPES_PER_PAGE;
    let pages = listed.len().div_ceil(RECIPES_PER_PAGE);
    let first = page * RECIPES_PER_PAGE;
    for (place, &i) in listed.iter().enumerate().skip(first).take(RECIPES_PER_PAGE) {
        let r = &RECIPES[i];
        let row = GRID_ROW + (place - first) * 2;
        if choice == Some(place) {
            text(bg3, tiles, GRID_COL, row, ">");
        }
        // Bright if it can be made now, whichever way.
        let ready = book.ways(i).iter().any(|&w| inventory.can_make(&RECIPES[w]) && menu.in_place_for(&RECIPES[w]));
        let name = r.makes.name();
        text_in(bg3, tiles, GRID_COL + 1, row, name, if ready { HUD_TEXT } else { HUD_DIM });
        if r.at_fire {
            text_in(bg3, tiles, GRID_COL + 2 + name.len(), row, "^", if near_fire { HUD_FOOD } else { HUD_DIM });
        }
        if r.at_lathe {
            text_in(bg3, tiles, GRID_COL + 2 + name.len(), row, LATHE_MARK, if menu.near_lathe { HUD_FOOD } else { HUD_DIM });
        }
        if book.item_fresh(i) {
            text_in(bg3, tiles, GRID_COL + 4 + name.len(), row, "!", HUD_FOOD);
        }
    }
    if pages > 1 {
        let row = GRID_ROW + RECIPES_PER_PAGE * 2;
        let label = format!("{}/{}", page + 1, pages);
        text_in(bg3, tiles, GRID_COL + 1, row, "<", if page > 0 { HUD_TEXT } else { HUD_DIM });
        text(bg3, tiles, GRID_COL + 3, row, &label);
        text_in(bg3, tiles, GRID_COL + 4 + label.len(), row, ">", if page + 1 < pages { HUD_TEXT } else { HUD_DIM });
    }
    if let Some(&i) = listed.get(at).filter(|_| choice.is_some()) {
        let makes = RECIPES[i].makes;
        let (w, h) = footprint(makes, false);
        let (gfx, pal) = sprites.bag.icon(makes, false);
        list.sky(ObjEntry::new(PREVIEW_COL as i32 * 8, GRID_ROW as i32 * 8, gfx, pal, 3));
        // A torch never takes room in the bag.
        if makes.goes_in_bag() {
            text(bg3, tiles, PREVIEW_COL, GRID_ROW + h as usize * 2 + 1, &format!("SIZE {w}X{h}"));
        }
    }
}

/// One recipe opened: what it makes and what that's for, the materials
/// with how many are needed and carried (short ones greyed), the tools it
/// takes, whether it needs a fire, and MAKE / BACK. An item made more than
/// one way shows which way this is (`< 1/2 >` by NEEDS), with a `!` on
/// the side of a way not yet looked at.
#[allow(clippy::too_many_arguments)]
fn draw_recipe(
    bg3: &mut Bg3Layer,
    tiles: &HudTiles,
    list: &mut DrawList,
    sprites: &SpriteIds,
    inventory: &Inventory,
    menu: &BagMenu,
    index: usize,
    choice: usize,
) {
    let (near_fire, near_lathe) = (menu.near_fire, menu.near_lathe);
    let (bag, book) = (&inventory.bag, &inventory.recipes);
    let recipe = &RECIPES[index];
    let makes = recipe.makes;
    let (w, h) = footprint(makes, false);
    text(bg3, tiles, GRID_COL, GRID_ROW, makes.name());
    if makes.goes_in_bag() {
        text(bg3, tiles, GRID_COL, GRID_ROW + 1, &format!("SIZE {w}X{h}"));
    }
    let (gfx, pal) = sprites.bag.icon(makes, false);
    list.sky(ObjEntry::new(ICON_RIGHT * 8 - w * CELL, GRID_ROW as i32 * 8, gfx, pal, 3));
    for (i, line) in wrap(makes.description(), DESC_WIDTH).iter().enumerate() {
        text(bg3, tiles, GRID_COL, GRID_ROW + 3 + i, line);
    }

    let top = GRID_ROW + 7;
    text(bg3, tiles, GRID_COL, top, "NEEDS");
    let ways = book.ways(index);
    if ways.len() > 1 {
        let at = ways.iter().position(|&w| w == index).unwrap_or(0);
        let (left, right) = (&ways[..at], &ways[at + 1..]);
        let col = GRID_COL + 7;
        let label = format!("{}/{}", at + 1, ways.len());
        text_in(bg3, tiles, col, top, "<", if left.is_empty() { HUD_DIM } else { HUD_TEXT });
        text(bg3, tiles, col + 2, top, &label);
        text_in(bg3, tiles, col + 3 + label.len(), top, ">", if right.is_empty() { HUD_DIM } else { HUD_TEXT });
        if left.iter().any(|&w| book.is_fresh(w)) {
            text_in(bg3, tiles, col - 1, top, "!", HUD_FOOD);
        }
        if right.iter().any(|&w| book.is_fresh(w)) {
            text_in(bg3, tiles, col + 4 + label.len(), top, "!", HUD_FOOD);
        }
    }
    text_in(bg3, tiles, NEED_COL - 2, top, "NEED", HUD_DIM);
    text_in(bg3, tiles, HAVE_COL - 2, top, "HAVE", HUD_DIM);
    for (i, &(kind, n)) in recipe.needs.iter().enumerate() {
        let row = top + 1 + i;
        let have = bag.count(kind);
        let palette = if have >= n { HUD_TEXT } else { HUD_DIM };
        text_in(bg3, tiles, GRID_COL + 1, row, kind.name(), palette);
        text_in(bg3, tiles, NEED_COL, row, &format!("{n:>2}"), palette);
        text_in(bg3, tiles, HAVE_COL, row, &format!("{:>2}", have.min(99)), palette);
    }
    // Tools: carried, not used up.
    for (i, &kind) in recipe.tools.iter().enumerate() {
        let row = top + 1 + recipe.needs.len() + i;
        let have = inventory.carries(kind);
        let palette = if have { HUD_TEXT } else { HUD_DIM };
        text_in(bg3, tiles, GRID_COL + 1, row, kind.name(), palette);
        text_in(bg3, tiles, NEED_COL - 2, row, "TOOL", palette);
        text_in(bg3, tiles, HAVE_COL - 1, row, if have { "YES" } else { " NO" }, palette);
    }
    if recipe.at_fire {
        let row = top + 1 + recipe.needs.len() + recipe.tools.len();
        let palette = if near_fire { HUD_TEXT } else { HUD_DIM };
        text_in(bg3, tiles, GRID_COL + 1, row, "^", if near_fire { HUD_FOOD } else { HUD_DIM });
        text_in(bg3, tiles, GRID_COL + 3, row, "A FIRE", palette);
        text_in(bg3, tiles, HAVE_COL - 1, row, if near_fire { "YES" } else { " NO" }, palette);
    }
    if recipe.at_lathe {
        let row = top + 1 + recipe.needs.len() + recipe.tools.len();
        let palette = if near_lathe { HUD_TEXT } else { HUD_DIM };
        text_in(bg3, tiles, GRID_COL + 1, row, LATHE_MARK, if near_lathe { HUD_FOOD } else { HUD_DIM });
        text_in(bg3, tiles, GRID_COL + 3, row, "A LATHE", palette);
        text_in(bg3, tiles, HAVE_COL - 1, row, if near_lathe { "YES" } else { " NO" }, palette);
    }

    for (i, label) in ["MAKE", "BACK"].into_iter().enumerate() {
        let row = CHOICE_ROW + i;
        if i == choice {
            text(bg3, tiles, GRID_COL, row, ">");
        }
        text(bg3, tiles, GRID_COL + 1, row, label);
    }
}

/// Breaks text into lines of at most `width` characters at spaces.
fn wrap(s: &str, width: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for word in s.split(' ') {
        match lines.last_mut() {
            Some(line) if line.len() + 1 + word.len() <= width => {
                line.push(' ');
                line.push_str(word);
            }
            _ => lines.push(word.to_string()),
        }
    }
    lines
}

/// The gear page beside the grid: each slot with what's in it (and the
/// room a pack adds), the tools on the belt's hooks under the waist, and
/// the choices for the line being changed.
fn draw_gear(bg3: &mut Bg3Layer, tiles: &HudTiles, inventory: &Inventory, menu: &BagMenu) {
    let (focus, picking) = match menu.mode {
        Mode::Gear { slot } => (Some(slot), None),
        Mode::GearPick { slot, choice } => (Some(slot), Some(choice)),
        _ => (None, None),
    };
    let gear = &inventory.gear;
    let rows = gear_rows(gear);
    let mut row = GRID_ROW;
    for (i, &line) in rows.iter().enumerate() {
        match line {
            GearRow::Slot(slot) => {
                if focus == Some(i) {
                    text(bg3, tiles, INFO_COL, row, ">");
                }
                text(bg3, tiles, INFO_COL + 1, row, slot.label());
                let worn = gear.worn(slot);
                text_fit(bg3, tiles, INFO_COL + 2, row + 1, worn.map_or("-", |k| k.name()));
                row += 2;
                if let Some(k) = worn.filter(|k| k.extra_rows() > 0) {
                    text(bg3, tiles, INFO_COL + 2, row, &format!("+{} ROOM", k.extra_rows() * BAG_W));
                    row += 1;
                }
                // A gap before the next slot, but not before the hooks.
                if slot != Slot::Waist || gear.hooks() == 0 {
                    row += 1;
                }
            }
            GearRow::Hook(h) => {
                // A step in from the belt's name: they hang from it.
                if focus == Some(i) {
                    text(bg3, tiles, INFO_COL + 2, row, ">");
                }
                text_fit(bg3, tiles, INFO_COL + 3, row, gear.belt[h].map_or("-", |k| k.name()));
                row += 1;
            }
        }
    }
    if let (Some(slot), Some(choice)) = (focus, picking) {
        // As many as fit above the help line, scrolled to keep the choice
        // in view.
        let top = row + 1;
        let room = (LAST_ROW - 2).saturating_sub(top).max(1);
        let first = (choice + 1).saturating_sub(room);
        for (i, c) in gear_choices(inventory, rows[slot]).iter().enumerate().skip(first).take(room) {
            let at = top + i - first;
            if i == choice {
                text(bg3, tiles, INFO_COL, at, ">");
            }
            text_fit(bg3, tiles, INFO_COL + 1, at, c.label());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_that_fits_is_left_whole() {
        assert_eq!(fit("STONE AXE", 12), "STONE AXE");
        assert_eq!(fit("BAMBOO SPADE", 12), "BAMBOO SPADE");
    }

    #[test]
    fn a_name_too_long_is_cut_to_the_width_with_an_ellipsis() {
        let cut = fit("DIGGING STICK", 12);
        assert_eq!(cut, "DIGGING STI…");
        assert_eq!(cut.chars().count(), 12);
    }

    #[test]
    fn no_space_is_left_before_the_ellipsis() {
        assert_eq!(fit("WOOD SPEAR", 6), "WOOD…");
    }
}

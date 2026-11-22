//! Draws the bag screen: a full-screen subscreen below the HUD. The world
//! layers are switched off; BG3 draws the slots and text, and sprites the
//! item icons and cursor (BG3's priority bit is cleared so they show in
//! front of it).

use crate::assets::palettes::{HUD_DIM, HUD_FOOD, HUD_HEART, HUD_MENU, HUD_TEXT, OBJ_FX};
use crate::assets::{HudTiles, SpriteIds};
use crate::crafting::{RECIPES, RecipeOrder};
use crate::game::bag::{BAG_W, Bag, Placed, footprint};
use crate::game::menu::{
    BagMenu, GearChoice, Mode, NEEDS_SHOWN, NeedLine, Page, RECIPES_PER_PAGE, Spot, commands_for, gear_choices, hook_at, item_at, lies_on_hook, need_lines,
    on_belt,
};
use crate::game::state::HOOK_CELLS;
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
/// The page tabs and each one's column (its `>`; the label follows), a
/// space apart, leaving room for CRAFT's `!`.
const TAB_ROW: usize = 5;
const TAB_COLS: [usize; 4] = [3, 10, 18, 24];
/// The L and R marks either side of the tabs.
const TAB_L_COL: usize = 1;
const TAB_R_COL: usize = 30;
/// The BUILD screen: the next stage's materials, what's been left at the
/// site of each (IN) between what it takes (NEED) and what's at hand
/// (HAVE), and BUILD below what it takes.
const BUILD_NEED_COL: usize = 17;
const BUILD_IN_COL: usize = 21;
const BUILD_HAVE_COL: usize = 27;
const BUILD_ROW: usize = NOTICE_ROW - 2;
/// The cart page: the cart's grid starts this column, right of the bag's.
const CART_COL: usize = GRID_COL + BAG_W as usize * 2 + 1;
/// The crafting page: the picture of the chosen recipe's result in the
/// list; in a recipe, its picture's right edge (tiles), the row its
/// description starts and the widest it runs (narrower beside a wide
/// picture), the top row of what it needs, and the count columns there.
const PREVIEW_COL: usize = 22;
const ICON_RIGHT: usize = 30;
const DESC_ROW: usize = GRID_ROW + 2;
const DESC_WIDTH: usize = 24;
const NEEDS_ROW: usize = 15;
const NEED_COL: usize = 20;
const HAVE_COL: usize = 26;
/// The ▲ / ▼ beside what a recipe needs when it scrolls.
const SCROLL_COL: usize = 29;
const NOTICE_ROW: usize = 24;
// What a recipe needs ends above the notice.
const _: () = assert!(NEEDS_ROW + NEEDS_SHOWN <= NOTICE_ROW);
/// The gear page, across the whole width: the slots (as many as
/// `PLACES_SHOWN`, more scroll), each's name and what's in it from
/// `GEAR_ITEM_COL`; below them what can go in the slot chosen (names
/// `GEAR_NAME_WIDTH` wide), with where each is kept at `GEAR_FROM_COL`,
/// and the picture of the thing in question to their right, from
/// `GEAR_PIC_COL` to the screen's edge; and at the bottom, from
/// `GEAR_DESC_ROW`, what it is (`GEAR_DESC_LINES` lines of
/// `GEAR_DESC_WIDTH`).
const GEAR_COL: usize = 2;
const PLACES_SHOWN: usize = 7;
const GEAR_ITEM_COL: usize = 12;
const GEAR_NAME_WIDTH: usize = 13;
const GEAR_FROM_COL: usize = 17;
const GEAR_PIC_COL: usize = 23;
const GEAR_PIC_WIDTH: i32 = 64;
const GEAR_DESC_ROW: usize = 20;
const GEAR_DESC_LINES: usize = 4;
const GEAR_DESC_WIDTH: usize = 28;
// The description ends above the notice.
const _: () = assert!(GEAR_DESC_ROW + GEAR_DESC_LINES <= NOTICE_ROW);
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
    // The BUILD screen (opened at the bench beside the mill's ground) is
    // on its own, without the bag's tabs.
    if menu.page() == Page::Build {
        draw_build_page(bg3, tiles, inventory, menu);
        return;
    }
    // Tabs: the page shown is lit, the others greyed; `>` marks the tab
    // under the cursor while choosing one.
    let pages = menu.pages();
    for (page, &col) in pages.into_iter().zip(&TAB_COLS) {
        if menu.mode == Mode::Tabs(page) {
            text(bg3, tiles, col, TAB_ROW, ">");
        }
        text_in(bg3, tiles, col + 1, TAB_ROW, page.label(), if page == menu.page() { HUD_TEXT } else { HUD_DIM });
        if page == Page::Craft && inventory.recipes.any_fresh() {
            text_in(bg3, tiles, col + 1 + page.label().len(), TAB_ROW, "!", HUD_FOOD);
        }
    }
    // L and R either side of the tabs: they switch pages.
    text_in(bg3, tiles, TAB_L_COL, TAB_ROW, "L", HUD_DIM);
    text_in(bg3, tiles, TAB_R_COL, TAB_ROW, "R", HUD_DIM);
    if menu.page() == Page::Craft {
        draw_craft_page(bg3, tiles, list, sprites, inventory, menu);
        return;
    }
    if menu.page() == Page::Cart {
        draw_cart_page(bg3, tiles, list, sprites, inventory, menu);
        return;
    }
    if menu.page() == Page::Gear {
        draw_gear_page(bg3, tiles, list, sprites, inventory, menu);
        return;
    }
    let rows = bag.rows();
    let hooks = inventory.gear.hooks();
    let used: i32 = bag.items().iter().map(|p| p.size().0 * p.size().1).sum();
    let room = format!("{used:02}/{}", BAG_W * rows);
    let under = GRID_ROW + rows as usize * 2;
    text(bg3, tiles, GRID_COL + BAG_W as usize * 2 - room.len(), under, &room);

    for y in 0..rows as usize {
        for x in 0..BAG_W as usize {
            for q in 0..4 {
                bg3.put(GRID_COL + x * 2 + q % 2, GRID_ROW + y * 2 + q / 2, tiles.bag_cell[q], HUD_MENU);
            }
        }
    }
    // The belt's hooks, side by side a row below the grid (a tile apart),
    // while one is worn: the cells of each, with a tool hung there lying
    // along them.
    let hook_gap = |x: usize| x / HOOK_CELLS as usize;
    for x in 0..hooks * HOOK_CELLS as usize {
        for q in 0..4 {
            bg3.put(GRID_COL + x * 2 + hook_gap(x) + q % 2, under + 1 + q / 2, tiles.bag_cell[q], HUD_MENU);
        }
    }
    let px_at = |x: i32, y: i32| {
        if y >= rows { ((GRID_COL + hook_gap(x as usize)) as i32 * 8 + x * CELL, (under as i32 + 1) * 8) } else { cell_px(x, y) }
    };

    let icons = &sprites.bag;
    // The held item floats over the grid, lifted a little (lying along a
    // hook over the belt).
    let held = menu.held_placement().map(|h| if on_belt((h.x, h.y), bag) { Placed { turned: lies_on_hook(h.kind), ..h } } else { h });
    // Counts and the in-hand mark go in first so they're drawn over the
    // icons.
    let in_hand = |kind: ItemKind| inventory.gear.hand == Some(kind);
    if let Some(h) = held {
        let (px, py) = px_at(h.x, h.y);
        draw_count(list, sprites, &h, (px - 2, py - 3));
        if in_hand(h.kind) {
            list.sky(ObjEntry::new(px - 2, py - 3, icons.equipped, OBJ_FX, 3));
        }
        let (gfx, pal) = icons.icon(h.kind, h.turned);
        list.sky(ObjEntry::new(px - 2, py - 3, gfx, pal, 3));
    }
    let hung: Vec<Placed> = (0..hooks)
        .filter_map(|i| inventory.gear.belt[i].map(|kind| Placed { kind, x: i as i32 * HOOK_CELLS, y: rows, turned: lies_on_hook(kind), count: 1 }))
        .collect();
    for p in bag.items().iter().chain(&hung) {
        let (px, py) = px_at(p.x, p.y);
        draw_count(list, sprites, p, (px, py));
        if in_hand(p.kind) {
            list.sky(ObjEntry::new(px, py, icons.equipped, OBJ_FX, 3));
        }
    }
    for p in bag.items().iter().chain(&hung) {
        let (px, py) = px_at(p.x, p.y);
        let (gfx, pal) = icons.icon(p.kind, p.turned);
        list.sky(ObjEntry::new(px, py, gfx, pal, 3));
    }

    // Cursor brackets around the item (or cell, or hook) in question;
    // blinking when a held item can't go down here.
    let (cx, cy) = menu.cursor;
    let hook_frame = |x: i32| (hook_at(x) as i32 * HOOK_CELLS, rows, HOOK_CELLS, 1);
    let (bx, by, bw, bh, blocked) = match held {
        Some(h) if on_belt((h.x, h.y), bag) => {
            let (x, y, w, ht) = hook_frame(h.x);
            (x, y, w, ht, !h.kind.hangs_on_belt() || h.count > 1)
        }
        Some(h) => {
            let (w, ht) = h.size();
            (h.x, h.y, w, ht, bag.overlapping(&h).len() > 1)
        }
        None if on_belt((cx, cy), bag) => {
            let (x, y, w, ht) = hook_frame(cx);
            (x, y, w, ht, false)
        }
        None => match bag.at(cx, cy).map(|i| bag.items()[i]) {
            Some(p) => (p.x, p.y, p.size().0, p.size().1, false),
            None => (cx, cy, 1, 1, false),
        },
    };
    let in_grid = matches!(menu.mode, Mode::Browse | Mode::Commands { .. } | Mode::Holding(_));
    if in_grid && (!blocked || (clock / 6).is_multiple_of(2)) {
        let (x0, y0) = px_at(bx, by);
        let (x1, y1) = (x0 + bw * CELL - 8, y0 + bh * CELL - 8);
        for (x, y, fh, fv) in [(x0 - 1, y0 - 1, false, false), (x1 + 1, y0 - 1, true, false), (x0 - 1, y1 + 1, false, true), (x1 + 1, y1 + 1, true, true)] {
            let mut e = ObjEntry::new(x, y, icons.corner, OBJ_FX, 3).flipped(fh);
            e.flip_v = fv;
            list.sky(e);
        }
    }

    // What's under the cursor (or in hand), and what can be done with it.
    let under_cursor = item_at(inventory, (cx, cy));
    let focus = held.map(|h| h.kind).or(under_cursor.map(|(_, k)| k)).filter(|_| in_grid);
    if let Some(kind) = focus {
        let (w, h) = footprint(kind, false);
        text(bg3, tiles, INFO_COL, GRID_ROW, kind.name());
        text(bg3, tiles, INFO_COL, GRID_ROW + 1, &format!("SIZE {w}X{h}"));
        if in_hand(kind) {
            text(bg3, tiles, INFO_COL, GRID_ROW + 2, "IN HAND");
        }
        let count = held.map(|h| h.count).or_else(|| bag.at(cx, cy).map(|i| bag.items()[i].count).filter(|_| !on_belt((cx, cy), bag)));
        if let Some(n) = count.filter(|_| kind.max_stack() > 1) {
            text(bg3, tiles, INFO_COL, GRID_ROW + 2, &format!("{n} OF {}", kind.max_stack()));
        }
    }
    if let Mode::Commands { item, choice } = menu.mode {
        let kind = match item {
            Spot::Bag(i) => bag.items()[i].kind,
            Spot::Hook(h) => inventory.gear.belt[h].unwrap_or(ItemKind::Stone),
        };
        for (i, c) in commands_for(kind, &inventory.gear).iter().enumerate() {
            let row = GRID_ROW + 4 + i;
            if i == choice {
                text(bg3, tiles, INFO_COL, row, ">");
            }
            text(bg3, tiles, INFO_COL + 1, row, c.label(kind));
        }
    }
    if let Some(n) = menu.notice {
        // Below the belt, if one is worn.
        text(bg3, tiles, GRID_COL, under + if hooks > 0 { 4 } else { 2 }, n);
    }
    draw_help(bg3, tiles, menu);
}

fn draw_help(bg3: &mut Bg3Layer, tiles: &HudTiles, menu: &BagMenu) {
    let help = match menu.mode {
        Mode::Tabs(_) => "A OPEN   B CLOSE",
        Mode::Browse => "A CHOOSE   B CLOSE",
        Mode::Crafting { .. } => "A CHOOSE   B BACK",
        Mode::Sorting { .. } => "A OK   B BACK",
        Mode::Recipe { confirming: true, .. } => "A YES   B NO",
        Mode::Recipe { .. } => "A MAKE   B BACK",
        Mode::Commands { .. } | Mode::GearPick { .. } => "A OK   B BACK",
        Mode::Holding(_) => "A PUT  Y TURN  B CANCEL",
        Mode::Gear { .. } => "A CHOOSE   B BACK",
        Mode::Cart { .. } => "A MOVE ACROSS   B BACK",
        Mode::Build { confirming: true, .. } => "A YES   B NO",
        Mode::Build { row, .. } if menu.works.as_ref().and_then(|w| w.next()).is_some_and(|s| row < s.needs.len()) => "A PUT IN   B CLOSE",
        Mode::Build { .. } => "A BUILD   B CLOSE",
    };
    text(bg3, tiles, GRID_COL, LAST_ROW - 1, help);
    // The list can be sorted: said at the right end, out of the way.
    if let Mode::Crafting { .. } = menu.mode {
        right_text(bg3, tiles, LAST_ROW - 1, "Y SORT");
    }
}

/// `s` on `row`, ending in the rightmost column used (the R of the tabs).
fn right_text(bg3: &mut Bg3Layer, tiles: &HudTiles, row: usize, s: &str) {
    text(bg3, tiles, TAB_R_COL + 1 - s.len(), row, s);
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

/// The build page: the mill's next stage, what it takes (left at the site
/// a few at a time) and the tools it's built with, and BUILD; the cursor
/// goes down the materials and on to BUILD.
fn draw_build_page(bg3: &mut Bg3Layer, tiles: &HudTiles, inventory: &Inventory, menu: &BagMenu) {
    let Some(works) = menu.works.as_ref() else { return };
    let Some(stage) = works.next() else { return };
    let row_on = match menu.mode {
        Mode::Build { row, .. } => Some(row),
        _ => None,
    };
    text(bg3, tiles, GRID_COL, TAB_ROW, "BUILDING THE MILL");
    text(bg3, tiles, GRID_COL, GRID_ROW, &format!("STAGE: {}", stage.name));
    let mut row = GRID_ROW + 2;
    text(bg3, tiles, GRID_COL, row, "MATERIALS");
    text(bg3, tiles, BUILD_NEED_COL - 2, row, "NEED");
    text(bg3, tiles, BUILD_IN_COL, row, "IN");
    text(bg3, tiles, BUILD_HAVE_COL - 2, row, "HAVE");
    for (i, &(kind, n)) in stage.needs.iter().enumerate() {
        row += 1;
        let (left, have) = (works.delivered(kind), menu.at_hand(inventory, kind));
        let palette = need_row(bg3, tiles, row, Supply::of(left + have, n));
        if row_on == Some(i) {
            text(bg3, tiles, GRID_COL - 1, row, ">");
        }
        text_in(bg3, tiles, GRID_COL + 2, row, kind.name(), palette);
        text_in(bg3, tiles, BUILD_NEED_COL, row, &format!("{n:>2}"), palette);
        text_in(bg3, tiles, BUILD_IN_COL, row, &format!("{left:>2}"), palette);
        text_in(bg3, tiles, BUILD_HAVE_COL, row, &format!("{:>2}", have.min(99)), palette);
    }
    if !stage.tools.is_empty() {
        row += 2;
        text(bg3, tiles, GRID_COL, row, "TOOLS");
        text(bg3, tiles, BUILD_HAVE_COL - 2, row, "HAVE");
        for &tool in stage.tools {
            row += 1;
            let have = Supply::from(inventory.carries(tool));
            let palette = need_row(bg3, tiles, row, have);
            text_in(bg3, tiles, GRID_COL + 2, row, tool.name(), palette);
            text_in(bg3, tiles, BUILD_HAVE_COL - 1, row, if have == Supply::Enough { "YES" } else { " NO" }, palette);
        }
    }
    if row_on == Some(stage.needs.len()) {
        text(bg3, tiles, GRID_COL - 1, BUILD_ROW, ">");
    }
    let ready = menu.why_not_build(inventory).is_none();
    text_in(bg3, tiles, GRID_COL, BUILD_ROW, "BUILD", if ready { HUD_TEXT } else { HUD_DIM });
    if let Mode::Build { row, confirming: true } = menu.mode {
        let ask = match stage.needs.get(row) {
            Some(&(kind, _)) => format!("PUT IN {} X{}?", kind.name(), menu.to_put_in(inventory, kind)),
            None => format!("BUILD THE {}?", stage.name),
        };
        text(bg3, tiles, GRID_COL, NOTICE_ROW, &fit(&ask, HUD_COLS - GRID_COL));
    } else if let Some(n) = menu.notice {
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
        Mode::Recipe { recipe, scroll, confirming } => {
            draw_recipe(bg3, tiles, list, sprites, inventory, menu, recipe, scroll);
            if confirming {
                text(bg3, tiles, GRID_COL, NOTICE_ROW, &fit(&format!("MAKE {}?", RECIPES[recipe].makes.name()), HUD_COLS - GRID_COL));
            }
        }
        Mode::Crafting { choice } | Mode::Sorting { choice, .. } => draw_recipe_list(bg3, tiles, list, sprites, inventory, menu, Some(choice)),
        _ => draw_recipe_list(bg3, tiles, list, sprites, inventory, menu, None),
    }
    if let Some(n) = menu.notice {
        text(bg3, tiles, GRID_COL, NOTICE_ROW, n);
    }
    draw_help(bg3, tiles, menu);
    // A MAKE greyed while something's short; it can still be pressed, to
    // hear why.
    if let Mode::Recipe { recipe, confirming: false, .. } = menu.mode
        && !menu.can_make_now(inventory, &RECIPES[recipe])
    {
        text_in(bg3, tiles, GRID_COL, LAST_ROW - 1, "A MAKE", HUD_DIM);
    }
}

/// A page of the recipes thought of so far, in the order chosen (said
/// under the list, beside the page number): those that can be made now
/// lit, the rest greyed, a flame by those made at a fire, a "!" by those
/// not yet looked at; and a picture of the chosen one's result, or while
/// choosing an order, the orders to choose from. `choice` (a place in the
/// list) is None while only previewing from the tab row. Recipes not yet
/// thought of aren't shown at all, not even as a count.
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
    let listed = menu.recipe_list(inventory);
    let sorting = match menu.mode {
        Mode::Sorting { pick, .. } => Some(pick),
        _ => None,
    };
    let at = choice.unwrap_or(0);
    let page = at / RECIPES_PER_PAGE;
    let pages = listed.len().div_ceil(RECIPES_PER_PAGE);
    let first = page * RECIPES_PER_PAGE;
    for (place, &i) in listed.iter().enumerate().skip(first).take(RECIPES_PER_PAGE) {
        let r = &RECIPES[i];
        let row = GRID_ROW + (place - first) * 2;
        if choice == Some(place) && sorting.is_none() {
            text(bg3, tiles, GRID_COL, row, ">");
        }
        // Bright if it can be made now, whichever way.
        let ready = book.ways(i).iter().any(|&w| menu.can_make_now(inventory, &RECIPES[w]));
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
    let row = GRID_ROW + RECIPES_PER_PAGE * 2;
    right_text(bg3, tiles, row, &format!("SORT: {}", book.order().label()));
    if pages > 1 {
        let label = format!("{}/{}", page + 1, pages);
        text_in(bg3, tiles, GRID_COL + 1, row, "<", if page > 0 { HUD_TEXT } else { HUD_DIM });
        text(bg3, tiles, GRID_COL + 3, row, &label);
        text_in(bg3, tiles, GRID_COL + 4 + label.len(), row, ">", if page + 1 < pages { HUD_TEXT } else { HUD_DIM });
    }
    if let Some(pick) = sorting {
        text(bg3, tiles, PREVIEW_COL + 1, GRID_ROW, "SORT");
        for (i, order) in RecipeOrder::ALL.iter().enumerate() {
            if i == pick {
                text(bg3, tiles, PREVIEW_COL, GRID_ROW + 2 + i, ">");
            }
            text(bg3, tiles, PREVIEW_COL + 1, GRID_ROW + 2 + i, order.label());
        }
        return;
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

/// One recipe opened: what it makes and what that's for, then what it
/// needs: the materials used up, with how many are needed and carried,
/// and under them what isn't used up, the tools carried and a fire or
/// the lathe to be beside. Short ones are marked (see `need_row`). What
/// it needs scrolls, with a ▲ or ▼ where there's more, red if something
/// lacking is out of sight. An item made more than one way shows which
/// way this is (`< 1/2 >` under its name), with a `!` on the side of a
/// way not yet looked at.
#[allow(clippy::too_many_arguments)]
fn draw_recipe(
    bg3: &mut Bg3Layer,
    tiles: &HudTiles,
    list: &mut DrawList,
    sprites: &SpriteIds,
    inventory: &Inventory,
    menu: &BagMenu,
    index: usize,
    scroll: usize,
) {
    let book = &inventory.recipes;
    let recipe = &RECIPES[index];
    let makes = recipe.makes;
    let (w, h) = footprint(makes, false);
    text(bg3, tiles, GRID_COL, GRID_ROW, makes.name());
    let mut col = GRID_COL;
    if makes.goes_in_bag() {
        let size = format!("SIZE {w}X{h}");
        text(bg3, tiles, GRID_COL, GRID_ROW + 1, &size);
        col += size.len() + 2;
    }
    let (gfx, pal) = sprites.bag.icon(makes, false);
    list.sky(ObjEntry::new((ICON_RIGHT as i32 * 8) - w * CELL, GRID_ROW as i32 * 8, gfx, pal, 3));
    for (i, line) in wrap(makes.description(), desc_width(w)).iter().enumerate() {
        text(bg3, tiles, GRID_COL, DESC_ROW + i, line);
    }

    let ways = book.ways(index);
    if ways.len() > 1 {
        let at = ways.iter().position(|&w| w == index).unwrap_or(0);
        let (left, right) = (&ways[..at], &ways[at + 1..]);
        let col = col + 1;
        let row = GRID_ROW + 1;
        let label = format!("{}/{}", at + 1, ways.len());
        text_in(bg3, tiles, col, row, "<", if left.is_empty() { HUD_DIM } else { HUD_TEXT });
        text(bg3, tiles, col + 2, row, &label);
        text_in(bg3, tiles, col + 3 + label.len(), row, ">", if right.is_empty() { HUD_DIM } else { HUD_TEXT });
        if left.iter().any(|&w| book.is_fresh(w)) {
            text_in(bg3, tiles, col - 1, row, "!", HUD_FOOD);
        }
        if right.iter().any(|&w| book.is_fresh(w)) {
            text_in(bg3, tiles, col + 4 + label.len(), row, "!", HUD_FOOD);
        }
    }

    let lines = need_lines(recipe);
    for (i, &line) in lines.iter().enumerate().skip(scroll).take(NEEDS_SHOWN) {
        draw_need(bg3, tiles, inventory, menu, NEEDS_ROW + i - scroll, line);
    }
    let short = |l: &NeedLine| supply(inventory, menu, *l) != Supply::Enough;
    let below = scroll + NEEDS_SHOWN;
    if scroll > 0 {
        let palette = if lines[..scroll].iter().any(short) { HUD_HEART } else { HUD_TEXT };
        text_in(bg3, tiles, SCROLL_COL, NEEDS_ROW, "▲", palette);
    }
    if lines.len() > below {
        let palette = if lines[below..].iter().any(short) { HUD_HEART } else { HUD_TEXT };
        text_in(bg3, tiles, SCROLL_COL, NEEDS_ROW + NEEDS_SHOWN - 1, "▼", palette);
    }
}

/// How wide a recipe's description runs beside a picture `w` cells wide.
fn desc_width(w: i32) -> usize {
    DESC_WIDTH.min(ICON_RIGHT - w as usize * 2 - GRID_COL - 1)
}

/// How much of what a line of a recipe's needs asks for is at hand
/// (headings and gaps ask for nothing).
fn supply(inventory: &Inventory, menu: &BagMenu, line: NeedLine) -> Supply {
    match line {
        NeedLine::Material(kind, n) => Supply::of(inventory.bag.count(kind), n),
        NeedLine::Tool(kind) => Supply::from(inventory.carries(kind)),
        NeedLine::Fire => Supply::from(menu.near_fire),
        NeedLine::Lathe => Supply::from(menu.near_lathe),
        NeedLine::Materials | NeedLine::Tools | NeedLine::Gap => Supply::Enough,
    }
}

fn draw_need(bg3: &mut Bg3Layer, tiles: &HudTiles, inventory: &Inventory, menu: &BagMenu, row: usize, line: NeedLine) {
    let have = supply(inventory, menu, line);
    let yes_no = |bg3: &mut Bg3Layer, palette| text_in(bg3, tiles, HAVE_COL - 1, row, if have == Supply::Enough { "YES" } else { " NO" }, palette);
    match line {
        NeedLine::Materials => {
            text(bg3, tiles, GRID_COL, row, "MATERIALS");
            text(bg3, tiles, NEED_COL - 2, row, "NEED");
            text(bg3, tiles, HAVE_COL - 2, row, "HAVE");
        }
        NeedLine::Tools => {
            text(bg3, tiles, GRID_COL, row, "TOOLS");
            text(bg3, tiles, HAVE_COL - 2, row, "HAVE");
        }
        NeedLine::Gap => {}
        NeedLine::Material(kind, n) => {
            let palette = need_row(bg3, tiles, row, have);
            text_in(bg3, tiles, GRID_COL + 2, row, kind.name(), palette);
            text_in(bg3, tiles, NEED_COL, row, &format!("{n:>2}"), palette);
            text_in(bg3, tiles, HAVE_COL, row, &format!("{:>2}", inventory.bag.count(kind).min(99)), palette);
        }
        NeedLine::Tool(kind) => {
            let palette = need_row(bg3, tiles, row, have);
            text_in(bg3, tiles, GRID_COL + 2, row, kind.name(), palette);
            yes_no(bg3, palette);
        }
        NeedLine::Fire | NeedLine::Lathe => {
            let palette = need_row(bg3, tiles, row, have);
            let (mark, name) = if line == NeedLine::Fire { ("^", "A FIRE") } else { (LATHE_MARK, "A LATHE") };
            text_in(bg3, tiles, GRID_COL + 2, row, mark, if have == Supply::Enough { HUD_FOOD } else { palette });
            text_in(bg3, tiles, GRID_COL + 4, row, name, palette);
            yes_no(bg3, palette);
        }
    }
}

/// How much of something a recipe needs is at hand.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Supply {
    Enough,
    /// Some, but fewer than needed.
    Short,
    None,
}

impl Supply {
    fn of(have: u32, need: u32) -> Self {
        if have >= need {
            Self::Enough
        } else if have > 0 {
            Self::Short
        } else {
            Self::None
        }
    }
}

impl From<bool> for Supply {
    fn from(have: bool) -> Self {
        if have { Self::Enough } else { Self::None }
    }
}

/// Starts a row of what a recipe needs and returns its palette. What's
/// missing stands out rather than fading (grey reads as "doesn't
/// matter"): too few in yellow, none at all in red with an X at the head.
fn need_row(bg3: &mut Bg3Layer, tiles: &HudTiles, row: usize, supply: Supply) -> u8 {
    match supply {
        Supply::Enough => HUD_TEXT,
        Supply::Short => HUD_FOOD,
        Supply::None => {
            text_in(bg3, tiles, GRID_COL, row, "X", HUD_HEART);
            HUD_HEART
        }
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

/// The picture of `kind` as it lies in the bag, beside the list of what
/// can go in a place (from row `top` down to the description), centred
/// there; turned on its side if it's too tall to stand up in that room.
fn draw_gear_picture(list: &mut DrawList, sprites: &SpriteIds, kind: ItemKind, top: usize) {
    let room = (GEAR_DESC_ROW - top) as i32 * 8;
    let (w, h) = footprint(kind, false);
    let turned = h * CELL > room && w * CELL <= room;
    let (w, h) = footprint(kind, turned);
    let x = GEAR_PIC_COL as i32 * 8 + (GEAR_PIC_WIDTH - w * CELL) / 2;
    let y = top as i32 * 8 + ((room - h * CELL) / 2).max(0);
    let (gfx, pal) = sprites.bag.icon(kind, turned);
    list.sky(ObjEntry::new(x, y, gfx, pal, 3));
}

/// `first` of `len` lines shown `room` at a time, scrolled to keep
/// `at` in view.
fn scroll_to(at: usize, room: usize) -> usize {
    (at + 1).saturating_sub(room)
}

/// The gear page: the places gear goes, what's in each; once one is
/// chosen, what can go there (and where each is kept: in the bag or on a
/// hook); and what the thing in question is.
fn draw_gear_page(bg3: &mut Bg3Layer, tiles: &HudTiles, list: &mut DrawList, sprites: &SpriteIds, inventory: &Inventory, menu: &BagMenu) {
    let (focus, picking) = match menu.mode {
        Mode::Gear { place } => (Some(place), None),
        Mode::GearPick { place, choice } => (Some(place), Some(choice)),
        _ => (None, None),
    };
    let gear = &inventory.gear;
    let places = Slot::ALL;
    let first = scroll_to(focus.unwrap_or(0), PLACES_SHOWN);
    for (i, &place) in places.iter().enumerate().skip(first).take(PLACES_SHOWN) {
        let row = GRID_ROW + i - first;
        let col = GEAR_COL;
        if focus == Some(i) {
            text_in(bg3, tiles, col, row, ">", if picking.is_some() { HUD_DIM } else { HUD_TEXT });
        }
        text(bg3, tiles, col + 1, row, place.label());
        match gear.worn(place) {
            Some(kind) => text_fit(bg3, tiles, GEAR_ITEM_COL, row, kind.name()),
            None => text_in(bg3, tiles, GEAR_ITEM_COL, row, "-", HUD_DIM),
        }
    }
    let shown = places.len().min(PLACES_SHOWN);
    if first > 0 {
        text_in(bg3, tiles, SCROLL_COL, GRID_ROW, "▲", HUD_DIM);
    }
    if first + shown < places.len() {
        text_in(bg3, tiles, SCROLL_COL, GRID_ROW + shown - 1, "▼", HUD_DIM);
    }
    let top = GRID_ROW + shown + 1;
    let mut about = focus.and_then(|i| gear.worn(places[i]));
    if let (Some(place), Some(choice)) = (focus, picking) {
        let choices = gear_choices(inventory, places[place]);
        let room = GEAR_DESC_ROW - 1 - top;
        let first = scroll_to(choice, room);
        for (i, c) in choices.iter().enumerate().skip(first).take(room) {
            let row = top + i - first;
            if i == choice {
                text(bg3, tiles, GEAR_COL, row, ">");
            }
            text(bg3, tiles, GEAR_COL + 1, row, &fit(c.label(), GEAR_NAME_WIDTH));
            if let GearChoice::Use(kind) = *c {
                let kept = inventory.hook_of(kind).map_or("BAG".to_string(), |h| format!("HOOK {}", h + 1));
                text_in(bg3, tiles, GEAR_FROM_COL, row, &kept, HUD_DIM);
            }
        }
        if first > 0 {
            text_in(bg3, tiles, SCROLL_COL, top, "▲", HUD_DIM);
        }
        if first + room < choices.len() {
            text_in(bg3, tiles, SCROLL_COL, top + room - 1, "▼", HUD_DIM);
        }
        about = match choices.get(choice) {
            Some(GearChoice::Use(kind)) => Some(*kind),
            _ => None,
        };
    }
    if let Some(kind) = about {
        draw_gear_picture(list, sprites, kind, top);
        for (i, line) in wrap(kind.description(), GEAR_DESC_WIDTH).iter().take(GEAR_DESC_LINES).enumerate() {
            text(bg3, tiles, GEAR_COL, GEAR_DESC_ROW + i, line);
        }
    }
    if let Some(n) = menu.notice {
        text(bg3, tiles, GEAR_COL, NOTICE_ROW, n);
    }
    draw_help(bg3, tiles, menu);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_recipe_description_ends_above_what_it_needs() {
        for r in RECIPES {
            let (w, _) = footprint(r.makes, false);
            let lines = wrap(r.makes.description(), desc_width(w));
            assert!(DESC_ROW + lines.len() < NEEDS_ROW, "{:?}'s description runs into its needs", r.makes);
            assert!(lines.iter().all(|l| l.len() <= desc_width(w)), "{:?}'s description runs under its picture", r.makes);
        }
    }

    #[test]
    fn every_gear_description_fits_under_the_gear_page() {
        for kind in ItemKind::ALL.iter().copied().filter(|k| k.slot().is_some()) {
            let lines = wrap(kind.description(), GEAR_DESC_WIDTH);
            assert!(lines.len() <= GEAR_DESC_LINES, "{kind:?}'s description runs too long");
            assert!(lines.iter().all(|l| l.len() <= GEAR_DESC_WIDTH), "{kind:?}'s description runs too wide");
        }
    }

    #[test]
    fn every_gear_picture_fits_beside_the_list() {
        let places = Slot::ALL.len();
        let room = (GEAR_DESC_ROW - (GRID_ROW + places + 1)) as i32 * 8;
        for kind in ItemKind::ALL.iter().copied().filter(|k| k.slot().is_some()) {
            let fits = [false, true].into_iter().map(|t| footprint(kind, t)).any(|(w, h)| w * CELL <= GEAR_PIC_WIDTH && h * CELL <= room);
            assert!(fits, "{kind:?}'s picture doesn't fit beside the list");
        }
    }

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

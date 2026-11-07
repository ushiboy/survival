//! Draws the bag screen: a full-screen subscreen below the HUD. The world
//! layers are switched off; BG3 draws the slots and text, and sprites the
//! item icons and cursor (BG3's priority bit is cleared so they show in
//! front of it).

use crate::assets::palettes::{HUD_DIM, HUD_FOOD, HUD_MENU, HUD_TEXT, OBJ_FX};
use crate::assets::{HudTiles, SpriteIds};
use crate::game::state::Inventory;
use crate::game::bag::{BAG_W, Bag, Placed, footprint};
use crate::crafting::{RECIPES, RecipeBook};
use crate::game::menu::{BagMenu, Mode, Page, RECIPES_PER_PAGE, commands_for, gear_choices};
use crate::items::Slot;
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
    if menu.page() == Page::Craft {
        draw_craft_page(bg3, tiles, list, sprites, bag, &inventory.recipes, menu);
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

fn draw_help(bg3: &mut Bg3Layer, tiles: &HudTiles, menu: &BagMenu) {
    let help = match menu.mode {
        Mode::Tabs(_) => "A OPEN   B CLOSE",
        Mode::Browse => "A CHOOSE   B CLOSE",
        Mode::Crafting { .. } => "A CHOOSE   B BACK",
        Mode::Recipe { .. } | Mode::Commands { .. } | Mode::GearPick { .. } => "A OK   B BACK",
        Mode::Holding(_) => "A PUT  Y TURN  B CANCEL",
        Mode::Gear { .. } => "A CHANGE   B BACK",
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

/// The crafting page: the recipe list, or one recipe opened from it.
fn draw_craft_page(bg3: &mut Bg3Layer, tiles: &HudTiles, list: &mut DrawList, sprites: &SpriteIds, bag: &Bag, book: &RecipeBook, menu: &BagMenu) {
    if menu.near_fire {
        text_in(bg3, tiles, INFO_COL + 3, TAB_ROW + 1, "^", HUD_FOOD);
        text(bg3, tiles, INFO_COL + 4, TAB_ROW + 1, "BY THE FIRE");
    }
    match menu.mode {
        Mode::Recipe { recipe, choice } => draw_recipe(bg3, tiles, list, sprites, bag, menu.near_fire, recipe, choice),
        Mode::Crafting { choice } => draw_recipe_list(bg3, tiles, list, sprites, bag, book, menu.near_fire, Some(choice)),
        _ => draw_recipe_list(bg3, tiles, list, sprites, bag, book, menu.near_fire, None),
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
fn draw_recipe_list(bg3: &mut Bg3Layer, tiles: &HudTiles, list: &mut DrawList, sprites: &SpriteIds, bag: &Bag, book: &RecipeBook, near_fire: bool, choice: Option<usize>) {
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
        let ready = bag.can_make(r) && (!r.at_fire || near_fire);
        let name = r.makes.name();
        text_in(bg3, tiles, GRID_COL + 1, row, name, if ready { HUD_TEXT } else { HUD_DIM });
        if r.at_fire {
            text_in(bg3, tiles, GRID_COL + 2 + name.len(), row, "^", if near_fire { HUD_FOOD } else { HUD_DIM });
        }
        if book.is_fresh(i) {
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
/// takes, whether it needs a fire, and MAKE / BACK.
#[allow(clippy::too_many_arguments)]
fn draw_recipe(bg3: &mut Bg3Layer, tiles: &HudTiles, list: &mut DrawList, sprites: &SpriteIds, bag: &Bag, near_fire: bool, index: usize, choice: usize) {
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
        let have = bag.has(kind);
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
/// room a pack adds), and the choices for the slot being changed.
fn draw_gear(bg3: &mut Bg3Layer, tiles: &HudTiles, inventory: &Inventory, menu: &BagMenu) {
    let (focus, picking) = match menu.mode {
        Mode::Gear { slot } => (Some(slot), None),
        Mode::GearPick { slot, choice } => (Some(slot), Some(choice)),
        _ => (None, None),
    };
    for (i, slot) in Slot::ALL.into_iter().enumerate() {
        let row = GRID_ROW + i * 4;
        if focus == Some(i) {
            text(bg3, tiles, INFO_COL, row, ">");
        }
        text(bg3, tiles, INFO_COL + 1, row, slot.label());
        let worn = inventory.gear.worn(slot);
        text(bg3, tiles, INFO_COL + 2, row + 1, worn.map_or("-", |k| k.name()));
        if let Some(k) = worn.filter(|k| k.extra_rows() > 0) {
            text(bg3, tiles, INFO_COL + 2, row + 2, &format!("+{} ROOM", k.extra_rows() * BAG_W));
        }
    }
    if let (Some(slot), Some(choice)) = (focus, picking) {
        let top = GRID_ROW + Slot::ALL.len() * 4;
        for (i, c) in gear_choices(&inventory.bag, &inventory.gear, Slot::ALL[slot]).iter().enumerate() {
            if i == choice {
                text(bg3, tiles, INFO_COL, top + i, ">");
            }
            text(bg3, tiles, INFO_COL + 1, top + i, c.label());
        }
    }
}

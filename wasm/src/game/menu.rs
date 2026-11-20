//! The bag screen: browse what's carried, use or put things down, and
//! rearrange items so more fit. Pure state and input handling; drawing lives
//! in `render::bag_screen` and the effects of using or dropping in `Game`.
//!
//! Controls: the d-pad moves the cursor (repeating while held), A confirms
//! and B cancels or goes back; everything else is picked from the screen.
//! The one exception is Y, which turns an item held for placing. X closes
//! the bag from anywhere but the hand.
//!
//! Pages are chosen on a tab row above the grid (pressing up off the top of
//! the grid or the crafting list reaches it), or with L and R, which open
//! the page beside the one shown (but not in the middle of something). On the items page, A on an
//! item opens its commands; B closes the bag. On the crafting page, A opens
//! the chosen recipe and B returns to the tabs; on a recipe, A asks
//! whether to make it and A again makes it from materials in the bag
//! (holding the result if there's no room for it), and B goes back. Holding an item, A
//! puts it down (swapping with the one item it covers, if any) and B
//! returns it where it came from (or, if it came off the ground or there's
//! no room left, leaves it on the ground). On the gear page, A on a slot
//! (hand, back) lists what can go there and TAKE OFF; B returns to the tabs.

use super::bag::{BAG_W, Bag, Placed, footprint};
use super::state::{Gear, Inventory};
use crate::common::input::{Buttons, InputState};
use crate::crafting::{RECIPES, RecipeOrder};
use crate::items::{ItemKind, Slot, Use};

/// Frames a direction is held before the cursor repeats, and between
/// repeats.
const REPEAT_DELAY: u32 = 14;
const REPEAT_EVERY: u32 = 5;

/// An item lifted out of the grid (or just picked up) and being placed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HeldItem {
    pub kind: ItemKind,
    pub turned: bool,
    /// How many are in hand (a stack is moved whole).
    pub count: u32,
    /// Where it was in the bag, or `None` if it came off the ground.
    pub origin: Option<Placed>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    Use(Use),
    Move,
    /// Take a tool in hand.
    Wield,
    /// Put a pack on the back or a belt round the waist.
    Wear,
    /// Hang a tool on the belt, out of the bag.
    Hang,
    /// Put it down on the ground (or pour it away).
    Drop,
}

impl Command {
    pub fn label(self, kind: ItemKind) -> &'static str {
        match self {
            Command::Use(Use::Eat) => "EAT",
            Command::Use(Use::Drink) => "DRINK",
            Command::Use(Use::Light) => "LIGHT FIRE",
            Command::Move => "MOVE",
            Command::Wield => "TAKE IN HAND",
            Command::Wear => "WEAR",
            Command::Hang => "HANG ON BELT",
            Command::Drop => kind.discard_words().0,
        }
    }
}

/// Pages of the bag screen, in tab order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    Items,
    Craft,
    Gear,
    /// Moving things between the bag and the handcart's load (only with
    /// the cart at hand).
    Cart,
}

impl Page {
    pub const ALL: [Page; 4] = [Page::Items, Page::Craft, Page::Gear, Page::Cart];

    pub fn label(self) -> &'static str {
        match self {
            Page::Items => "ITEMS",
            Page::Craft => "CRAFT",
            Page::Gear => "GEAR",
            Page::Cart => "CART",
        }
    }
}

/// A line of the gear page: a slot, or a hook of the belt worn at the
/// waist (listed after the slots, one for each hook).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GearRow {
    Slot(Slot),
    Hook(usize),
}

/// The lines of the gear page, top to bottom.
pub fn gear_rows(gear: &Gear) -> Vec<GearRow> {
    Slot::ALL.into_iter().map(GearRow::Slot).chain((0..gear.hooks()).map(GearRow::Hook)).collect()
}

/// A choice for a gear slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GearChoice {
    /// Hold, wear or hang this (carried in the bag; a tool to hold may
    /// also be on the belt).
    Use(ItemKind),
    TakeOff,
}

impl GearChoice {
    pub fn label(self) -> &'static str {
        match self {
            GearChoice::Use(kind) => kind.name(),
            GearChoice::TakeOff => "TAKE OFF",
        }
    }
}

/// What can go on a line of the gear page: each kind of gear for it
/// carried (but the one already there), then taking off what's there. A
/// tool to hold may come from the bag or the belt; one to hang, only from
/// the bag.
pub fn gear_choices(inv: &Inventory, row: GearRow) -> Vec<GearChoice> {
    let (fits, there): (fn(ItemKind) -> bool, Option<ItemKind>) = match row {
        GearRow::Slot(Slot::Hand) => (|k| k.slot() == Some(Slot::Hand), inv.gear.hand),
        GearRow::Slot(Slot::Back) => (|k| k.slot() == Some(Slot::Back), inv.gear.back),
        GearRow::Slot(Slot::Waist) => (|k| k.slot() == Some(Slot::Waist), inv.gear.waist),
        GearRow::Hook(h) => (ItemKind::hangs_on_belt, inv.gear.belt[h]),
    };
    let from_belt = match row {
        GearRow::Slot(Slot::Hand) => inv.gear.belt.iter().flatten().copied().collect(),
        _ => Vec::new(),
    };
    let mut out: Vec<GearChoice> = Vec::new();
    for kind in inv.bag.items().iter().map(|p| p.kind).chain(from_belt) {
        let choice = GearChoice::Use(kind);
        if fits(kind) && Some(kind) != there && !out.contains(&choice) {
            out.push(choice);
        }
    }
    if there.is_some() {
        out.push(GearChoice::TakeOff);
    }
    out
}

/// Recipes listed on one page of the crafting list.
pub const RECIPES_PER_PAGE: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// On the tab row, about to pick a page.
    Tabs(Page),
    Browse,
    /// Choosing what to do with the item at `item`.
    Commands {
        item: usize,
        choice: usize,
    },
    Holding(HeldItem),
    /// Choosing something to make from the recipes thought of so far
    /// (`choice` is a place in that list).
    Crafting {
        choice: usize,
    },
    /// Choosing the order the recipe list is sorted in: `pick` is a place
    /// in `RecipeOrder::ALL`, `choice` the list's cursor to go back to.
    Sorting {
        choice: usize,
        pick: usize,
    },
    /// Looking over one recipe (index into `RECIPES`), what it needs
    /// scrolled down `scroll` lines; `confirming` while asking whether to
    /// make it. Left and right change to another way of making the same
    /// thing.
    Recipe {
        recipe: usize,
        scroll: usize,
        confirming: bool,
    },
    /// On a line of the gear page (index into `gear_rows`).
    Gear {
        slot: usize,
    },
    /// Choosing from `gear_choices` for a line.
    GearPick {
        slot: usize,
        choice: usize,
    },
    /// On the cart page, the cursor (`BagMenu::cursor`) on a cell of the
    /// bag or (`in_cart`) of the cart's load.
    Cart {
        in_cart: bool,
    },
}

/// What the game must do after a frame of the bag screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Stay,
    Close,
    /// An item was taken out (or, if it isn't used up, chosen) to be used;
    /// the bag closes unless the game finds it can't be used here.
    Use(ItemKind),
    /// Items (`count` of a kind) were taken out to leave on the ground.
    Drop(ItemKind, u32),
    /// The recipe (index into `RECIPES`) is to be cooked over the fire;
    /// its materials are still in the bag.
    Cook(usize),
    /// Something made that doesn't go in the bag (a handcart) is handed
    /// over to the game; its materials are used up.
    Made(ItemKind),
}

#[derive(Clone, Debug)]
pub struct BagMenu {
    /// Cell under the cursor; while holding, the held item's top-left cell.
    pub cursor: (i32, i32),
    pub mode: Mode,
    /// A short message about the last action.
    pub notice: Option<&'static str>,
    /// Standing by a campfire (recipes made at a fire are possible).
    pub near_fire: bool,
    /// Standing by the wood lathe (recipes turned on it are possible).
    pub near_lathe: bool,
    /// The handcart is at hand (being pulled, or set down close by): its
    /// load can be got at.
    pub cart_at_hand: bool,
    held_frames: u32,
}

/// Lines of what a recipe needs shown at once on its page; more scroll.
pub const NEEDS_SHOWN: usize = 9;

/// A line of what a recipe needs, as listed on its page: the materials
/// used up under one heading, then (if any) what isn't used up under
/// another: the tools carried, and a fire or the lathe to be beside.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NeedLine {
    Materials,
    Material(ItemKind, u32),
    Gap,
    Tools,
    Tool(ItemKind),
    Fire,
    Lathe,
}

pub fn need_lines(recipe: &crate::crafting::Recipe) -> Vec<NeedLine> {
    let mut lines = vec![NeedLine::Materials];
    lines.extend(recipe.needs.iter().map(|&(kind, n)| NeedLine::Material(kind, n)));
    let place = [(recipe.at_fire, NeedLine::Fire), (recipe.at_lathe, NeedLine::Lathe)].into_iter().filter(|p| p.0).map(|p| p.1);
    let others: Vec<NeedLine> = recipe.tools.iter().map(|&t| NeedLine::Tool(t)).chain(place).collect();
    if !others.is_empty() {
        lines.extend([NeedLine::Gap, NeedLine::Tools]);
        lines.extend(others);
    }
    lines
}

/// `scroll` moved `dy` lines through `lines` lines of needs, no further
/// than the last line at the bottom.
pub fn scrolled(scroll: usize, dy: i32, lines: usize) -> usize {
    let last = lines.saturating_sub(NEEDS_SHOWN) as i32;
    (scroll as i32 + dy).clamp(0, last) as usize
}

/// Why `recipe` can't be made here and now, if it can't.
fn why_not(menu: &BagMenu, inv: &Inventory, recipe: &crate::crafting::Recipe) -> Option<&'static str> {
    if !inv.can_make(recipe) {
        return Some(if recipe.tools.iter().all(|&t| inv.carries(t)) { "NOT ENOUGH MATERIALS" } else { "MISSING A TOOL" });
    }
    if recipe.at_fire && !menu.near_fire {
        return Some("NEEDS A FIRE");
    }
    if recipe.at_lathe && !menu.near_lathe {
        return Some("NEEDS A LATHE");
    }
    // A handcart: there's only ever the one.
    if recipe.makes == ItemKind::Handcart && inv.cart.is_some() {
        return Some("YOU HAVE ONE ALREADY");
    }
    None
}

/// Commands offered for an item, in menu order: taking a tool in hand
/// (unless it's already held) or wearing a pack or a belt come after
/// using it, then hanging a tool on the belt (when one is worn with a
/// hook free).
pub fn commands_for(kind: ItemKind, gear: &Gear) -> Vec<Command> {
    let gear_command = match kind.slot() {
        Some(Slot::Hand) if gear.hand != Some(kind) => Some(Command::Wield),
        Some(Slot::Back | Slot::Waist) => Some(Command::Wear),
        _ => None,
    };
    let hook_free = gear.belt[..gear.hooks()].contains(&None);
    let hang = (kind.hangs_on_belt() && hook_free).then_some(Command::Hang);
    kind.usage().map(Command::Use).into_iter().chain(gear_command).chain(hang).chain([Command::Move, Command::Drop]).collect()
}

impl BagMenu {
    /// Whether this is the place to make `recipe`: by a fire, or by the
    /// lathe, if it must be made there.
    pub fn in_place_for(&self, recipe: &crate::crafting::Recipe) -> bool {
        (!recipe.at_fire || self.near_fire) && (!recipe.at_lathe || self.near_lathe)
    }

    /// The recipes on the CRAFT list, top to bottom, sorted in the order
    /// chosen (what can be made counts the fire and the lathe).
    pub fn recipe_list(&self, inv: &Inventory) -> Vec<usize> {
        inv.recipes.sorted(|w| inv.can_make(&RECIPES[w]) && self.in_place_for(&RECIPES[w]))
    }

    /// The pages to choose from: the cart's only with it at hand.
    pub fn pages(&self) -> Vec<Page> {
        Page::ALL.into_iter().filter(|&p| p != Page::Cart || self.cart_at_hand).collect()
    }

    pub fn browse() -> Self {
        Self { cursor: (0, 0), mode: Mode::Browse, notice: None, near_fire: false, near_lathe: false, cart_at_hand: false, held_frames: 0 }
    }

    /// Opens holding something just picked up (`count` of a kind) that
    /// didn't fit anywhere.
    pub fn holding(kind: ItemKind, count: u32) -> Self {
        let held = HeldItem { kind, turned: false, count, origin: None };
        Self { mode: Mode::Holding(held), notice: Some("NO ROOM - MAKE SPACE"), ..Self::browse() }
    }

    /// How many are in hand while placing.
    pub fn held_count(&self) -> u32 {
        match self.mode {
            Mode::Holding(h) => h.count,
            _ => 0,
        }
    }

    /// Where the held item would go at the cursor.
    pub fn held_placement(&self) -> Option<Placed> {
        match self.mode {
            Mode::Holding(h) => Some(Placed { kind: h.kind, x: self.cursor.0, y: self.cursor.1, turned: h.turned, count: h.count }),
            _ => None,
        }
    }

    /// The page being shown.
    pub fn page(&self) -> Page {
        match self.mode {
            Mode::Tabs(page) => page,
            Mode::Crafting { .. } | Mode::Sorting { .. } | Mode::Recipe { .. } => Page::Craft,
            Mode::Gear { .. } | Mode::GearPick { .. } => Page::Gear,
            Mode::Cart { .. } => Page::Cart,
            _ => Page::Items,
        }
    }

    fn open(&mut self, page: Page) {
        self.notice = None;
        self.mode = match page {
            Page::Items => Mode::Browse,
            Page::Craft => Mode::Crafting { choice: 0 },
            Page::Gear => Mode::Gear { slot: 0 },
            Page::Cart => {
                self.cursor = (0, 0);
                Mode::Cart { in_cart: false }
            }
        };
    }

    /// L or R (`dx` -1 or 1): on the tab row, moves along it; on a page,
    /// opens the one beside it. Not in the middle of something (a command
    /// list, a recipe, a gear choice, an item in hand). Whether it was
    /// taken.
    fn switch_page(&mut self, dx: i32) -> bool {
        let on_tabs = matches!(self.mode, Mode::Tabs(_));
        if !on_tabs && !matches!(self.mode, Mode::Browse | Mode::Crafting { .. } | Mode::Gear { .. } | Mode::Cart { .. }) {
            return false;
        }
        let page = self.page();
        let pages = self.pages();
        let i = pages.iter().position(|&p| p == page).unwrap_or(0) as i32;
        let next = pages[(i + dx).clamp(0, pages.len() as i32 - 1) as usize];
        if next != page {
            if on_tabs {
                self.mode = Mode::Tabs(next);
                self.notice = None;
            } else {
                self.open(next);
            }
        }
        true
    }

    pub fn update(&mut self, input: &InputState, inv: &mut Inventory) -> Outcome {
        let step = self.direction(input);
        if input.pressed(Buttons::X) && !matches!(self.mode, Mode::Holding(_)) {
            return Outcome::Close;
        }
        if let Some(dx) = [(Buttons::L, -1), (Buttons::R, 1)].into_iter().find(|&(b, _)| input.pressed(b)).map(|(_, dx)| dx)
            && self.switch_page(dx)
        {
            return Outcome::Stay;
        }
        match self.mode {
            Mode::Tabs(page) => {
                let pages = self.pages();
                let i = pages.iter().position(|&p| p == page).unwrap_or(0) as i32;
                match step {
                    Some((dx, 0)) => {
                        let next = pages[(i + dx).clamp(0, pages.len() as i32 - 1) as usize];
                        if next != page {
                            self.mode = Mode::Tabs(next);
                            self.notice = None;
                        }
                    }
                    Some((0, 1)) => self.open(page),
                    _ => {}
                }
                if input.pressed(Buttons::A) {
                    self.open(page);
                } else if input.pressed(Buttons::B) {
                    return Outcome::Close;
                }
                Outcome::Stay
            }
            Mode::Browse => {
                if let Some((dx, dy)) = step {
                    if dy < 0 && self.cursor.1 == 0 {
                        self.mode = Mode::Tabs(Page::Items);
                        return Outcome::Stay;
                    }
                    self.cursor = ((self.cursor.0 + dx).clamp(0, BAG_W - 1), (self.cursor.1 + dy).clamp(0, inv.bag.rows() - 1));
                }
                if input.pressed(Buttons::B) {
                    return Outcome::Close;
                }
                if input.pressed(Buttons::A)
                    && let Some(item) = inv.bag.at(self.cursor.0, self.cursor.1)
                {
                    self.mode = Mode::Commands { item, choice: 0 };
                    self.notice = None;
                }
                Outcome::Stay
            }
            Mode::Commands { item, choice } => {
                let bag = &mut inv.bag;
                let kind = bag.items()[item].kind;
                let commands = commands_for(kind, &inv.gear);
                if let Some((_, dy)) = step.filter(|s| s.1 != 0) {
                    self.mode = Mode::Commands { item, choice: (choice as i32 + dy).rem_euclid(commands.len() as i32) as usize };
                }
                if input.pressed(Buttons::B) {
                    self.mode = Mode::Browse;
                    return Outcome::Stay;
                }
                if !input.pressed(Buttons::A) {
                    return Outcome::Stay;
                }
                self.mode = Mode::Browse;
                match commands[choice] {
                    Command::Use(how) => {
                        if how.uses_up() {
                            bag.take_one(item);
                        }
                        Outcome::Use(kind)
                    }
                    Command::Wield => {
                        inv.gear.hand = Some(kind);
                        self.notice = Some("TOOK IT IN HAND");
                        Outcome::Stay
                    }
                    Command::Wear => {
                        self.notice = Some(match inv.wear(item) {
                            Ok(()) => "PUT IT ON",
                            Err(why) => why,
                        });
                        self.cursor = (self.cursor.0, self.cursor.1.min(inv.bag.rows() - 1));
                        Outcome::Stay
                    }
                    Command::Hang => {
                        self.notice = Some(match inv.hang(item) {
                            Ok(()) => "HUNG IT ON THE BELT",
                            Err(why) => why,
                        });
                        Outcome::Stay
                    }
                    Command::Move => {
                        let p = bag.remove(item);
                        self.cursor = (p.x, p.y);
                        self.mode = Mode::Holding(HeldItem { kind, turned: p.turned, count: p.count, origin: Some(p) });
                        Outcome::Stay
                    }
                    Command::Drop => {
                        let p = bag.remove(item);
                        self.notice = Some(kind.discard_words().1);
                        Outcome::Drop(kind, p.count)
                    }
                }
            }
            Mode::Holding(held) => self.update_holding(input, &mut inv.bag, held, step),
            Mode::Cart { in_cart } => self.update_cart(input, inv, in_cart, step),
            Mode::Crafting { choice } => self.update_crafting(input, inv, choice, step),
            Mode::Sorting { choice, pick } => {
                let orders = RecipeOrder::ALL;
                let mut pick = pick;
                if let Some((_, dy)) = step.filter(|s| s.1 != 0) {
                    pick = (pick as i32 + dy).rem_euclid(orders.len() as i32) as usize;
                }
                self.mode = Mode::Sorting { choice, pick };
                if input.pressed(Buttons::B) || input.pressed(Buttons::Y) {
                    self.mode = Mode::Crafting { choice };
                } else if input.pressed(Buttons::A) {
                    inv.recipes.set_order(orders[pick]);
                    self.mode = Mode::Crafting { choice: 0 };
                }
                Outcome::Stay
            }
            Mode::Recipe { recipe, scroll, confirming } => self.update_recipe(input, inv, recipe, scroll, confirming, step),
            Mode::Gear { slot } => {
                let rows = gear_rows(&inv.gear);
                if let Some((_, dy)) = step.filter(|s| s.1 != 0) {
                    if dy < 0 && slot == 0 {
                        self.mode = Mode::Tabs(Page::Gear);
                        return Outcome::Stay;
                    }
                    self.mode = Mode::Gear { slot: (slot as i32 + dy).clamp(0, rows.len() as i32 - 1) as usize };
                    self.notice = None;
                }
                if input.pressed(Buttons::B) {
                    self.mode = Mode::Tabs(Page::Gear);
                    self.notice = None;
                } else if input.pressed(Buttons::A) {
                    if gear_choices(inv, rows[slot]).is_empty() {
                        self.notice = Some("NOTHING TO PUT HERE");
                    } else {
                        self.mode = Mode::GearPick { slot, choice: 0 };
                        self.notice = None;
                    }
                }
                Outcome::Stay
            }
            Mode::GearPick { slot, choice } => {
                let row = gear_rows(&inv.gear)[slot];
                let choices = gear_choices(inv, row);
                if let Some((_, dy)) = step.filter(|s| s.1 != 0) {
                    self.mode = Mode::GearPick { slot, choice: (choice as i32 + dy).rem_euclid(choices.len() as i32) as usize };
                }
                if input.pressed(Buttons::B) {
                    self.mode = Mode::Gear { slot };
                } else if input.pressed(Buttons::A) {
                    let in_bag = |inv: &Inventory, kind| inv.bag.items().iter().position(|p| p.kind == kind);
                    let result = match (choices[choice], row) {
                        (GearChoice::Use(kind), GearRow::Slot(Slot::Hand)) => {
                            inv.gear.hand = Some(kind);
                            Ok(())
                        }
                        (GearChoice::Use(kind), GearRow::Slot(_)) => in_bag(inv, kind).map_or(Ok(()), |i| inv.wear(i)),
                        (GearChoice::Use(kind), GearRow::Hook(h)) => in_bag(inv, kind).map_or(Ok(()), |i| inv.hang_at(i, h)),
                        (GearChoice::TakeOff, GearRow::Slot(s)) => inv.take_off(s),
                        (GearChoice::TakeOff, GearRow::Hook(h)) => inv.unhang(h),
                    };
                    let done = match (choices[choice], row) {
                        (GearChoice::TakeOff, _) => "TOOK IT OFF",
                        (_, GearRow::Slot(Slot::Hand)) => "TOOK IT IN HAND",
                        (_, GearRow::Hook(_)) => "HUNG IT ON THE BELT",
                        (_, GearRow::Slot(_)) => "PUT IT ON",
                    };
                    self.notice = Some(result.err().unwrap_or(done));
                    // Taking the belt off takes its hooks off the page.
                    let last = gear_rows(&inv.gear).len() - 1;
                    self.mode = Mode::Gear { slot: slot.min(last) };
                    self.cursor = (self.cursor.0, self.cursor.1.min(inv.bag.rows() - 1));
                }
                Outcome::Stay
            }
        }
    }

    /// The list of recipes thought of so far (`choice` is a place in it):
    /// up and down through it, left and right a page at a time; A opens the
    /// recipe, which is then no longer fresh; Y opens the menu of orders to
    /// sort it in.
    fn update_crafting(&mut self, input: &InputState, inv: &mut Inventory, mut choice: usize, step: Option<(i32, i32)>) -> Outcome {
        let listed = self.recipe_list(inv);
        let last = listed.len() as i32 - 1;
        match step {
            Some((0, dy)) if dy < 0 && choice == 0 => {
                self.mode = Mode::Tabs(Page::Craft);
                return Outcome::Stay;
            }
            Some((0, dy)) => choice = (choice as i32 + dy).clamp(0, last.max(0)) as usize,
            Some((dx, _)) => {
                let page = (choice / RECIPES_PER_PAGE) as i32 + dx;
                if (0..=last / RECIPES_PER_PAGE as i32).contains(&page) {
                    choice = (page * RECIPES_PER_PAGE as i32 + (choice % RECIPES_PER_PAGE) as i32).min(last) as usize;
                }
            }
            None => {}
        }
        self.mode = Mode::Crafting { choice };
        if input.pressed(Buttons::B) {
            self.mode = Mode::Tabs(Page::Craft);
            self.notice = None;
        } else if input.pressed(Buttons::Y) {
            let pick = RecipeOrder::ALL.iter().position(|&o| o == inv.recipes.order()).unwrap_or(0);
            self.mode = Mode::Sorting { choice, pick };
            self.notice = None;
        } else if let Some(&recipe) = listed.get(choice).filter(|_| input.pressed(Buttons::A)) {
            inv.recipes.looked_at(recipe);
            self.mode = Mode::Recipe { recipe, scroll: 0, confirming: false };
            self.notice = None;
        }
        Outcome::Stay
    }

    /// One recipe: A asks whether to make it (or says why it can't be
    /// made), and A again makes it; B goes back (or, while asking, doesn't
    /// make it). Up and down scroll what it needs. Making keeps the recipe
    /// open so more can be made, unless it's cooked at the fire (the bag
    /// closes) or there's no room for it (it's held to be placed).
    fn update_recipe(
        &mut self,
        input: &InputState,
        inv: &mut Inventory,
        mut index: usize,
        mut scroll: usize,
        confirming: bool,
        step: Option<(i32, i32)>,
    ) -> Outcome {
        if confirming {
            if input.pressed(Buttons::B) {
                self.mode = Mode::Recipe { recipe: index, scroll, confirming: false };
                self.notice = None;
                return Outcome::Stay;
            }
            if input.pressed(Buttons::A) {
                self.mode = Mode::Recipe { recipe: index, scroll, confirming: false };
                return self.make(inv, index);
            }
            return Outcome::Stay;
        }
        match step {
            Some((0, dy)) => scroll = scrolled(scroll, dy, need_lines(&RECIPES[index]).len()),
            // Another way of making the same thing, if there is one.
            Some((dx, _)) => {
                let ways = inv.recipes.ways(index);
                let at = ways.iter().position(|&i| i == index).unwrap_or(0) as i32;
                if let Some(&next) = ways.get((at + dx).clamp(0, ways.len() as i32 - 1) as usize)
                    && next != index
                {
                    index = next;
                    scroll = 0;
                    inv.recipes.looked_at(index);
                    self.notice = None;
                }
            }
            None => {}
        }
        self.mode = Mode::Recipe { recipe: index, scroll, confirming: false };
        if input.pressed(Buttons::B) {
            let makes = RECIPES[index].makes;
            let place = self.recipe_list(inv).iter().position(|&i| RECIPES[i].makes == makes).unwrap_or(0);
            self.mode = Mode::Crafting { choice: place };
            self.notice = None;
        } else if input.pressed(Buttons::A) {
            match why_not(self, inv, &RECIPES[index]) {
                Some(why) => self.notice = Some(why),
                None => {
                    self.mode = Mode::Recipe { recipe: index, scroll, confirming: true };
                    self.notice = None;
                }
            }
        }
        Outcome::Stay
    }

    /// Makes the recipe at `index`, already found makeable.
    fn make(&mut self, inv: &mut Inventory, index: usize) -> Outcome {
        let recipe = &RECIPES[index];
        if let Some(why) = why_not(self, inv, recipe) {
            self.notice = Some(why);
            return Outcome::Stay;
        }
        if !recipe.makes.goes_in_bag() && !recipe.at_fire {
            inv.recipes.made(index);
            inv.bag.use_up(recipe);
            self.notice = Some("MADE IT!");
            return Outcome::Made(recipe.makes);
        }
        inv.recipes.made(index);
        if recipe.at_fire {
            // Cooked over the fire, outside the bag.
            return Outcome::Cook(index);
        }
        let bag = &mut inv.bag;
        bag.use_up(recipe);
        if bag.stow_one(recipe.makes) {
            inv.took(recipe.makes);
            self.notice = Some("MADE IT!");
        } else {
            *self = Self { near_fire: self.near_fire, near_lathe: self.near_lathe, cart_at_hand: self.cart_at_hand, ..Self::holding(recipe.makes, 1) };
        }
        Outcome::Stay
    }

    /// The cart page: the cursor goes over the bag's cells and on across
    /// into the cart's (and back); A moves the stack under it to the other
    /// side, as much of it as fits there; up off the top row or B goes
    /// back to the tabs.
    fn update_cart(&mut self, input: &InputState, inv: &mut Inventory, mut in_cart: bool, step: Option<(i32, i32)>) -> Outcome {
        let Some(cart) = inv.cart.as_mut() else {
            self.mode = Mode::Tabs(Page::Cart);
            return Outcome::Stay;
        };
        let size = |in_cart: bool, cart: &Bag| if in_cart { (cart.width(), cart.rows()) } else { (BAG_W, inv.bag.rows()) };
        if let Some((dx, dy)) = step {
            let (x, y) = (self.cursor.0 + dx, self.cursor.1 + dy);
            if y < 0 {
                self.mode = Mode::Tabs(Page::Cart);
                return Outcome::Stay;
            }
            let (w, _) = size(in_cart, cart);
            if !in_cart && x >= w {
                in_cart = true;
                self.cursor.0 = 0;
            } else if in_cart && x < 0 {
                in_cart = false;
                self.cursor.0 = BAG_W - 1;
            } else {
                self.cursor.0 = x.clamp(0, w - 1);
            }
            let (_, rows) = size(in_cart, cart);
            self.cursor.1 = y.clamp(0, rows - 1);
            self.notice = None;
        }
        self.mode = Mode::Cart { in_cart };
        if input.pressed(Buttons::B) {
            self.mode = Mode::Tabs(Page::Cart);
            self.notice = None;
            return Outcome::Stay;
        }
        if !input.pressed(Buttons::A) {
            return Outcome::Stay;
        }
        let (from, to) = if in_cart { (cart, &mut inv.bag) } else { (&mut inv.bag, cart) };
        let Some(i) = from.at(self.cursor.0, self.cursor.1) else { return Outcome::Stay };
        let p = from.remove(i);
        let left = to.stow(p.kind, p.count);
        if left > 0 {
            // What doesn't fit stays where it was.
            from.put(Placed { count: left, ..p });
        }
        self.notice = Some(match (left, in_cart) {
            (n, _) if n == p.count => "NO ROOM",
            (0, false) => "INTO THE CART",
            (0, true) => "INTO THE BAG",
            _ => "SOME OF IT WOULDN'T FIT",
        });
        Outcome::Stay
    }

    fn update_holding(&mut self, input: &InputState, bag: &mut Bag, mut held: HeldItem, step: Option<(i32, i32)>) -> Outcome {
        if input.pressed(Buttons::Y) && held.kind.size().0 != held.kind.size().1 {
            held.turned = !held.turned;
            self.mode = Mode::Holding(held);
        }
        let (w, h) = footprint(held.kind, held.turned);
        let (dx, dy) = step.unwrap_or((0, 0));
        self.cursor = ((self.cursor.0 + dx).clamp(0, BAG_W - w), (self.cursor.1 + dy).clamp(0, bag.rows() - h));

        if input.pressed(Buttons::A) {
            let spot = Placed { kind: held.kind, x: self.cursor.0, y: self.cursor.1, turned: held.turned, count: held.count };
            match bag.overlapping(&spot)[..] {
                // Onto a stack of the same: top it up, keeping any left over.
                [other] if bag.items()[other].kind == held.kind && held.kind.max_stack() > 1 => {
                    let left = bag.add_to(other, held.count);
                    if left == 0 {
                        self.mode = Mode::Browse;
                        self.notice = None;
                    } else {
                        self.mode = Mode::Holding(HeldItem { count: left, ..held });
                        self.notice = Some("THAT STACK IS FULL");
                    }
                }
                [] => {
                    bag.put(spot);
                    self.mode = Mode::Browse;
                    self.notice = None;
                }
                [other] => {
                    // Swap: put this one down and pick up the one beneath.
                    let lifted = bag.remove(other);
                    bag.put(spot);
                    let next = HeldItem { kind: lifted.kind, turned: lifted.turned, count: lifted.count, origin: Some(lifted) };
                    let (w, h) = footprint(next.kind, next.turned);
                    self.cursor = (self.cursor.0.min(BAG_W - w), self.cursor.1.min(bag.rows() - h));
                    self.mode = Mode::Holding(next);
                    self.notice = None;
                }
                _ => self.notice = Some("IN THE WAY"),
            }
            return Outcome::Stay;
        }
        if input.pressed(Buttons::B) || input.pressed(Buttons::X) {
            self.mode = Mode::Browse;
            // Back where it was (as many as are still in hand).
            if let Some(origin) = held.origin.map(|o| Placed { count: held.count, ..o }).filter(|o| bag.fits(o)) {
                bag.put(origin);
                self.cursor = (origin.x, origin.y);
                self.notice = None;
                return Outcome::Stay;
            }
            self.notice = Some(if held.kind.can_put_down() { "LEFT IT ON THE GROUND" } else { held.kind.discard_words().1 });
            return Outcome::Drop(held.kind, held.count);
        }
        Outcome::Stay
    }

    /// The cursor step this frame: on press, then repeating while held.
    fn direction(&mut self, input: &InputState) -> Option<(i32, i32)> {
        let dirs = [(Buttons::UP, (0, -1)), (Buttons::DOWN, (0, 1)), (Buttons::LEFT, (-1, 0)), (Buttons::RIGHT, (1, 0))];
        if let Some(&(_, d)) = dirs.iter().find(|(b, _)| input.pressed(*b)) {
            self.held_frames = 0;
            return Some(d);
        }
        let held = dirs.iter().find(|(b, _)| input.held(*b));
        match held {
            Some(&(_, d)) => {
                self.held_frames += 1;
                let repeat = self.held_frames >= REPEAT_DELAY && (self.held_frames - REPEAT_DELAY).is_multiple_of(REPEAT_EVERY);
                repeat.then_some(d)
            }
            None => {
                self.held_frames = 0;
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Wearing a tool belt with an axe hung on it and a digging stick in
    /// the bag.
    fn belted() -> Inventory {
        let mut inv = Inventory::default();
        for k in [ItemKind::ToolBelt, ItemKind::StoneAxe, ItemKind::DiggingStick] {
            inv.bag.stow_one(k);
        }
        let at = |inv: &Inventory, k| inv.bag.items().iter().position(|p| p.kind == k).unwrap();
        inv.wear(at(&inv, ItemKind::ToolBelt)).unwrap();
        inv.hang(at(&inv, ItemKind::StoneAxe)).unwrap();
        inv
    }

    /// Presses A on `menu` for a frame.
    fn press_a(menu: &mut BagMenu, inv: &mut Inventory) {
        let mut input = InputState::default();
        input.update(Buttons::A);
        menu.update(&input, inv);
    }

    /// The menu open on a recipe made at the lathe, with its materials and
    /// tools carried.
    fn at_a_lathe_recipe() -> (BagMenu, Inventory) {
        let index = RECIPES.iter().position(|r| r.at_lathe).expect("a recipe made at the lathe");
        let mut inv = Inventory::default();
        for &(kind, n) in RECIPES[index].needs {
            inv.bag.stow(kind, n);
        }
        for &kind in RECIPES[index].tools {
            inv.bag.stow_one(kind);
        }
        let menu = BagMenu { mode: Mode::Recipe { recipe: index, scroll: 0, confirming: false }, ..BagMenu::browse() };
        (menu, inv)
    }

    #[test]
    fn a_recipe_made_at_the_lathe_needs_the_lathe() {
        let (mut menu, mut inv) = at_a_lathe_recipe();
        let before = inv.bag.clone();
        press_a(&mut menu, &mut inv);
        assert_eq!(menu.notice, Some("NEEDS A LATHE"));
        assert_eq!(inv.bag, before, "nothing is made");
    }

    #[test]
    fn a_recipe_made_at_the_lathe_is_made_beside_it() {
        let (mut menu, mut inv) = at_a_lathe_recipe();
        menu.near_lathe = true;
        let index = RECIPES.iter().position(|r| r.at_lathe).unwrap();
        press_a(&mut menu, &mut inv);
        press_a(&mut menu, &mut inv);
        assert!(inv.bag.has(RECIPES[index].makes));
    }

    /// The menu open on the handcart's recipe, its materials and tools
    /// carried.
    fn at_the_handcart() -> (BagMenu, Inventory) {
        let index = RECIPES.iter().position(|r| r.makes == ItemKind::Handcart).unwrap();
        let mut inv = Inventory::default();
        inv.bag.set_rows(super::super::bag::BASE_ROWS + 2);
        for &(kind, n) in RECIPES[index].needs {
            assert_eq!(inv.bag.stow(kind, n), 0);
        }
        for &kind in RECIPES[index].tools {
            inv.bag.stow_one(kind);
        }
        (BagMenu { mode: Mode::Recipe { recipe: index, scroll: 0, confirming: false }, ..BagMenu::browse() }, inv)
    }

    #[test]
    fn a_handcart_made_is_handed_over_not_put_in_the_bag() {
        let (mut menu, mut inv) = at_the_handcart();
        press_a(&mut menu, &mut inv);
        let mut input = InputState::default();
        input.update(Buttons::A);
        assert_eq!(menu.update(&input, &mut inv), Outcome::Made(ItemKind::Handcart));
        assert!(!inv.bag.has(ItemKind::Handcart));
        assert!(!inv.bag.has(ItemKind::SpokedWheel), "the materials are used up");
    }

    #[test]
    fn a_second_handcart_isnt_made() {
        let (mut menu, mut inv) = at_the_handcart();
        inv.cart = Some(Bag::with_size(8, 6));
        let before = inv.bag.clone();
        press_a(&mut menu, &mut inv);
        assert_eq!(inv.bag, before, "nothing is used up");
        assert!(menu.notice.is_some());
    }

    /// The menu open on the stone knife's recipe, with two stones carried.
    fn at_the_knife() -> (BagMenu, Inventory) {
        let index = RECIPES.iter().position(|r| r.makes == ItemKind::StoneKnife).unwrap();
        let mut inv = Inventory::default();
        inv.bag.stow(ItemKind::Stone, 2);
        (BagMenu { mode: Mode::Recipe { recipe: index, scroll: 0, confirming: false }, ..BagMenu::browse() }, inv)
    }

    #[test]
    fn a_on_a_recipe_asks_before_making_and_a_again_makes_it() {
        let (mut menu, mut inv) = at_the_knife();
        let before = inv.bag.clone();
        press(&mut menu, &mut inv, Buttons::A);
        assert_eq!(inv.bag, before, "nothing is made yet");
        assert!(matches!(menu.mode, Mode::Recipe { confirming: true, .. }));
        press(&mut menu, &mut inv, Buttons::A);
        assert!(inv.bag.has(ItemKind::StoneKnife));
        assert!(matches!(menu.mode, Mode::Recipe { confirming: false, .. }), "the recipe stays open to make another");
    }

    #[test]
    fn b_while_asking_makes_nothing_and_stays_on_the_recipe() {
        let (mut menu, mut inv) = at_the_knife();
        let before = inv.bag.clone();
        press(&mut menu, &mut inv, Buttons::A);
        press(&mut menu, &mut inv, Buttons::B);
        assert_eq!(inv.bag, before);
        assert!(matches!(menu.mode, Mode::Recipe { confirming: false, .. }));
    }

    #[test]
    fn b_on_a_recipe_goes_back_to_the_list() {
        let (mut menu, mut inv) = at_the_knife();
        press(&mut menu, &mut inv, Buttons::B);
        assert!(matches!(menu.mode, Mode::Crafting { .. }));
    }

    #[test]
    fn a_on_a_recipe_that_cant_be_made_says_why_without_asking() {
        let (mut menu, mut inv) = at_the_knife();
        inv.bag = Bag::default();
        press(&mut menu, &mut inv, Buttons::A);
        assert_eq!(menu.notice, Some("NOT ENOUGH MATERIALS"));
        assert!(matches!(menu.mode, Mode::Recipe { confirming: false, .. }));
    }

    #[test]
    fn what_a_recipe_needs_lists_the_materials_then_the_tools_and_the_place() {
        let lathe = RECIPES.iter().find(|r| r.at_lathe && !r.tools.is_empty()).unwrap();
        let lines = need_lines(lathe);
        assert_eq!(lines[0], NeedLine::Materials);
        let tools = lines.iter().position(|l| *l == NeedLine::Tools).expect("a tools heading");
        assert!(lines[..tools].iter().all(|l| !matches!(l, NeedLine::Tool(_) | NeedLine::Lathe | NeedLine::Fire)));
        assert!(lines[tools..].iter().all(|l| !matches!(l, NeedLine::Material(..))));
        assert!(lines[tools..].contains(&NeedLine::Lathe), "the lathe goes with the tools");
    }

    #[test]
    fn a_recipe_without_tools_has_no_tools_heading() {
        let knife = RECIPES.iter().find(|r| r.makes == ItemKind::StoneKnife).unwrap();
        assert!(!need_lines(knife).contains(&NeedLine::Tools));
    }

    #[test]
    fn every_recipe_fits_its_needs_on_the_page_without_scrolling() {
        // Not a rule, just how things stand: scrolling is for later.
        assert!(RECIPES.iter().all(|r| need_lines(r).len() <= NEEDS_SHOWN));
    }

    #[test]
    fn up_and_down_scroll_the_needs_only_as_far_as_they_go() {
        let long = NEEDS_SHOWN + 3;
        assert_eq!(scrolled(0, -1, long), 0);
        assert_eq!(scrolled(0, 1, long), 1);
        assert_eq!(scrolled(3, 1, long), 3, "stops with the last line at the bottom");
        assert_eq!(scrolled(0, 1, NEEDS_SHOWN), 0, "nothing to scroll when it all fits");
    }

    /// The cart page, with a stack of rope in the bag and the cart at hand.
    fn at_the_cart() -> (BagMenu, Inventory) {
        let mut inv = Inventory::default();
        inv.bag.stow(ItemKind::Rope, 3);
        inv.cart = Some(Bag::with_size(8, 6));
        let mut menu = BagMenu::browse();
        menu.cart_at_hand = true;
        menu.mode = Mode::Cart { in_cart: false };
        menu.cursor = (0, 0);
        (menu, inv)
    }

    #[test]
    fn the_cart_page_is_there_only_with_the_cart_at_hand() {
        let mut menu = BagMenu::browse();
        assert!(!menu.pages().contains(&Page::Cart));
        menu.cart_at_hand = true;
        assert!(menu.pages().contains(&Page::Cart));
    }

    #[test]
    fn on_the_cart_page_a_moves_a_stack_into_the_cart_and_back() {
        let (mut menu, mut inv) = at_the_cart();
        press_a(&mut menu, &mut inv);
        assert!(!inv.bag.has(ItemKind::Rope));
        assert_eq!(inv.cart.as_ref().unwrap().count(ItemKind::Rope), 3);
        menu.mode = Mode::Cart { in_cart: true };
        menu.cursor = (0, 0);
        let mut input = InputState::default();
        input.update(0);
        menu.update(&input, &mut inv);
        press_a(&mut menu, &mut inv);
        assert_eq!(inv.bag.count(ItemKind::Rope), 3);
    }

    #[test]
    fn what_doesnt_fit_the_other_side_stays_where_it_was() {
        let (mut menu, mut inv) = at_the_cart();
        let cart = inv.cart.as_mut().unwrap();
        while cart.stow_one(ItemKind::Stone) {}
        press_a(&mut menu, &mut inv);
        assert_eq!(inv.bag.count(ItemKind::Rope), 3);
        assert_eq!(menu.notice, Some("NO ROOM"));
    }

    #[test]
    fn the_belts_hooks_are_on_the_gear_page_only_while_it_is_worn() {
        assert!(!gear_rows(&Inventory::default().gear).iter().any(|r| matches!(r, GearRow::Hook(_))));
        let hooks = gear_rows(&belted().gear).into_iter().filter(|r| matches!(r, GearRow::Hook(_))).count();
        assert_eq!(hooks, ItemKind::ToolBelt.hooks());
    }

    #[test]
    fn a_tool_on_the_belt_can_be_taken_in_hand() {
        let choices = gear_choices(&belted(), GearRow::Slot(Slot::Hand));
        assert!(choices.contains(&GearChoice::Use(ItemKind::StoneAxe)));
        assert!(choices.contains(&GearChoice::Use(ItemKind::DiggingStick)));
    }

    #[test]
    fn a_free_hook_offers_the_tools_in_the_bag_that_hang() {
        let inv = belted();
        let free = (0..inv.gear.hooks()).find(|&h| inv.gear.belt[h].is_none()).unwrap();
        assert_eq!(gear_choices(&inv, GearRow::Hook(free)), vec![GearChoice::Use(ItemKind::DiggingStick)]);
    }

    #[test]
    fn hanging_is_offered_for_a_tool_only_while_a_hook_is_free() {
        let mut inv = belted();
        assert!(commands_for(ItemKind::DiggingStick, &inv.gear).contains(&Command::Hang));
        let at = inv.bag.items().iter().position(|p| p.kind == ItemKind::DiggingStick).unwrap();
        inv.hang(at).unwrap();
        inv.bag.stow_one(ItemKind::WoodSpear);
        assert!(!commands_for(ItemKind::WoodSpear, &inv.gear).contains(&Command::Hang));
    }

    /// Presses `buttons` for a frame of the bag screen, then lets go.
    fn press(menu: &mut BagMenu, inv: &mut Inventory, buttons: u16) -> Outcome {
        let mut input = InputState::default();
        input.update(buttons);
        let outcome = menu.update(&input, inv);
        input.update(0);
        menu.update(&input, inv);
        outcome
    }

    #[test]
    fn r_and_l_switch_to_the_next_and_previous_page() {
        let mut inv = Inventory::default();
        let mut menu = BagMenu::browse();
        press(&mut menu, &mut inv, Buttons::R);
        assert_eq!(menu.page(), Page::Craft);
        assert!(matches!(menu.mode, Mode::Crafting { .. }), "the page opens, not just its tab");
        press(&mut menu, &mut inv, Buttons::R);
        assert_eq!(menu.mode, Mode::Gear { slot: 0 });
        press(&mut menu, &mut inv, Buttons::L);
        press(&mut menu, &mut inv, Buttons::L);
        assert_eq!(menu.mode, Mode::Browse);
    }

    #[test]
    fn r_and_l_stop_at_the_ends_of_the_tabs() {
        let mut inv = Inventory::default();
        let mut menu = BagMenu::browse();
        press(&mut menu, &mut inv, Buttons::L);
        assert_eq!(menu.mode, Mode::Browse);
        for _ in 0..Page::ALL.len() {
            press(&mut menu, &mut inv, Buttons::R);
        }
        assert_eq!(menu.mode, Mode::Gear { slot: 0 }, "the last page without the cart at hand");
    }

    #[test]
    fn r_and_l_on_the_tab_row_move_along_it() {
        let mut inv = Inventory::default();
        let mut menu = BagMenu::browse();
        menu.mode = Mode::Tabs(Page::Items);
        press(&mut menu, &mut inv, Buttons::R);
        assert_eq!(menu.mode, Mode::Tabs(Page::Craft));
    }

    #[test]
    fn r_and_l_do_nothing_while_holding_an_item() {
        let mut inv = Inventory::default();
        let mut menu = BagMenu::holding(ItemKind::Stone, 1);
        press(&mut menu, &mut inv, Buttons::R);
        assert!(matches!(menu.mode, Mode::Holding(_)));
    }

    #[test]
    fn the_items_page_keeps_its_cursor_when_switched_back_to() {
        let mut inv = Inventory::default();
        let mut menu = BagMenu::browse();
        press(&mut menu, &mut inv, Buttons::RIGHT);
        press(&mut menu, &mut inv, Buttons::DOWN);
        press(&mut menu, &mut inv, Buttons::R);
        press(&mut menu, &mut inv, Buttons::L);
        assert_eq!(menu.cursor, (1, 1));
    }

    #[test]
    fn with_the_cart_at_hand_r_and_l_go_by_its_page_too() {
        let mut inv = Inventory { cart: Some(Bag::with_size(8, 6)), ..Inventory::default() };
        let mut menu = BagMenu::browse();
        menu.cart_at_hand = true;
        for _ in 0..3 {
            press(&mut menu, &mut inv, Buttons::R);
        }
        assert_eq!(menu.page(), Page::Cart, "after GEAR");
        press(&mut menu, &mut inv, Buttons::L);
        assert_eq!(menu.page(), Page::Gear);
    }

    #[test]
    fn without_the_cart_at_hand_r_and_l_pass_its_page_by() {
        let mut inv = Inventory::default();
        let mut menu = BagMenu { mode: Mode::Gear { slot: 0 }, ..BagMenu::browse() };
        press(&mut menu, &mut inv, Buttons::R);
        assert_eq!(menu.mode, Mode::Gear { slot: 0 });
    }

    /// Every recipe known, the list open with the cursor at `choice`.
    fn on_the_list(choice: usize) -> (BagMenu, Inventory) {
        let mut inv = Inventory::default();
        inv.recipes.learn_all();
        (BagMenu { mode: Mode::Crafting { choice }, ..BagMenu::browse() }, inv)
    }

    #[test]
    fn y_on_the_recipe_list_opens_the_sort_menu_at_the_order_in_use() {
        let (mut menu, mut inv) = on_the_list(3);
        inv.recipes.set_order(RecipeOrder::New);
        press(&mut menu, &mut inv, Buttons::Y);
        let pick = RecipeOrder::ALL.iter().position(|&o| o == RecipeOrder::New).unwrap();
        assert_eq!(menu.mode, Mode::Sorting { choice: 3, pick });
    }

    #[test]
    fn choosing_an_order_sorts_the_list_and_goes_to_its_top() {
        let (mut menu, mut inv) = on_the_list(3);
        press(&mut menu, &mut inv, Buttons::Y);
        press(&mut menu, &mut inv, Buttons::DOWN);
        press(&mut menu, &mut inv, Buttons::A);
        assert_eq!(inv.recipes.order(), RecipeOrder::ALL[1]);
        assert_eq!(menu.mode, Mode::Crafting { choice: 0 });
    }

    #[test]
    fn b_on_the_sort_menu_leaves_the_order_as_it_was() {
        let (mut menu, mut inv) = on_the_list(3);
        press(&mut menu, &mut inv, Buttons::Y);
        press(&mut menu, &mut inv, Buttons::DOWN);
        press(&mut menu, &mut inv, Buttons::B);
        assert_eq!(inv.recipes.order(), RecipeOrder::AToZ);
        assert_eq!(menu.mode, Mode::Crafting { choice: 3 });
    }

    #[test]
    fn the_list_opens_the_recipe_shown_at_the_cursor() {
        let (mut menu, mut inv) = on_the_list(0);
        press(&mut menu, &mut inv, Buttons::A);
        let Mode::Recipe { recipe, .. } = menu.mode else { panic!("{:?}", menu.mode) };
        assert_eq!(recipe, menu.recipe_list(&inv)[0]);
        assert_ne!(recipe, 0, "the first in A to Z isn't the first recipe");
    }

    #[test]
    fn going_back_from_a_recipe_puts_the_cursor_on_it_in_the_list() {
        let (mut menu, mut inv) = on_the_list(0);
        inv.recipes.set_order(RecipeOrder::CanMake);
        inv.bag.stow(ItemKind::Stone, 2);
        let knife = RECIPES.iter().position(|r| r.makes == ItemKind::StoneKnife).unwrap();
        menu.mode = Mode::Recipe { recipe: knife, scroll: 0, confirming: false };
        press(&mut menu, &mut inv, Buttons::A);
        press(&mut menu, &mut inv, Buttons::A);
        press(&mut menu, &mut inv, Buttons::B);
        let Mode::Crafting { choice } = menu.mode else { panic!("{:?}", menu.mode) };
        assert_eq!(menu.recipe_list(&inv)[choice], knife, "the stones are used up, so the knife has moved down");
    }
}

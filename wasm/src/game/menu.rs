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
//! no room left, leaves it on the ground). With a tool belt worn, its
//! hooks lie side by side below the grid: the cursor goes down onto them,
//! a tool there has the same commands as one in the bag, and a tool held
//! for placing is hung on one with A (swapping with a tool already there).
//! On the gear page, A on a slot (hand, back, waist) lists what can go
//! there and TAKE OFF, and A on one of those puts it there; B goes back to
//! the slots, and from them to the tabs. EQUIP on an item puts it in its
//! slot straight away, from the items page.

use super::bag::{BAG_W, Bag, Placed, footprint};
use super::mill::MillWorks;
use super::state::{Gear, HOOK_CELLS, Inventory};
use crate::common::input::{Buttons, InputState};
use crate::crafting::{RECIPES, RecipeOrder};
use crate::items::{ItemKind, MakeLimit, Slot, Use};

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
    /// Where it was in the bag, or `None` if it came off the ground (or
    /// a hook).
    pub origin: Option<Placed>,
    /// The belt's hook it was lifted off, if it was.
    pub hook: Option<usize>,
}

/// Where an item on the items page is: in the bag (index into its
/// items) or on a hook of the belt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Spot {
    Bag(usize),
    Hook(usize),
}

/// Whether `cursor` is on the row of the belt's hooks, below the bag's
/// grid (only there while a belt is worn).
pub fn on_belt(cursor: (i32, i32), bag: &Bag) -> bool {
    cursor.1 >= bag.rows()
}

/// The hook under a cursor on the belt's row.
pub fn hook_at(x: i32) -> usize {
    (x / HOOK_CELLS) as usize
}

/// Whether a tool held to be placed on the belt's row lies turned: it
/// lies along the hook, on its long side.
pub fn lies_on_hook(kind: ItemKind) -> bool {
    let (w, h) = kind.size();
    h > w
}

/// The item at `cursor` on the items page, and where it is.
pub fn item_at(inv: &Inventory, cursor: (i32, i32)) -> Option<(Spot, ItemKind)> {
    if on_belt(cursor, &inv.bag) {
        let h = hook_at(cursor.0);
        return inv.gear.belt.get(h).copied().flatten().filter(|_| h < inv.gear.hooks()).map(|k| (Spot::Hook(h), k));
    }
    inv.bag.at(cursor.0, cursor.1).map(|i| (Spot::Bag(i), inv.bag.items()[i].kind))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    Use(Use),
    Move,
    /// Put it in its slot (hand, back, waist) there and then.
    Equip,
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
            Command::Equip => "EQUIP",
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
    /// Leaving materials at the mill's site and building its next stage
    /// (only near its marked-out ground, while there's a stage to build).
    Build,
}

impl Page {
    pub const ALL: [Page; 5] = [Page::Items, Page::Craft, Page::Gear, Page::Cart, Page::Build];

    pub fn label(self) -> &'static str {
        match self {
            Page::Items => "ITEMS",
            Page::Craft => "CRAFT",
            Page::Gear => "GEAR",
            Page::Cart => "CART",
            Page::Build => "BUILD",
        }
    }
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

/// What can go in a slot on the gear page: each kind carried that goes
/// there (but the one already there), then taking off what's there. A
/// tool to hold may come from the bag or the belt; anything else, only
/// from the bag.
pub fn gear_choices(inv: &Inventory, slot: Slot) -> Vec<GearChoice> {
    let there = inv.gear.worn(slot);
    let from_belt = inv.gear.belt.iter().flatten().copied().filter(|_| slot.keeps_in_bag());
    let mut out: Vec<GearChoice> = Vec::new();
    for kind in inv.bag.items().iter().map(|p| p.kind).chain(from_belt) {
        let choice = GearChoice::Use(kind);
        if slot.holds(kind) && Some(kind) != there && !out.contains(&choice) {
            out.push(choice);
        }
    }
    if there.is_some() {
        out.push(GearChoice::TakeOff);
    }
    out
}

/// Where EQUIP puts `kind`: its slot, unless it's there already.
pub fn equip_slot(gear: &Gear, kind: ItemKind) -> Option<Slot> {
    kind.slot().filter(|&s| gear.worn(s) != Some(kind))
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
        item: Spot,
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
    /// On a slot of the gear page (index into `Slot::ALL`).
    Gear {
        place: usize,
    },
    /// Choosing from `gear_choices` for a slot.
    GearPick {
        place: usize,
        choice: usize,
    },
    /// On the cart page, the cursor (`BagMenu::cursor`) on a cell of the
    /// bag or (`in_cart`) of the cart's load.
    Cart {
        in_cart: bool,
    },
    /// On the build page, on a line of the next stage's materials (`row`,
    /// from 0) or, after them, on BUILD; `confirming` while asking whether
    /// to build it.
    Build {
        row: usize,
        confirming: bool,
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
    /// As many of a kind as the mill's next stage still takes are to be
    /// left at its site (from the bag, then the cart).
    Deliver(ItemKind),
    /// The mill's next stage is to be built: everything it takes is in,
    /// and the tools are carried. The bag closes.
    Build,
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
    /// The castaway is fit to see the mill's next stage through (its hours
    /// of work wouldn't leave them close to collapse).
    pub fit_to_build: bool,
    /// How far the mill has got, while the BUILD screen is open at the
    /// bench beside its ground (the build page is there only then, on its
    /// own, not among the bag's pages).
    pub works: Option<MillWorks>,
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

/// Why `recipe` can't be made here and now, if it can't. A limit on how
/// many may be made comes first: with one made already, gathering more
/// materials wouldn't help.
fn why_not(menu: &BagMenu, inv: &Inventory, recipe: &crate::crafting::Recipe) -> Option<&'static str> {
    match recipe.makes.make_limit() {
        MakeLimit::Any => {}
        MakeLimit::OneAtATime => {
            if inv.cart.is_some() {
                return Some("YOU HAVE ONE ALREADY");
            }
        }
        MakeLimit::UpTo(n) => {
            if inv.recipes.times_made(recipe.makes) >= n {
                return Some("NOT NEEDED NOW");
            }
        }
    }
    if !inv.can_make(recipe) {
        return Some(if recipe.tools.iter().all(|&t| inv.carries(t)) { "NOT ENOUGH MATERIALS" } else { "MISSING A TOOL" });
    }
    if recipe.at_fire && !menu.near_fire {
        return Some("NEEDS A FIRE");
    }
    if recipe.at_lathe && !menu.near_lathe {
        return Some("NEEDS A LATHE");
    }
    None
}

/// Commands offered for an item, in menu order: using it, then EQUIP
/// (gear not in its slot already: it's put there straight away).
pub fn commands_for(kind: ItemKind, gear: &Gear) -> Vec<Command> {
    let equip = equip_slot(gear, kind).map(|_| Command::Equip);
    kind.usage().map(Command::Use).into_iter().chain(equip).chain([Command::Move, Command::Drop]).collect()
}

impl BagMenu {
    /// Whether this is the place to make `recipe`: by a fire, or by the
    /// lathe, if it must be made there.
    pub fn in_place_for(&self, recipe: &crate::crafting::Recipe) -> bool {
        (!recipe.at_fire || self.near_fire) && (!recipe.at_lathe || self.near_lathe)
    }

    /// Whether `recipe` can be made here and now (it's lit on the list):
    /// nothing stands in the way, not even a limit on how many.
    pub fn can_make_now(&self, inv: &Inventory, recipe: &crate::crafting::Recipe) -> bool {
        why_not(self, inv, recipe).is_none()
    }

    /// The recipes on the CRAFT list, top to bottom, sorted in the order
    /// chosen (what can be made counts the fire, the lathe and any limit).
    pub fn recipe_list(&self, inv: &Inventory) -> Vec<usize> {
        inv.recipes.sorted(|w| self.can_make_now(inv, &RECIPES[w]))
    }

    /// The pages to choose from: the cart's only with it at hand, the
    /// build page only by the mill's site.
    pub fn pages(&self) -> Vec<Page> {
        Page::ALL
            .into_iter()
            .filter(|&p| match p {
                _ if self.works.is_some() => p == Page::Build,
                Page::Cart => self.cart_at_hand,
                Page::Build => false,
                _ => true,
            })
            .collect()
    }

    pub fn browse() -> Self {
        Self {
            cursor: (0, 0),
            mode: Mode::Browse,
            notice: None,
            near_fire: false,
            near_lathe: false,
            cart_at_hand: false,
            fit_to_build: true,
            works: None,
            held_frames: 0,
        }
    }

    /// Opens the BUILD screen (at the bench beside the mill's ground), on
    /// its first line, with how far the mill has got.
    pub fn building(works: MillWorks) -> Self {
        Self { mode: Mode::Build { row: 0, confirming: false }, works: Some(works), ..Self::browse() }
    }

    /// How many of `kind` are at hand to leave at the mill's site: in the
    /// bag, and in the cart while it's at hand.
    pub fn at_hand(&self, inv: &Inventory, kind: ItemKind) -> u32 {
        inv.bag.count(kind) + inv.cart.as_ref().filter(|_| self.cart_at_hand).map_or(0, |c| c.count(kind))
    }

    /// Opens holding something just picked up (`count` of a kind) that
    /// didn't fit anywhere.
    pub fn holding(kind: ItemKind, count: u32) -> Self {
        let held = HeldItem { kind, turned: false, count, origin: None, hook: None };
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
            Mode::Build { .. } => Page::Build,
            _ => Page::Items,
        }
    }

    fn open(&mut self, page: Page) {
        self.notice = None;
        self.mode = match page {
            Page::Items => Mode::Browse,
            Page::Craft => Mode::Crafting { choice: 0 },
            Page::Gear => Mode::Gear { place: 0 },
            Page::Cart => {
                self.cursor = (0, 0);
                Mode::Cart { in_cart: false }
            }
            Page::Build => Mode::Build { row: 0, confirming: false },
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
                    // The belt's row is below the grid while one is worn.
                    let belt = inv.gear.hooks() > 0;
                    let last_row = inv.bag.rows() - if belt { 0 } else { 1 };
                    let y = (self.cursor.1 + dy).clamp(0, last_row);
                    let width = if y >= inv.bag.rows() { inv.gear.hooks() as i32 * HOOK_CELLS } else { BAG_W };
                    self.cursor = ((self.cursor.0 + dx).clamp(0, width - 1), y);
                }
                if input.pressed(Buttons::B) {
                    return Outcome::Close;
                }
                if input.pressed(Buttons::A)
                    && let Some((item, _)) = item_at(inv, self.cursor)
                {
                    self.mode = Mode::Commands { item, choice: 0 };
                    self.notice = None;
                }
                Outcome::Stay
            }
            Mode::Commands { item, choice } => {
                let kind = match item {
                    Spot::Bag(i) => inv.bag.items()[i].kind,
                    Spot::Hook(h) => inv.gear.belt[h].expect("a tool on the hook"),
                };
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
                // Takes it out of where it is: one of a stack in the bag,
                // or all of it.
                let take = |inv: &mut Inventory, whole: bool| -> Option<Placed> {
                    match item {
                        Spot::Bag(i) if whole => Some(inv.bag.remove(i)),
                        Spot::Bag(i) => {
                            inv.bag.take_one(i);
                            None
                        }
                        Spot::Hook(h) => {
                            inv.gear.belt[h] = None;
                            None
                        }
                    }
                };
                match commands[choice] {
                    Command::Use(how) => {
                        if how.uses_up() {
                            take(inv, false);
                        }
                        Outcome::Use(kind)
                    }
                    Command::Equip => {
                        if let Some(slot) = equip_slot(&inv.gear, kind) {
                            self.notice = Some(inv.put_in(slot, kind).err().unwrap_or(slot.done_words()));
                            // A pack swapped may change the bag's rows.
                            self.cursor = (self.cursor.0, self.cursor.1.min(inv.bag.rows() - 1 + (inv.gear.hooks() > 0) as i32));
                        }
                        Outcome::Stay
                    }
                    Command::Move => {
                        let p = take(inv, true);
                        let hook = match item {
                            Spot::Hook(h) => Some(h),
                            Spot::Bag(_) => None,
                        };
                        let turned = p.map_or(lies_on_hook(kind), |p| p.turned);
                        if let Some(p) = p {
                            self.cursor = (p.x, p.y);
                        }
                        self.mode = Mode::Holding(HeldItem { kind, turned, count: p.map_or(1, |p| p.count), origin: p, hook });
                        Outcome::Stay
                    }
                    Command::Drop => {
                        let count = match item {
                            Spot::Bag(i) => inv.bag.remove(i).count,
                            Spot::Hook(_) => {
                                take(inv, true);
                                1
                            }
                        };
                        self.notice = Some(kind.discard_words().1);
                        Outcome::Drop(kind, count)
                    }
                }
            }
            Mode::Holding(held) => self.update_holding(input, inv, held, step),
            Mode::Cart { in_cart } => self.update_cart(input, inv, in_cart, step),
            Mode::Build { row, confirming } => self.update_build(input, inv, row, confirming, step),
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
            Mode::Gear { place } => {
                let places = Slot::ALL;
                if let Some((_, dy)) = step.filter(|s| s.1 != 0) {
                    if dy < 0 && place == 0 {
                        self.mode = Mode::Tabs(Page::Gear);
                        return Outcome::Stay;
                    }
                    self.mode = Mode::Gear { place: (place as i32 + dy).clamp(0, places.len() as i32 - 1) as usize };
                    self.notice = None;
                }
                if input.pressed(Buttons::B) {
                    self.mode = Mode::Tabs(Page::Gear);
                    self.notice = None;
                } else if input.pressed(Buttons::A) {
                    if gear_choices(inv, places[place]).is_empty() {
                        self.notice = Some("NOTHING TO PUT HERE");
                    } else {
                        self.mode = Mode::GearPick { place, choice: 0 };
                        self.notice = None;
                    }
                }
                Outcome::Stay
            }
            Mode::GearPick { place, choice } => {
                let at = Slot::ALL[place];
                let choices = gear_choices(inv, at);
                if let Some((_, dy)) = step.filter(|s| s.1 != 0) {
                    self.mode = Mode::GearPick { place, choice: (choice as i32 + dy).rem_euclid(choices.len() as i32) as usize };
                }
                if input.pressed(Buttons::B) {
                    self.mode = Mode::Gear { place };
                } else if input.pressed(Buttons::A) {
                    let (result, done) = match choices[choice] {
                        GearChoice::Use(kind) => (inv.put_in(at, kind), at.done_words()),
                        GearChoice::TakeOff => (inv.empty(at), "TOOK IT OFF"),
                    };
                    self.notice = Some(result.err().unwrap_or(done));
                    self.mode = Mode::Gear { place };
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
            *self = Self {
                near_fire: self.near_fire,
                near_lathe: self.near_lathe,
                cart_at_hand: self.cart_at_hand,
                works: self.works.take(),
                ..Self::holding(recipe.makes, 1)
            };
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

    /// The BUILD screen: up and down go over the next stage's materials
    /// and on to BUILD; A on a material asks whether to put what's at hand
    /// of it in at the site (A again puts it in), and A on BUILD asks
    /// whether to build it (A again builds it); B says no to either.
    /// Otherwise B closes it.
    fn update_build(&mut self, input: &InputState, inv: &Inventory, row: usize, confirming: bool, step: Option<(i32, i32)>) -> Outcome {
        let Some(stage) = self.works.as_ref().and_then(MillWorks::next) else {
            return Outcome::Close;
        };
        let lines = stage.needs.len() + 1;
        if confirming {
            if input.pressed(Buttons::A) {
                self.mode = Mode::Build { row, confirming: false };
                return match stage.needs.get(row) {
                    Some(&(kind, _)) => Outcome::Deliver(kind),
                    None => Outcome::Build,
                };
            }
            if input.pressed(Buttons::B) {
                self.mode = Mode::Build { row, confirming: false };
                self.notice = None;
            }
            return Outcome::Stay;
        }
        if let Some((_, dy)) = step.filter(|s| s.1 != 0) {
            self.mode = Mode::Build { row: (row as i32 + dy).clamp(0, lines as i32 - 1) as usize, confirming: false };
            self.notice = None;
            return Outcome::Stay;
        }
        if input.pressed(Buttons::B) {
            return Outcome::Close;
        }
        if !input.pressed(Buttons::A) {
            return Outcome::Stay;
        }
        // A material: asked whether to put in what would go (none to put
        // in says so straight away).
        if let Some(&(kind, _)) = stage.needs.get(row) {
            match self.why_not_put_in(inv, kind) {
                Some(why) => self.notice = Some(why),
                None => {
                    self.mode = Mode::Build { row, confirming: true };
                    self.notice = None;
                }
            }
            return Outcome::Stay;
        }
        match self.why_not_build(inv) {
            Some(why) => self.notice = Some(why),
            None => {
                self.mode = Mode::Build { row, confirming: true };
                self.notice = None;
            }
        }
        Outcome::Stay
    }

    /// How many of `kind` would be put in at the mill's site now: as many
    /// as are at hand, no more than the next stage still takes.
    pub fn to_put_in(&self, inv: &Inventory, kind: ItemKind) -> u32 {
        self.works.as_ref().map_or(0, |w| w.still_needs(kind).min(self.at_hand(inv, kind)))
    }

    /// Why none of `kind` can be put in at the mill's site now, if none
    /// can.
    fn why_not_put_in(&self, inv: &Inventory, kind: ItemKind) -> Option<&'static str> {
        let works = self.works.as_ref()?;
        if works.still_needs(kind) == 0 {
            return Some("THAT'S ALL IT TAKES");
        }
        (self.at_hand(inv, kind) == 0).then_some("NONE AT HAND")
    }

    /// Why the mill's next stage can't be built now, if it can't.
    pub fn why_not_build(&self, inv: &Inventory) -> Option<&'static str> {
        let works = self.works.as_ref()?;
        let stage = works.next()?;
        if !works.all_in() {
            return Some("NOT ENOUGH MATERIALS");
        }
        if !stage.tools.iter().all(|&t| inv.carries(t)) {
            return Some("MISSING A TOOL");
        }
        (!self.fit_to_build).then_some("TOO WEAK TO BUILD")
    }

    fn update_holding(&mut self, input: &InputState, inv: &mut Inventory, mut held: HeldItem, step: Option<(i32, i32)>) -> Outcome {
        if input.pressed(Buttons::Y) && held.kind.size().0 != held.kind.size().1 {
            held.turned = !held.turned;
            self.mode = Mode::Holding(held);
        }
        let rows = inv.bag.rows();
        let (dx, dy) = step.unwrap_or((0, 0));
        if on_belt(self.cursor, &inv.bag) {
            // Along the hooks a hook at a time; up, back into the grid.
            let hooks = inv.gear.hooks() as i32;
            if dy < 0 {
                let (w, h) = footprint(held.kind, held.turned);
                self.cursor = (self.cursor.0.min(BAG_W - w), rows - h);
            } else {
                let hook = (hook_at(self.cursor.0) as i32 + dx).clamp(0, hooks - 1);
                self.cursor = (hook * HOOK_CELLS, rows);
            }
        } else {
            let (w, h) = footprint(held.kind, held.turned);
            let at_bottom = self.cursor.1 + h >= rows;
            if dy > 0 && at_bottom && inv.gear.hooks() > 0 {
                let hook = hook_at(self.cursor.0).min(inv.gear.hooks() - 1) as i32;
                self.cursor = (hook * HOOK_CELLS, rows);
            } else {
                self.cursor = ((self.cursor.0 + dx).clamp(0, BAG_W - w), (self.cursor.1 + dy).clamp(0, rows - h));
            }
        }

        if input.pressed(Buttons::A) {
            if on_belt(self.cursor, &inv.bag) {
                // Onto a hook: only a tool that hangs; one already there
                // is picked up in its place.
                if held.kind.hangs_on_belt() && held.count == 1 {
                    let hook = hook_at(self.cursor.0);
                    match inv.gear.belt[hook].replace(held.kind) {
                        Some(old) => self.mode = Mode::Holding(HeldItem { kind: old, turned: lies_on_hook(old), count: 1, origin: None, hook: Some(hook) }),
                        None => self.mode = Mode::Browse,
                    }
                    self.notice = None;
                }
                return Outcome::Stay;
            }
            let bag = &mut inv.bag;
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
                    let next = HeldItem { kind: lifted.kind, turned: lifted.turned, count: lifted.count, origin: Some(lifted), hook: None };
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
            // Back where it was: on its hook, if that's still free...
            if let Some(hook) = held.hook.filter(|&h| h < inv.gear.hooks() && inv.gear.belt[h].is_none()) {
                inv.gear.belt[hook] = Some(held.kind);
                self.cursor = (hook as i32 * HOOK_CELLS, rows);
                self.notice = None;
                return Outcome::Stay;
            }
            // ...or in the bag (as many as are still in hand).
            let bag = &mut inv.bag;
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
    use crate::game::bag::BASE_ROWS;
    use crate::game::state::HOOK_CELLS;
    use crate::items::Slot;

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

    /// The menu open on the first recipe for `kind`, beside a fire and the
    /// lathe, with its materials and tools carried.
    fn at_the_recipe_for(kind: ItemKind) -> (BagMenu, Inventory, usize) {
        let index = RECIPES.iter().position(|r| r.makes == kind).unwrap();
        let mut inv = Inventory::default();
        inv.bag.set_rows(BASE_ROWS + 2);
        for &(kind, n) in RECIPES[index].needs {
            assert_eq!(inv.bag.stow(kind, n), 0);
        }
        for &kind in RECIPES[index].tools {
            assert!(inv.bag.stow_one(kind));
        }
        let menu = BagMenu { mode: Mode::Recipe { recipe: index, scroll: 0, confirming: false }, near_fire: true, near_lathe: true, ..BagMenu::browse() };
        (menu, inv, index)
    }

    /// The things that may be made only so many times, with how many.
    fn limited() -> impl Iterator<Item = (ItemKind, u32)> {
        ItemKind::ALL.into_iter().filter_map(|k| match k.make_limit() {
            MakeLimit::UpTo(n) => Some((k, n)),
            _ => None,
        })
    }

    #[test]
    fn a_limited_thing_is_made_until_as_many_as_needed_have_been() {
        for (kind, n) in limited() {
            let (mut menu, mut inv, index) = at_the_recipe_for(kind);
            for _ in 1..n {
                inv.recipes.made(index);
            }
            press_a(&mut menu, &mut inv);
            assert!(matches!(menu.mode, Mode::Recipe { confirming: true, .. }), "{kind:?} asks to be made");
        }
    }

    #[test]
    fn a_limited_thing_isnt_made_once_as_many_as_needed_have_been() {
        for (kind, n) in limited() {
            let (mut menu, mut inv, index) = at_the_recipe_for(kind);
            for _ in 0..n {
                inv.recipes.made(index);
            }
            let before = inv.bag.clone();
            press_a(&mut menu, &mut inv);
            assert_eq!(menu.notice, Some("NOT NEEDED NOW"), "{kind:?}");
            assert_eq!(inv.bag, before, "nothing is used up for {kind:?}");
            assert!(!menu.can_make_now(&inv, &RECIPES[index]), "{kind:?} isn't lit on the list");
        }
    }

    #[test]
    fn a_thing_made_once_isnt_wanted_again_even_without_materials() {
        let index = RECIPES.iter().position(|r| r.makes == ItemKind::Ladder).unwrap();
        let mut inv = Inventory::default();
        inv.recipes.made(index);
        let mut menu = BagMenu { mode: Mode::Recipe { recipe: index, scroll: 0, confirming: false }, ..BagMenu::browse() };
        press_a(&mut menu, &mut inv);
        assert_eq!(menu.notice, Some("NOT NEEDED NOW"), "not a hint to gather more for it");
    }

    #[test]
    fn the_bridge_the_ladder_the_mill_parts_and_the_stakes_are_needed_only_once() {
        for kind in [ItemKind::LogBridge, ItemKind::Ladder, ItemKind::WaterWheel, ItemKind::WoodLathe, ItemKind::MarkingStakes] {
            assert_eq!(kind.make_limit(), MakeLimit::UpTo(1), "{kind:?}");
        }
    }

    #[test]
    fn spoked_wheels_are_needed_as_many_as_a_handcart_takes() {
        let cart = RECIPES.iter().find(|r| r.makes == ItemKind::Handcart).unwrap();
        let wheels = cart.needs.iter().find(|&&(k, _)| k == ItemKind::SpokedWheel).unwrap().1;
        assert_eq!(ItemKind::SpokedWheel.make_limit(), MakeLimit::UpTo(wheels));
    }

    #[test]
    fn things_of_everyday_use_can_be_made_again() {
        let (mut menu, mut inv, index) = at_the_recipe_for(ItemKind::Rope);
        inv.recipes.made(index);
        press_a(&mut menu, &mut inv);
        assert!(matches!(menu.mode, Mode::Recipe { confirming: true, .. }));
        assert!(menu.can_make_now(&inv, &RECIPES[index]));
    }

    #[test]
    fn a_second_handcart_isnt_lit_on_the_list() {
        let (menu, mut inv) = at_the_handcart();
        let index = RECIPES.iter().position(|r| r.makes == ItemKind::Handcart).unwrap();
        assert!(menu.can_make_now(&inv, &RECIPES[index]));
        inv.cart = Some(Bag::with_size(8, 6));
        assert!(!menu.can_make_now(&inv, &RECIPES[index]));
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
    fn the_build_screen_is_a_page_on_its_own_and_the_bag_has_none() {
        let mut menu = BagMenu::browse();
        menu.cart_at_hand = true;
        assert!(!menu.pages().contains(&Page::Build));
        let menu = BagMenu::building(MillWorks::default());
        assert_eq!(menu.pages(), vec![Page::Build]);
    }

    #[test]
    fn a_material_with_none_at_hand_says_so_without_asking() {
        let mut menu = BagMenu::building(MillWorks::default());
        let mut inv = Inventory::default();
        press_a(&mut menu, &mut inv);
        assert_eq!(menu.mode, Mode::Build { row: 0, confirming: false });
        assert_eq!(menu.notice, Some("NONE AT HAND"));
    }

    #[test]
    fn a_material_at_hand_is_asked_about_before_it_goes_in() {
        let mut menu = BagMenu::building(MillWorks::default());
        let mut inv = Inventory::default();
        inv.bag.stow(ItemKind::Stone, 3);
        press_a(&mut menu, &mut inv);
        assert_eq!(menu.mode, Mode::Build { row: 0, confirming: true });
        assert_eq!(menu.to_put_in(&inv, ItemKind::Stone), 3);
    }

    #[test]
    fn build_waits_for_every_material_then_the_tools() {
        let mut menu = BagMenu::building(MillWorks::default());
        let mut inv = Inventory::default();
        assert_eq!(menu.why_not_build(&inv), Some("NOT ENOUGH MATERIALS"));
        let works = menu.works.as_mut().unwrap();
        for &(kind, n) in works.next().unwrap().needs {
            works.deliver(kind, n);
        }
        assert_eq!(menu.why_not_build(&inv), Some("MISSING A TOOL"));
        for &tool in menu.works.as_ref().unwrap().next().unwrap().tools {
            inv.bag.stow_one(tool);
        }
        assert_eq!(menu.why_not_build(&inv), None);
    }

    #[test]
    fn with_everything_in_build_still_waits_for_the_strength_to_see_it_through() {
        let mut menu = BagMenu::building(MillWorks::default());
        let mut inv = Inventory::default();
        let works = menu.works.as_mut().unwrap();
        for &(kind, n) in works.next().unwrap().needs {
            works.deliver(kind, n);
        }
        for &tool in menu.works.as_ref().unwrap().next().unwrap().tools {
            inv.bag.stow_one(tool);
        }
        menu.fit_to_build = false;
        assert_eq!(menu.why_not_build(&inv), Some("TOO WEAK TO BUILD"));
        menu.fit_to_build = true;
        assert_eq!(menu.why_not_build(&inv), None);
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
    fn a_tool_on_the_belt_can_be_taken_in_hand() {
        let choices = gear_choices(&belted(), Slot::Hand);
        assert!(choices.contains(&GearChoice::Use(ItemKind::StoneAxe)));
        assert!(choices.contains(&GearChoice::Use(ItemKind::DiggingStick)));
    }

    #[test]
    fn a_choice_says_whether_it_is_in_the_bag_or_on_a_hook() {
        let inv = belted();
        assert_eq!(inv.hook_of(ItemKind::DiggingStick), None);
        assert_eq!(inv.hook_of(ItemKind::StoneAxe), Some(0));
    }

    #[test]
    fn the_gear_page_lists_only_the_slots_even_with_a_belt_worn() {
        let mut inv = belted();
        let mut menu = BagMenu { mode: Mode::Gear { place: 0 }, ..BagMenu::browse() };
        for _ in 0..6 {
            press(&mut menu, &mut inv, Buttons::DOWN);
        }
        assert_eq!(menu.mode, Mode::Gear { place: Slot::ALL.len() - 1 });
    }

    #[test]
    fn equip_is_offered_while_there_is_somewhere_to_put_it() {
        let mut inv = Inventory::default();
        assert!(commands_for(ItemKind::StoneAxe, &inv.gear).contains(&Command::Equip));
        assert!(commands_for(ItemKind::PackFrame, &inv.gear).contains(&Command::Equip));
        assert!(!commands_for(ItemKind::Stone, &inv.gear).contains(&Command::Equip));
        inv.gear.hand = Some(ItemKind::StoneAxe);
        assert!(!commands_for(ItemKind::StoneAxe, &inv.gear).contains(&Command::Equip));
    }

    /// The menu with the commands open on the bag's `kind` and EQUIP chosen.
    fn on_equip(inv: &Inventory, kind: ItemKind) -> BagMenu {
        let item = inv.bag.items().iter().position(|p| p.kind == kind).unwrap();
        let choice = commands_for(kind, &inv.gear).iter().position(|&c| c == Command::Equip).unwrap();
        BagMenu { mode: Mode::Commands { item: Spot::Bag(item), choice }, ..BagMenu::browse() }
    }

    #[test]
    fn equip_holds_a_tool_right_there_on_the_items_page() {
        let mut inv = belted();
        let mut menu = on_equip(&inv, ItemKind::DiggingStick);
        press_a(&mut menu, &mut inv);
        assert_eq!(inv.gear.hand, Some(ItemKind::DiggingStick));
        assert_eq!(menu.mode, Mode::Browse);
        assert_eq!(menu.notice, Some("TOOK IT IN HAND"));
    }

    #[test]
    fn equip_puts_on_a_pack_right_there_on_the_items_page() {
        let mut inv = Inventory::default();
        inv.bag.stow_one(ItemKind::PackFrame);
        let mut menu = on_equip(&inv, ItemKind::PackFrame);
        press_a(&mut menu, &mut inv);
        assert_eq!(inv.gear.back, Some(ItemKind::PackFrame));
        assert_eq!(menu.mode, Mode::Browse);
        assert_eq!(menu.notice, Some("PUT IT ON"));
    }

    #[test]
    fn equip_takes_a_tool_on_the_belt_in_hand() {
        let (mut menu, mut inv) = browsing((0, 0));
        let choice = commands_for(ItemKind::StoneAxe, &inv.gear).iter().position(|&c| c == Command::Equip).unwrap();
        menu.mode = Mode::Commands { item: Spot::Hook(0), choice };
        press(&mut menu, &mut inv, Buttons::A);
        assert_eq!(inv.gear.hand, Some(ItemKind::StoneAxe));
        assert_eq!(inv.gear.belt[0], Some(ItemKind::StoneAxe));
    }

    /// `belted()` browsing the items page, the cursor at `at`.
    fn browsing(at: (i32, i32)) -> (BagMenu, Inventory) {
        (BagMenu { cursor: at, ..BagMenu::browse() }, belted())
    }

    #[test]
    fn below_the_bag_the_cursor_reaches_the_belt_only_while_it_is_worn() {
        let (mut menu, mut inv) = browsing((0, BASE_ROWS - 1));
        press(&mut menu, &mut inv, Buttons::DOWN);
        assert_eq!(menu.cursor, (0, BASE_ROWS));
        let mut inv = Inventory::default();
        let mut menu = BagMenu { cursor: (0, BASE_ROWS - 1), ..BagMenu::browse() };
        press(&mut menu, &mut inv, Buttons::DOWN);
        assert_eq!(menu.cursor, (0, BASE_ROWS - 1));
    }

    #[test]
    fn a_tool_on_the_belt_has_the_commands_of_one_in_the_bag() {
        let (mut menu, mut inv) = browsing((1, BASE_ROWS));
        press(&mut menu, &mut inv, Buttons::A);
        assert!(matches!(menu.mode, Mode::Commands { item: Spot::Hook(0), .. }));
        let commands = commands_for(ItemKind::StoneAxe, &inv.gear);
        assert!(commands.contains(&Command::Equip) && commands.contains(&Command::Move) && commands.contains(&Command::Drop));
    }

    /// Lifts the bag's `kind` to move it (MOVE from its commands).
    fn lift(menu: &mut BagMenu, inv: &mut Inventory, kind: ItemKind) {
        let item = inv.bag.items().iter().position(|p| p.kind == kind).unwrap();
        let choice = commands_for(kind, &inv.gear).iter().position(|&c| c == Command::Move).unwrap();
        menu.mode = Mode::Commands { item: Spot::Bag(item), choice };
        press(menu, inv, Buttons::A);
        assert!(matches!(menu.mode, Mode::Holding(_)));
    }

    /// Presses down until the held item is over the belt, then right to
    /// hook `hook`.
    fn down_to_hook(menu: &mut BagMenu, inv: &mut Inventory, hook: usize) {
        for _ in 0..8 {
            press(menu, inv, Buttons::DOWN);
        }
        for _ in 0..hook * HOOK_CELLS as usize {
            press(menu, inv, Buttons::RIGHT);
        }
    }

    #[test]
    fn a_tool_moved_onto_a_free_hook_hangs_there_out_of_the_bag() {
        let (mut menu, mut inv) = browsing((0, 0));
        lift(&mut menu, &mut inv, ItemKind::DiggingStick);
        down_to_hook(&mut menu, &mut inv, 1);
        press(&mut menu, &mut inv, Buttons::A);
        assert_eq!(inv.gear.belt[1], Some(ItemKind::DiggingStick));
        assert!(!inv.bag.has(ItemKind::DiggingStick));
        assert_eq!(menu.mode, Mode::Browse);
    }

    #[test]
    fn what_does_not_hang_is_not_put_on_a_hook() {
        let (mut menu, mut inv) = browsing((0, 0));
        inv.bag.stow_one(ItemKind::Stone);
        lift(&mut menu, &mut inv, ItemKind::Stone);
        down_to_hook(&mut menu, &mut inv, 1);
        press(&mut menu, &mut inv, Buttons::A);
        assert_eq!(inv.gear.belt[1], None);
        assert!(matches!(menu.mode, Mode::Holding(h) if h.kind == ItemKind::Stone));
    }

    #[test]
    fn a_tool_put_on_a_taken_hook_swaps_with_the_one_there() {
        let (mut menu, mut inv) = browsing((0, 0));
        lift(&mut menu, &mut inv, ItemKind::DiggingStick);
        down_to_hook(&mut menu, &mut inv, 0);
        press(&mut menu, &mut inv, Buttons::A);
        assert_eq!(inv.gear.belt[0], Some(ItemKind::DiggingStick));
        assert!(matches!(menu.mode, Mode::Holding(h) if h.kind == ItemKind::StoneAxe));
    }

    /// Lifts the tool on hook `hook` (MOVE from its commands).
    fn lift_off_hook(menu: &mut BagMenu, inv: &mut Inventory, hook: usize) {
        let kind = inv.gear.belt[hook].unwrap();
        let choice = commands_for(kind, &inv.gear).iter().position(|&c| c == Command::Move).unwrap();
        menu.mode = Mode::Commands { item: Spot::Hook(hook), choice };
        press(menu, inv, Buttons::A);
    }

    #[test]
    fn a_tool_moved_off_the_belt_into_the_bag_leaves_its_hook() {
        let (mut menu, mut inv) = browsing((0, 0));
        lift_off_hook(&mut menu, &mut inv, 0);
        assert_eq!(inv.gear.belt[0], None);
        assert!(matches!(menu.mode, Mode::Holding(h) if h.kind == ItemKind::StoneAxe));
        for _ in 0..8 {
            press(&mut menu, &mut inv, Buttons::UP);
        }
        press(&mut menu, &mut inv, Buttons::RIGHT);
        press(&mut menu, &mut inv, Buttons::RIGHT);
        press(&mut menu, &mut inv, Buttons::RIGHT);
        press(&mut menu, &mut inv, Buttons::A);
        assert!(inv.bag.has(ItemKind::StoneAxe), "{:?}", menu.notice);
        assert_eq!(menu.mode, Mode::Browse);
    }

    #[test]
    fn b_puts_a_tool_lifted_off_the_belt_back_on_its_hook() {
        let (mut menu, mut inv) = browsing((0, 0));
        lift_off_hook(&mut menu, &mut inv, 0);
        press(&mut menu, &mut inv, Buttons::UP);
        press(&mut menu, &mut inv, Buttons::B);
        assert_eq!(inv.gear.belt[0], Some(ItemKind::StoneAxe));
        assert_eq!(menu.mode, Mode::Browse);
    }

    #[test]
    fn a_tool_on_the_belt_can_be_put_down() {
        let (mut menu, mut inv) = browsing((0, 0));
        let choice = commands_for(ItemKind::StoneAxe, &inv.gear).iter().position(|&c| c == Command::Drop).unwrap();
        menu.mode = Mode::Commands { item: Spot::Hook(0), choice };
        assert_eq!(press(&mut menu, &mut inv, Buttons::A), Outcome::Drop(ItemKind::StoneAxe, 1));
        assert_eq!(inv.gear.belt[0], None);
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
        assert_eq!(menu.mode, Mode::Gear { place: 0 });
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
        assert_eq!(menu.mode, Mode::Gear { place: 0 }, "the last page without the cart at hand");
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
        let mut menu = BagMenu { mode: Mode::Gear { place: 0 }, ..BagMenu::browse() };
        press(&mut menu, &mut inv, Buttons::R);
        assert_eq!(menu.mode, Mode::Gear { place: 0 });
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

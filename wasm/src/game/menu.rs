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
//! the grid or the crafting list reaches it). On the items page, A on an
//! item opens its commands; B closes the bag. On the crafting page, A makes
//! the chosen recipe from materials in the bag (holding the result if
//! there's no room for it) and B returns to the tabs. Holding an item, A
//! puts it down (swapping with the one item it covers, if any) and B
//! returns it where it came from (or, if it came off the ground or there's
//! no room left, leaves it on the ground).

use super::bag::{BAG_H, BAG_W, Bag, Placed, footprint};
use crate::common::input::{Buttons, InputState};
use crate::crafting::RECIPES;
use crate::items::{ItemKind, Use};

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
            Command::Drop if kind.can_put_down() => "PUT DOWN",
            Command::Drop => "POUR OUT",
        }
    }
}

/// Pages of the bag screen, in tab order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    Items,
    Craft,
}

impl Page {
    pub const ALL: [Page; 2] = [Page::Items, Page::Craft];

    pub fn label(self) -> &'static str {
        match self {
            Page::Items => "ITEMS",
            Page::Craft => "CRAFT",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// On the tab row, about to pick a page.
    Tabs(Page),
    Browse,
    /// Choosing what to do with the item at `item`.
    Commands { item: usize, choice: usize },
    Holding(HeldItem),
    /// Choosing something to make from `RECIPES`.
    Crafting { choice: usize },
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
    /// A tool was chosen to hold in hand.
    Wield(ItemKind),
    /// The recipe (index into `RECIPES`) is to be cooked over the fire;
    /// its materials are still in the bag.
    Cook(usize),
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
    held_frames: u32,
}

/// Commands offered for an item, in menu order. `in_hand` is whether it's
/// the tool already held.
pub fn commands_for(kind: ItemKind, in_hand: bool) -> Vec<Command> {
    let wield = (kind.is_tool() && !in_hand).then_some(Command::Wield);
    kind.usage().map(Command::Use).into_iter().chain(wield).chain([Command::Move, Command::Drop]).collect()
}

impl BagMenu {
    pub fn browse() -> Self {
        Self { cursor: (0, 0), mode: Mode::Browse, notice: None, near_fire: false, held_frames: 0 }
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
            Mode::Crafting { .. } => Page::Craft,
            _ => Page::Items,
        }
    }

    fn open(&mut self, page: Page) {
        self.notice = None;
        self.mode = match page {
            Page::Items => Mode::Browse,
            Page::Craft => Mode::Crafting { choice: 0 },
        };
    }

    /// `tool` is the tool in hand.
    pub fn update(&mut self, input: &InputState, bag: &mut Bag, tool: Option<ItemKind>) -> Outcome {
        let step = self.direction(input);
        if input.pressed(Buttons::X) && !matches!(self.mode, Mode::Holding(_)) {
            return Outcome::Close;
        }
        match self.mode {
            Mode::Tabs(page) => {
                let i = Page::ALL.iter().position(|&p| p == page).unwrap_or(0) as i32;
                match step {
                    Some((dx, 0)) => {
                        let next = Page::ALL[(i + dx).clamp(0, Page::ALL.len() as i32 - 1) as usize];
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
                    self.cursor = ((self.cursor.0 + dx).clamp(0, BAG_W - 1), (self.cursor.1 + dy).clamp(0, BAG_H - 1));
                }
                if input.pressed(Buttons::B) {
                    return Outcome::Close;
                }
                if input.pressed(Buttons::A)
                    && let Some(item) = bag.at(self.cursor.0, self.cursor.1)
                {
                    self.mode = Mode::Commands { item, choice: 0 };
                    self.notice = None;
                }
                Outcome::Stay
            }
            Mode::Commands { item, choice } => {
                let kind = bag.items()[item].kind;
                let commands = commands_for(kind, tool == Some(kind));
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
                        self.notice = Some("TOOK IT IN HAND");
                        Outcome::Wield(kind)
                    }
                    Command::Move => {
                        let p = bag.remove(item);
                        self.cursor = (p.x, p.y);
                        self.mode = Mode::Holding(HeldItem { kind, turned: p.turned, count: p.count, origin: Some(p) });
                        Outcome::Stay
                    }
                    Command::Drop => {
                        let p = bag.remove(item);
                        self.notice = Some(if kind.can_put_down() { "PUT IT DOWN" } else { "POURED IT OUT" });
                        Outcome::Drop(kind, p.count)
                    }
                }
            }
            Mode::Holding(held) => self.update_holding(input, bag, held, step),
            Mode::Crafting { choice } => self.update_crafting(input, bag, choice, step),
        }
    }

    fn update_crafting(&mut self, input: &InputState, bag: &mut Bag, mut choice: usize, step: Option<(i32, i32)>) -> Outcome {
        if let Some((_, dy)) = step.filter(|s| s.1 != 0) {
            if dy < 0 && choice == 0 {
                self.mode = Mode::Tabs(Page::Craft);
                return Outcome::Stay;
            }
            choice = (choice as i32 + dy).clamp(0, RECIPES.len() as i32 - 1) as usize;
            self.mode = Mode::Crafting { choice };
        }
        if input.pressed(Buttons::B) {
            self.mode = Mode::Tabs(Page::Craft);
            self.notice = None;
            return Outcome::Stay;
        }
        if !input.pressed(Buttons::A) {
            return Outcome::Stay;
        }
        let recipe = &RECIPES[choice];
        if recipe.at_fire && !self.near_fire {
            self.notice = Some("NEEDS A FIRE");
            return Outcome::Stay;
        }
        if !bag.can_make(recipe) {
            self.notice = Some("NOT ENOUGH MATERIALS");
            return Outcome::Stay;
        }
        if recipe.at_fire {
            // Cooked over the fire, outside the bag.
            return Outcome::Cook(choice);
        }
        bag.use_up(recipe);
        if bag.stow_one(recipe.makes) {
            self.notice = Some("MADE IT!");
        } else {
            *self = Self { near_fire: self.near_fire, ..Self::holding(recipe.makes, 1) };
        }
        Outcome::Stay
    }

    fn update_holding(&mut self, input: &InputState, bag: &mut Bag, mut held: HeldItem, step: Option<(i32, i32)>) -> Outcome {
        if input.pressed(Buttons::Y) && held.kind.size().0 != held.kind.size().1 {
            held.turned = !held.turned;
            self.mode = Mode::Holding(held);
        }
        let (w, h) = footprint(held.kind, held.turned);
        let (dx, dy) = step.unwrap_or((0, 0));
        self.cursor = ((self.cursor.0 + dx).clamp(0, BAG_W - w), (self.cursor.1 + dy).clamp(0, BAG_H - h));

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
                    self.cursor = (self.cursor.0.min(BAG_W - w), self.cursor.1.min(BAG_H - h));
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
            self.notice = Some(if held.kind.can_put_down() { "LEFT IT ON THE GROUND" } else { "POURED IT OUT" });
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
                let repeat = self.held_frames >= REPEAT_DELAY && (self.held_frames - REPEAT_DELAY) % REPEAT_EVERY == 0;
                repeat.then_some(d)
            }
            None => {
                self.held_frames = 0;
                None
            }
        }
    }
}

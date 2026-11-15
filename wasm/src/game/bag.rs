//! The castaway's bag: a grid of cells in which every item occupies a
//! rectangle the size of its footprint. Items may be turned a quarter (their
//! footprint swaps width and height) to fit; nothing may overlap or stick out.
//! Some things (leaves) stack: one placement holds up to `max_stack` of them.
//! The bag is always `BAG_W` wide; a pack frame on the back adds rows.

use crate::crafting::Recipe;
use crate::items::ItemKind;

/// Bag width in cells, and its rows with nothing on the back.
pub const BAG_W: i32 = 6;
pub const BASE_ROWS: i32 = 4;

/// An item (or a stack of them) lying in the bag with its top-left cell at
/// (`x`, `y`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Placed {
    pub kind: ItemKind,
    pub x: i32,
    pub y: i32,
    pub turned: bool,
    /// How many are stacked here (1 for things that don't stack).
    pub count: u32,
}

impl Placed {
    pub fn size(&self) -> (i32, i32) {
        footprint(self.kind, self.turned)
    }

    pub fn covers(&self, x: i32, y: i32) -> bool {
        let (w, h) = self.size();
        (self.x..self.x + w).contains(&x) && (self.y..self.y + h).contains(&y)
    }

    fn overlaps(&self, o: &Placed) -> bool {
        let ((w, h), (ow, oh)) = (self.size(), o.size());
        self.x < o.x + ow && o.x < self.x + w && self.y < o.y + oh && o.y < self.y + h
    }

    /// Whether it lies within a bag `rows` deep.
    pub fn in_bounds(&self, rows: i32) -> bool {
        let (w, h) = self.size();
        self.x >= 0 && self.y >= 0 && self.x + w <= BAG_W && self.y + h <= rows
    }

    /// How many more the stack has room for.
    pub fn room(&self) -> u32 {
        self.kind.max_stack() - self.count
    }
}

/// Cells an item covers (width, height), turned or not.
pub fn footprint(kind: ItemKind, turned: bool) -> (i32, i32) {
    let (w, h) = kind.size();
    if turned { (h, w) } else { (w, h) }
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Bag {
    items: Vec<Placed>,
    rows: i32,
}

impl Default for Bag {
    fn default() -> Self {
        Self { items: Vec::new(), rows: BASE_ROWS }
    }
}

impl Bag {
    /// How many rows of cells the bag has now.
    pub fn rows(&self) -> i32 {
        self.rows
    }

    /// Whether everything in it would still fit if it had `rows` rows.
    pub fn fits_rows(&self, rows: i32) -> bool {
        self.items.iter().all(|p| p.in_bounds(rows))
    }

    /// Grows or shrinks the bag to `rows`; the caller has checked
    /// `fits_rows`.
    pub fn set_rows(&mut self, rows: i32) {
        debug_assert!(self.fits_rows(rows));
        self.rows = rows;
    }

    pub fn items(&self) -> &[Placed] {
        &self.items
    }

    /// Index of the item covering a cell.
    pub fn at(&self, x: i32, y: i32) -> Option<usize> {
        self.items.iter().position(|p| p.covers(x, y))
    }

    /// Indices of the items a placement would overlap.
    pub fn overlapping(&self, p: &Placed) -> Vec<usize> {
        (0..self.items.len()).filter(|&i| self.items[i].overlaps(p)).collect()
    }

    pub fn fits(&self, p: &Placed) -> bool {
        p.in_bounds(self.rows) && self.overlapping(p).is_empty()
    }

    /// Puts an item in; the caller has checked it `fits`.
    pub fn put(&mut self, p: Placed) {
        debug_assert!(self.fits(&p));
        self.items.push(p);
    }

    pub fn remove(&mut self, i: usize) -> Placed {
        self.items.remove(i)
    }

    /// Adds up to `n` to the stack at `i`; returns how many didn't fit.
    pub fn add_to(&mut self, i: usize, n: u32) -> u32 {
        let p = &mut self.items[i];
        let added = n.min(p.room());
        p.count += added;
        n - added
    }

    /// Takes one off the item at `i`, removing it when it was the last.
    pub fn take_one(&mut self, i: usize) {
        if self.items[i].count > 1 {
            self.items[i].count -= 1;
        } else {
            self.items.remove(i);
        }
    }

    /// Takes one of a kind out of the bag; false if there's none.
    pub fn take_kind(&mut self, kind: ItemKind) -> bool {
        // From the smallest stack, so full ones stay full.
        let i = (0..self.items.len()).filter(|&i| self.items[i].kind == kind).min_by_key(|&i| self.items[i].count);
        i.map(|i| self.take_one(i)).is_some()
    }

    /// Puts `n` of a kind in: topping up stacks already carried, then into
    /// the first free spaces (reading order, upright before turned).
    /// Returns how many didn't fit.
    pub fn stow(&mut self, kind: ItemKind, mut n: u32) -> u32 {
        for i in 0..self.items.len() {
            if n > 0 && self.items[i].kind == kind {
                n = self.add_to(i, n);
            }
        }
        while n > 0 {
            let Some(mut p) = self.free_spot(kind) else {
                break;
            };
            p.count = n.min(kind.max_stack());
            n -= p.count;
            self.items.push(p);
        }
        n
    }

    /// Puts a single item in; false if there's no room.
    pub fn stow_one(&mut self, kind: ItemKind) -> bool {
        self.stow(kind, 1) == 0
    }

    fn free_spot(&self, kind: ItemKind) -> Option<Placed> {
        for turned in [false, true] {
            if turned && kind.size().0 == kind.size().1 {
                continue;
            }
            for y in 0..self.rows {
                for x in 0..BAG_W {
                    let p = Placed { kind, x, y, turned, count: 1 };
                    if self.fits(&p) {
                        return Some(p);
                    }
                }
            }
        }
        None
    }

    /// How many of a kind are carried, across stacks.
    pub fn count(&self, kind: ItemKind) -> u32 {
        self.items.iter().filter(|p| p.kind == kind).map(|p| p.count).sum()
    }

    pub fn has(&self, kind: ItemKind) -> bool {
        self.items.iter().any(|p| p.kind == kind)
    }

    /// Takes a recipe's materials out of the bag; the caller has checked
    /// they can be made (`Inventory::can_make`).
    pub fn use_up(&mut self, recipe: &Recipe) {
        for &(kind, n) in recipe.needs {
            for _ in 0..n {
                let taken = self.take_kind(kind);
                debug_assert!(taken, "material in the bag");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stacking() -> ItemKind {
        *ItemKind::ALL.iter().find(|k| k.max_stack() > 1 && k.size() == (1, 1)).expect("something stacks")
    }

    fn single() -> ItemKind {
        *ItemKind::ALL.iter().find(|k| k.max_stack() == 1 && k.size() == (1, 1)).expect("something doesn't stack")
    }

    fn long() -> ItemKind {
        *ItemKind::ALL.iter().find(|k| k.size().1 > 1 && k.size().0 == 1).expect("something is long")
    }

    #[test]
    fn a_turned_item_swaps_its_width_and_height() {
        let kind = long();
        let (w, h) = kind.size();
        assert_eq!(footprint(kind, false), (w, h));
        assert_eq!(footprint(kind, true), (h, w));
    }

    #[test]
    fn things_that_stack_fill_one_place_up_to_their_limit() {
        let kind = stacking();
        let mut bag = Bag::default();
        assert_eq!(bag.stow(kind, kind.max_stack()), 0);
        assert_eq!(bag.items().len(), 1);
        assert_eq!(bag.stow(kind, 1), 0);
        assert_eq!(bag.items().len(), 2);
        assert_eq!(bag.count(kind), kind.max_stack() + 1);
    }

    #[test]
    fn stowing_tops_up_a_stack_already_carried_first() {
        let kind = stacking();
        let mut bag = Bag::default();
        bag.stow(kind, 1);
        bag.stow(kind, 1);
        assert_eq!(bag.items().len(), 1);
        assert_eq!(bag.items()[0].count, 2);
    }

    #[test]
    fn things_that_dont_stack_each_take_a_place() {
        let kind = single();
        let mut bag = Bag::default();
        assert_eq!(bag.stow(kind, 2), 0);
        assert_eq!(bag.items().len(), 2);
    }

    #[test]
    fn a_full_bag_says_how_many_did_not_fit() {
        let kind = single();
        let cells = (BAG_W * BASE_ROWS) as u32;
        let mut bag = Bag::default();
        assert_eq!(bag.stow(kind, cells + 3), 3);
        assert_eq!(bag.count(kind), cells);
        assert!(!bag.stow_one(kind));
    }

    #[test]
    fn a_long_item_is_turned_when_it_only_fits_lying_down() {
        let kind = long();
        let (_, h) = kind.size();
        let mut bag = Bag::default();
        bag.set_rows(h - 1);
        assert!(bag.stow_one(kind));
        assert!(bag.items()[0].turned);
    }

    #[test]
    fn nothing_may_overlap_or_stick_out() {
        let kind = long();
        let mut bag = Bag::default();
        bag.put(Placed { kind, x: 0, y: 0, turned: false, count: 1 });
        assert!(!bag.fits(&Placed { kind, x: 0, y: 1, turned: false, count: 1 }));
        assert!(bag.fits(&Placed { kind, x: 1, y: 0, turned: false, count: 1 }));
        assert!(!bag.fits(&Placed { kind, x: BAG_W - 1, y: 0, turned: true, count: 1 }));
        assert!(!bag.fits(&Placed { kind, x: -1, y: 0, turned: false, count: 1 }));
        assert!(!bag.fits(&Placed { kind, x: 0, y: BASE_ROWS - 1, turned: false, count: 1 }));
    }

    #[test]
    fn every_cell_an_item_covers_finds_it() {
        let kind = long();
        let mut bag = Bag::default();
        bag.put(Placed { kind, x: 2, y: 1, turned: true, count: 1 });
        let (w, h) = footprint(kind, true);
        for y in 1..1 + h {
            for x in 2..2 + w {
                assert_eq!(bag.at(x, y), Some(0));
            }
        }
        assert_eq!(bag.at(1, 1), None);
    }

    #[test]
    fn the_bag_shrinks_only_while_everything_still_fits() {
        let mut bag = Bag::default();
        bag.put(Placed { kind: single(), x: 0, y: BASE_ROWS - 1, turned: false, count: 1 });
        assert!(bag.fits_rows(BASE_ROWS));
        assert!(!bag.fits_rows(BASE_ROWS - 1));
        assert!(bag.fits_rows(BASE_ROWS + 2));
    }

    #[test]
    fn taking_one_leaves_full_stacks_full() {
        let kind = stacking();
        let mut bag = Bag::default();
        bag.stow(kind, kind.max_stack() + 2);
        assert!(bag.take_kind(kind));
        assert!(bag.items().iter().any(|p| p.count == kind.max_stack()));
        assert_eq!(bag.count(kind), kind.max_stack() + 1);
    }

    #[test]
    fn the_last_one_taken_empties_its_place() {
        let kind = single();
        let mut bag = Bag::default();
        bag.stow_one(kind);
        assert!(bag.take_kind(kind));
        assert!(bag.items().is_empty());
        assert!(!bag.take_kind(kind));
    }
}

//! The castaway's bag: a grid of cells in which every item occupies a
//! rectangle the size of its footprint. Items may be turned a quarter (their
//! footprint swaps width and height) to fit; nothing may overlap or stick out.
//! Some things (leaves) stack: one placement holds up to `max_stack` of them.

use crate::crafting::Recipe;
use crate::items::ItemKind;

/// Bag size in cells.
pub const BAG_W: i32 = 6;
pub const BAG_H: i32 = 4;

/// An item (or a stack of them) lying in the bag with its top-left cell at
/// (`x`, `y`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

    pub fn in_bounds(&self) -> bool {
        let (w, h) = self.size();
        self.x >= 0 && self.y >= 0 && self.x + w <= BAG_W && self.y + h <= BAG_H
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

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Bag {
    items: Vec<Placed>,
}

impl Bag {
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
        p.in_bounds() && self.overlapping(p).is_empty()
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
            for y in 0..BAG_H {
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

    /// Whether the bag holds every material a recipe needs.
    pub fn can_make(&self, recipe: &Recipe) -> bool {
        recipe.needs.iter().all(|&(kind, n)| self.count(kind) >= n)
    }

    /// Takes a recipe's materials out of the bag; the caller has checked it
    /// `can_make` it.
    pub fn use_up(&mut self, recipe: &Recipe) {
        for &(kind, n) in recipe.needs {
            for _ in 0..n {
                let taken = self.take_kind(kind);
                debug_assert!(taken, "material in the bag");
            }
        }
    }
}

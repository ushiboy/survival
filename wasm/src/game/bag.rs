//! The castaway's bag: a grid of cells in which every item occupies a
//! rectangle the size of its footprint. Items may be turned a quarter (their
//! footprint swaps width and height) to fit; nothing may overlap or stick out.

use crate::items::ItemKind;

/// Bag size in cells.
pub const BAG_W: i32 = 6;
pub const BAG_H: i32 = 4;

/// An item lying in the bag with its top-left cell at (`x`, `y`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Placed {
    pub kind: ItemKind,
    pub x: i32,
    pub y: i32,
    pub turned: bool,
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

    /// Puts an item into the first free space (reading order, upright
    /// before turned). Returns false if there's no room.
    pub fn stow(&mut self, kind: ItemKind) -> bool {
        for turned in [false, true] {
            if turned && kind.size().0 == kind.size().1 {
                continue;
            }
            for y in 0..BAG_H {
                for x in 0..BAG_W {
                    let p = Placed { kind, x, y, turned };
                    if self.fits(&p) {
                        self.items.push(p);
                        return true;
                    }
                }
            }
        }
        false
    }

    pub fn count(&self, kind: ItemKind) -> u32 {
        self.items.iter().filter(|p| p.kind == kind).count() as u32
    }

    pub fn has(&self, kind: ItemKind) -> bool {
        self.items.iter().any(|p| p.kind == kind)
    }
}

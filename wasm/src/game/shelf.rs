//! The shelf in the mill: things left on it, as many as are put there.
//! Nothing is fitted into cells as in the bag: it keeps how many of each
//! kind there are, listed by name (A to Z).

use crate::items::ItemKind;

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Shelf {
    /// Each kind on it and how many, in the order listed (by name). No
    /// kind with none.
    stock: Vec<(ItemKind, u32)>,
}

impl Shelf {
    /// What's on it, as listed: each kind and how many, A to Z.
    pub fn lines(&self) -> &[(ItemKind, u32)] {
        &self.stock
    }

    /// How many of `kind` are on it.
    pub fn count(&self, kind: ItemKind) -> u32 {
        self.stock.iter().find(|&&(k, _)| k == kind).map_or(0, |&(_, n)| n)
    }

    /// Puts `n` of `kind` on it (there's always room).
    pub fn put(&mut self, kind: ItemKind, n: u32) {
        if n == 0 {
            return;
        }
        match self.stock.iter_mut().find(|(k, _)| *k == kind) {
            Some((_, have)) => *have += n,
            None => {
                let at = self.stock.partition_point(|(k, _)| k.name() < kind.name());
                self.stock.insert(at, (kind, n));
            }
        }
    }

    /// Takes up to `n` of `kind` off it; returns how many were taken.
    pub fn take(&mut self, kind: ItemKind, n: u32) -> u32 {
        let Some(i) = self.stock.iter().position(|&(k, _)| k == kind) else { return 0 };
        let taken = n.min(self.stock[i].1);
        self.stock[i].1 -= taken;
        if self.stock[i].1 == 0 {
            self.stock.remove(i);
        }
        taken
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_shelf_takes_as_many_as_are_put_on_it() {
        let mut shelf = Shelf::default();
        for _ in 0..100 {
            shelf.put(ItemKind::Stone, 50);
        }
        assert_eq!(shelf.count(ItemKind::Stone), 5000);
    }

    #[test]
    fn whats_on_the_shelf_is_listed_by_name() {
        let mut shelf = Shelf::default();
        for kind in [ItemKind::Stone, ItemKind::Bamboo, ItemKind::Rope, ItemKind::Clay] {
            shelf.put(kind, 1);
        }
        let names: Vec<&str> = shelf.lines().iter().map(|(k, _)| k.name()).collect();
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(names, sorted);
    }

    #[test]
    fn no_more_is_taken_off_the_shelf_than_is_on_it() {
        let mut shelf = Shelf::default();
        shelf.put(ItemKind::Rope, 3);
        assert_eq!(shelf.take(ItemKind::Rope, 5), 3);
        assert_eq!(shelf.take(ItemKind::Stone, 1), 0);
    }

    #[test]
    fn a_kind_all_taken_off_the_shelf_is_no_longer_listed() {
        let mut shelf = Shelf::default();
        shelf.put(ItemKind::Rope, 2);
        shelf.put(ItemKind::Stone, 2);
        shelf.take(ItemKind::Rope, 2);
        assert_eq!(shelf.lines(), &[(ItemKind::Stone, 2)]);
    }
}

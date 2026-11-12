//! Collects sprites for a frame and writes them to OAM in priority order.

use crate::ppu::oam::{Oam, ObjEntry};

/// OAM order decides overlap between sprites (lower index = in front), so
/// sprites are gathered first and flushed as: sky → actors (nearest first)
/// → ground shadows.
#[derive(Default)]
pub struct DrawList {
    sky: Vec<ObjEntry>,
    actors: Vec<(i32, ObjEntry)>,
    shadows: Vec<ObjEntry>,
}

impl DrawList {
    pub fn clear(&mut self) {
        self.sky.clear();
        self.actors.clear();
        self.shadows.clear();
    }

    /// Flying things drawn above everything else.
    pub fn sky(&mut self, e: ObjEntry) {
        self.sky.push(e);
    }

    /// `depth` is the actor's ground y; larger = nearer the viewer. Parts
    /// pushed with the same depth keep their order.
    pub fn actor(&mut self, depth: i32, e: ObjEntry) {
        self.actors.push((depth, e));
    }

    pub fn shadow(&mut self, e: ObjEntry) {
        self.shadows.push(e);
    }

    pub fn flush(&mut self, oam: &mut Oam) {
        self.actors.sort_by_key(|&(depth, _)| std::cmp::Reverse(depth));
        for e in self.sky.iter().chain(self.actors.iter().map(|(_, e)| e)).chain(self.shadows.iter()) {
            oam.push(*e);
        }
    }
}

//! Which parts of the island are awake. Creatures move only in the chunks
//! around the screen; the rest of the island waits, and when a chunk wakes
//! again it is told how long it slept so it can catch up on that time all
//! at once (crabs breeding or dying off, say) rather than being run frame by
//! frame while nobody watches.

use crate::common::math::Rect;
use crate::world::chunk::chunks_touching;
use crate::world::{CHUNK_PX, ChunkId, WorldMap};

/// How far around the screen (px) chunks wake, and how far they must be
/// left behind to fall asleep again (further, so a chunk at the edge doesn't
/// flicker between the two).
const WAKE_MARGIN: i32 = CHUNK_PX / 2;
const SLEEP_MARGIN: i32 = CHUNK_PX / 2 + 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    Awake,
    /// Asleep since this game minute.
    Asleep(u32),
}

pub struct Regions {
    count: (i32, i32),
    state: Vec<State>,
}

impl Regions {
    /// Every chunk asleep, as of `now` (the island as it was made).
    pub fn new(map: &WorldMap, now: u32) -> Self {
        let count = map.chunk_count();
        Self { count, state: vec![State::Asleep(now); (count.0 * count.1) as usize] }
    }

    /// Since when each chunk has slept (`None` if awake), row by row (for
    /// saving).
    pub fn asleep_since(&self) -> Vec<Option<u32>> {
        self.state
            .iter()
            .map(|s| match *s {
                State::Awake => None,
                State::Asleep(t) => Some(t),
            })
            .collect()
    }

    /// Puts the chunks back as saved: asleep since when they fell asleep,
    /// awake if they were (the next `update` lets any left behind sleep, as
    /// it would have). A save for another size of island leaves things as
    /// they are.
    pub fn restore(&mut self, since: &[Option<u32>]) {
        if since.len() == self.state.len() {
            self.state = since.iter().map(|s| s.map_or(State::Awake, State::Asleep)).collect();
        }
    }

    fn slot(&self, id: ChunkId) -> Option<usize> {
        let ok = (0..self.count.0).contains(&id.cx) && (0..self.count.1).contains(&id.cy);
        ok.then(|| (id.cy * self.count.0 + id.cx) as usize)
    }

    /// Whether things at a world pixel move this frame.
    pub fn is_awake(&self, (x, y): (i32, i32)) -> bool {
        self.slot(ChunkId::of_px(x, y)).is_some_and(|i| self.state[i] == State::Awake)
    }

    /// Wakes the chunks near the screen (`view`, world px) and puts those
    /// left far behind to sleep, as of game minute `now`. Returns the chunks
    /// just woken with the minutes each slept.
    pub fn update(&mut self, view: Rect, now: u32) -> Vec<(ChunkId, u32)> {
        let grow = |m: i32| Rect::new(view.x - m, view.y - m, view.w + 2 * m, view.h + 2 * m);
        let stay = grow(SLEEP_MARGIN);
        for cy in 0..self.count.1 {
            for cx in 0..self.count.0 {
                let id = ChunkId::new(cx, cy);
                let i = (cy * self.count.0 + cx) as usize;
                if self.state[i] == State::Awake && !id.rect().overlaps(&stay) {
                    self.state[i] = State::Asleep(now);
                }
            }
        }
        let mut woken = Vec::new();
        for id in chunks_touching(grow(WAKE_MARGIN), self.count) {
            let Some(i) = self.slot(id) else { continue };
            if let State::Asleep(since) = self.state[i] {
                self.state[i] = State::Awake;
                woken.push((id, now.saturating_sub(since)));
            }
        }
        woken
    }
}

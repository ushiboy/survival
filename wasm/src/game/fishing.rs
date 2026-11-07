//! A line in the water: the float, the fish it has drawn, and the bite.
//!
//! A fish within reach of the float swims over, nibbles a few times (the
//! float bobs), then bites (the float goes under) for `BITE_FRAMES`. Struck
//! then, it's caught; any other time, or too late, it's frightened off.

use crate::common::rng::Rng;
use crate::entity::Fish;

/// Frames the float stays under on a bite: the time to strike (3/4 s,
/// time enough to see it go and react).
pub const BITE_FRAMES: u32 = 45;
/// Frames between nibbles, and how long the float bobs on each.
const NIBBLE_GAP: (i32, i32) = (25, 60);
const BOB_FRAMES: u32 = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Waiting,
    Nibbling { left: u32 },
    Biting,
}

#[derive(Clone, Debug)]
pub struct Line {
    /// Where the float rides (world px).
    pub float: (i32, i32),
    /// The fish drawn to it, if any (index into the fish).
    pub fish: Option<usize>,
    phase: Phase,
    timer: u32,
    bob: u32,
    pub age: u32,
}

/// How the float looks this frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Float {
    Riding,
    Bobbing,
    Under,
}

impl Line {
    pub fn new(float: (i32, i32)) -> Self {
        Self { float, fish: None, phase: Phase::Waiting, timer: 0, bob: 0, age: 0 }
    }

    pub fn biting(&self) -> bool {
        self.phase == Phase::Biting
    }

    pub fn float(&self) -> Float {
        match self.phase {
            Phase::Biting => Float::Under,
            _ if self.bob > 0 => Float::Bobbing,
            _ => Float::Riding,
        }
    }

    /// One frame: draws in a fish, counts its nibbles to the bite, and
    /// frightens it off if the bite isn't struck in time. Whether the float
    /// was just pulled under.
    pub fn update(&mut self, fish: &mut [Fish], rng: &mut Rng) -> bool {
        self.age += 1;
        self.bob = self.bob.saturating_sub(1);
        match self.phase {
            Phase::Waiting => match self.fish {
                None => {
                    let near = fish
                        .iter()
                        .enumerate()
                        .filter(|(_, f)| f.would_take(self.float))
                        .min_by_key(|(_, f)| {
                            let (x, y) = f.pos();
                            (x - self.float.0).pow(2) + (y - self.float.1).pow(2)
                        })
                        .map(|(i, _)| i);
                    if let Some(i) = near {
                        fish[i].lure(self.float);
                        self.fish = Some(i);
                    }
                }
                Some(i) if fish[i].at_float() => {
                    self.phase = Phase::Nibbling { left: rng.between(1, 3) as u32 };
                    self.timer = rng.between(NIBBLE_GAP.0, NIBBLE_GAP.1) as u32;
                }
                Some(_) => {}
            },
            Phase::Nibbling { left } => {
                self.timer = self.timer.saturating_sub(1);
                if self.timer == 0 {
                    if left > 0 {
                        self.bob = BOB_FRAMES;
                        self.phase = Phase::Nibbling { left: left - 1 };
                        self.timer = rng.between(NIBBLE_GAP.0, NIBBLE_GAP.1) as u32;
                    } else {
                        self.phase = Phase::Biting;
                        self.timer = BITE_FRAMES;
                        return true;
                    }
                }
            }
            Phase::Biting => {
                self.timer = self.timer.saturating_sub(1);
                if self.timer == 0 {
                    // Too late: it got away.
                    self.let_go(fish);
                }
            }
        }
        false
    }

    /// Frightens off the fish at the float, if any, and waits again.
    pub fn let_go(&mut self, fish: &mut [Fish]) {
        if let Some(i) = self.fish.take() {
            fish[i].spook(self.float);
        }
        self.phase = Phase::Waiting;
    }
}

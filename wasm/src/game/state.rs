//! Survival-related state: time of day and the castaway's vitals.

use super::bag::Bag;
use crate::items::ItemKind;
use crate::assets::character::Outfit;

/// Video frames per in-game minute (one day ≈ 8 real minutes).
pub const FRAMES_PER_MINUTE: u32 = 20;
const MINUTES_PER_DAY: u32 = 24 * 60;

#[derive(Clone, Debug)]
pub struct GameClock {
    pub day: u32,
    /// Minute of the day, 0..1440.
    pub minute: u32,
    sub: u32,
}

impl GameClock {
    pub fn new(day: u32, hour: u32) -> Self {
        Self { day, minute: hour * 60, sub: 0 }
    }

    /// Returns true when a new in-game minute starts.
    pub fn tick(&mut self) -> bool {
        self.sub += 1;
        if self.sub < FRAMES_PER_MINUTE {
            return false;
        }
        self.sub = 0;
        self.advance_minutes(1);
        true
    }

    pub fn advance_minutes(&mut self, m: u32) {
        let total = self.minute + m;
        self.day += total / MINUTES_PER_DAY;
        self.minute = total % MINUTES_PER_DAY;
    }

    pub fn hour_minute(&self) -> (u32, u32) {
        (self.minute / 60, self.minute % 60)
    }

    /// Minutes since the start of day 0, for timing things across days.
    pub fn total_minutes(&self) -> u32 {
        self.day * MINUTES_PER_DAY + self.minute
    }

    /// Fractional minute of day, for smooth lighting.
    pub fn minute_f(&self) -> f32 {
        self.minute as f32 + self.sub as f32 / FRAMES_PER_MINUTE as f32
    }
}

#[derive(Clone, Debug)]
pub struct Vitals {
    pub hp: u8,
    pub max_hp: u8,
    /// 0..=MAX
    pub food: u32,
    pub water: u32,
}

impl Vitals {
    pub const MAX: u32 = 64;
    const FOOD_DECAY_MINUTES: u32 = 15;
    const WATER_DECAY_MINUTES: u32 = 10;

    pub fn new() -> Self {
        Self { hp: 5, max_hp: 5, food: Self::MAX, water: Self::MAX }
    }

    /// Called once per in-game minute. Placeholder decay until eating and
    /// drinking exist.
    pub fn on_minute(&mut self, clock: &GameClock) {
        let total = clock.day * MINUTES_PER_DAY + clock.minute;
        if total % Self::FOOD_DECAY_MINUTES == 0 {
            self.food = self.food.saturating_sub(1);
        }
        if total % Self::WATER_DECAY_MINUTES == 0 {
            self.water = self.water.saturating_sub(1);
        }
    }
}

impl Vitals {
    pub fn eat(&mut self, amount: u32) {
        self.food = (self.food + amount).min(Self::MAX);
    }

    pub fn drink(&mut self, amount: u32) {
        self.water = (self.water + amount).min(Self::MAX);
    }

    /// Restores `hearts` of health, up to the most there can be.
    pub fn heal(&mut self, hearts: u8) {
        self.hp = self.hp.saturating_add(hearts).min(self.max_hp);
    }
}

impl Default for Vitals {
    fn default() -> Self {
        Self::new()
    }
}

/// What the castaway carries and wears.
#[derive(Clone, Debug)]
pub struct Inventory {
    pub bag: Bag,
    /// Lights the surroundings at night (debug toggle until it's an item).
    pub lantern: bool,
    /// What the castaway is wearing.
    pub outfit: Outfit,
    /// The tool held in hand (one carried in the bag), used with B.
    pub tool: Option<ItemKind>,
}

impl Inventory {
    /// Keeps the tool in hand one that's still carried: lets go of one
    /// that left the bag, and takes up the first tool carried if the hands
    /// are empty.
    pub fn settle_tool(&mut self) {
        if self.tool.is_some_and(|t| !self.bag.has(t)) {
            self.tool = None;
        }
        if self.tool.is_none() {
            self.tool = self.bag.items().iter().map(|p| p.kind).find(|k| k.is_tool());
        }
    }
}

impl Default for Inventory {
    fn default() -> Self {
        Self { bag: Bag::default(), lantern: false, outfit: Outfit::CASTAWAY, tool: None }
    }
}

//! Survival-related state: time of day and the castaway's vitals.

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

    pub fn phase(&self) -> DayPhase {
        match self.minute / 60 {
            5 | 17 | 18 => DayPhase::Twilight,
            6..=16 => DayPhase::Day,
            _ => DayPhase::Night,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DayPhase {
    Day,
    Twilight,
    Night,
}

#[derive(Clone, Debug)]
pub struct Vitals {
    pub hp: u8,
    pub max_hp: u8,
    pub food: u8,
    pub water: u8,
}

impl Vitals {
    pub const GAUGE_MAX: u8 = 8;
    const FOOD_DECAY_MINUTES: u32 = 120;
    const WATER_DECAY_MINUTES: u32 = 90;

    pub fn new() -> Self {
        Self { hp: 5, max_hp: 5, food: Self::GAUGE_MAX, water: Self::GAUGE_MAX }
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

impl Default for Vitals {
    fn default() -> Self {
        Self::new()
    }
}

//! Survival-related state: time of day and the castaway's vitals.

use super::bag::{BASE_ROWS, Bag};
use crate::items::{ItemKind, Slot};
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
    /// Effort spent since each gauge last went down a notch (in
    /// `AWAKE_EFFORT`s per minute awake).
    food_spent: u32,
    water_spent: u32,
}

impl Vitals {
    pub const MAX: u32 = 64;
    /// Minutes awake for each gauge to go down a notch.
    const FOOD_DECAY_MINUTES: u32 = 15;
    const WATER_DECAY_MINUTES: u32 = 10;
    /// Effort of a minute awake and a minute asleep: sleep burns a sixth
    /// as fast.
    const AWAKE_EFFORT: u32 = 6;
    const ASLEEP_EFFORT: u32 = 1;

    pub fn new() -> Self {
        Self { hp: 5, max_hp: 5, food: Self::MAX, water: Self::MAX, food_spent: 0, water_spent: 0 }
    }

    /// Called once per in-game minute: hunger and thirst grow, slower while
    /// `asleep`.
    pub fn on_minute(&mut self, asleep: bool) {
        let effort = if asleep { Self::ASLEEP_EFFORT } else { Self::AWAKE_EFFORT };
        self.food_spent += effort;
        if self.food_spent >= Self::FOOD_DECAY_MINUTES * Self::AWAKE_EFFORT {
            self.food_spent -= Self::FOOD_DECAY_MINUTES * Self::AWAKE_EFFORT;
            self.food = self.food.saturating_sub(1);
        }
        self.water_spent += effort;
        if self.water_spent >= Self::WATER_DECAY_MINUTES * Self::AWAKE_EFFORT {
            self.water_spent -= Self::WATER_DECAY_MINUTES * Self::AWAKE_EFFORT;
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
    pub gear: Gear,
}

/// What's held and worn. The tool in hand is one carried in the bag (it
/// still takes room there); a pack on the back is out of the bag and adds
/// rows to it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Gear {
    pub hand: Option<ItemKind>,
    pub back: Option<ItemKind>,
}

impl Gear {
    pub fn worn(&self, slot: Slot) -> Option<ItemKind> {
        match slot {
            Slot::Hand => self.hand,
            Slot::Back => self.back,
        }
    }
}

impl Inventory {
    /// Lets go of a tool in hand that's no longer carried.
    pub fn settle_hand(&mut self) {
        if self.gear.hand.is_some_and(|t| !self.bag.has(t)) {
            self.gear.hand = None;
        }
    }

    /// Something was just obtained: a tool is taken in hand if the hands
    /// are empty.
    pub fn took(&mut self, kind: ItemKind) {
        if self.gear.hand.is_none() && kind.slot() == Some(Slot::Hand) && self.bag.has(kind) {
            self.gear.hand = Some(kind);
        }
    }

    /// Puts on the pack at bag index `i`, the bag growing (or shrinking) to
    /// suit it; any pack worn before goes into the bag. Nothing changes if
    /// things would no longer fit.
    pub fn wear(&mut self, i: usize) -> Result<(), &'static str> {
        let pack = self.bag.remove(i);
        let rows = BASE_ROWS + pack.kind.extra_rows();
        if !self.bag.fits_rows(rows) {
            self.bag.put(pack);
            return Err("EMPTY THE EXTRA SPACE FIRST");
        }
        let before = self.bag.rows();
        self.bag.set_rows(rows);
        if let Some(old) = self.gear.back
            && !self.bag.stow_one(old)
        {
            self.bag.set_rows(before);
            self.bag.put(pack);
            return Err("NO ROOM FOR THE OLD ONE");
        }
        self.gear.back = Some(pack.kind);
        Ok(())
    }

    /// Takes off what's worn in `slot`: a tool is just let go of; a pack
    /// goes into the bag, which shrinks back, if everything still fits.
    pub fn take_off(&mut self, slot: Slot) -> Result<(), &'static str> {
        match slot {
            Slot::Hand => self.gear.hand = None,
            Slot::Back => {
                let Some(pack) = self.gear.back else { return Ok(()) };
                if !self.bag.fits_rows(BASE_ROWS) {
                    return Err("EMPTY THE EXTRA SPACE FIRST");
                }
                let before = self.bag.rows();
                self.bag.set_rows(BASE_ROWS);
                if !self.bag.stow_one(pack) {
                    self.bag.set_rows(before);
                    return Err("NO ROOM IN THE BAG FOR IT");
                }
                self.gear.back = None;
            }
        }
        Ok(())
    }
}

impl Default for Inventory {
    fn default() -> Self {
        Self { bag: Bag::default(), lantern: false, outfit: Outfit::CASTAWAY, gear: Gear::default() }
    }
}

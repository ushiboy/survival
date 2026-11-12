//! Survival-related state: time of day and the castaway's vitals.

use super::bag::{BASE_ROWS, Bag};
use crate::crafting::RecipeBook;
use crate::items::{ItemKind, Slot};
use crate::assets::character::Outfit;

/// Video frames per in-game minute (one day ≈ 8 real minutes).
pub const FRAMES_PER_MINUTE: u32 = 20;
const MINUTES_PER_DAY: u32 = 24 * 60;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
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

/// How the castaway is passing a minute (how hungry and thirsty it makes
/// them).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rest {
    /// Up and about: `ran` is how many of the minute's frames were spent
    /// running (out of `FRAMES_PER_MINUTE`).
    Awake { ran: u32 },
    Asleep,
    /// Asleep under a shelter.
    Sheltered,
}

impl Rest {
    /// A minute awake, walking (or standing about).
    pub const AWAKE: Self = Self::Awake { ran: 0 };
    /// A minute awake spent running the whole time.
    pub const RUNNING: Self = Self::Awake { ran: FRAMES_PER_MINUTE };
}

/// Health, hunger and thirst: three gauges, health running from `max_hp`
/// down to nothing (and then the castaway collapses).
///
/// Health wears down while awake (tiredness), and faster with nothing to
/// eat or drink; sleep brings it back. Every rate is given as minutes for a
/// sixty-fourth of the gauge, so a bigger gauge keeps the same days.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Vitals {
    pub hp: u32,
    pub max_hp: u32,
    /// 0..=MAX
    pub food: u32,
    pub water: u32,
    /// Effort spent since each gauge last went down a notch (in
    /// `AWAKE_EFFORT`s per minute awake).
    food_spent: u32,
    water_spent: u32,
    /// Minutes awake not yet slept off (never shown): sleep pays it back
    /// two minutes for one, and with nothing to eat or drink that is all
    /// sleep gives back.
    tired: u32,
    /// Health worn away and mended, in points, not yet a whole one.
    wear: f32,
    mend: f32,
    /// Frames of running not yet a whole minute's extra tiredness (not
    /// kept: never more than a minute's worth).
    #[serde(skip)]
    run_tired: u32,
}

impl Vitals {
    pub const MAX: u32 = 64;
    /// Minutes awake for each gauge to go down a notch.
    const FOOD_DECAY_MINUTES: u32 = 15;
    const WATER_DECAY_MINUTES: u32 = 10;
    /// Effort of a minute awake. Asleep, hunger grows a sixth as fast
    /// (under a shelter a twenty-fourth), thirst half as fast (a sixth):
    /// breath still takes water, but out of the sun and wind a shelter
    /// keeps most of it in.
    const AWAKE_EFFORT: u32 = 24;
    /// Minutes for a sixty-fourth of health to wear away: awake (about a
    /// fifth of it in a 16-hour day), with no water, with no food.
    const TIRE_MINUTES: f32 = 75.0;
    const THIRST_MINUTES: f32 = 68.0;
    const HUNGER_MINUTES: f32 = 600.0;
    /// Minutes of sleep for a sixty-fourth of health to come back, in the
    /// open and under a shelter.
    const SLEEP_MEND_MINUTES: f32 = 15.0;
    const SHELTER_MEND_MINUTES: f32 = 10.0;
    /// Minutes of tiredness one minute of sleep pays back.
    const SLEEP_PAYS: u32 = 2;
    /// How many times as tiring a minute running is as a minute walking.
    const RUN_TIRE: u32 = 2;

    pub fn new() -> Self {
        Self { hp: Self::MAX, max_hp: Self::MAX, food: Self::MAX, water: Self::MAX, food_spent: 0, water_spent: 0, tired: 0, wear: 0.0, mend: 0.0, run_tired: 0 }
    }

    /// Nothing left to eat: too weak to run.
    pub fn starving(&self) -> bool {
        self.food == 0
    }

    /// Fed and watered: healing works (sleep mends past tiredness, cooked
    /// food and boiled water heal).
    fn nourished(&self) -> bool {
        self.food > 0 && self.water > 0
    }

    pub fn collapsed(&self) -> bool {
        self.hp == 0
    }

    /// Points of health for `minutes` at `per` minutes a sixty-fourth.
    fn points(&self, minutes: f32, per: f32) -> f32 {
        self.max_hp as f32 / 64.0 * minutes / per
    }

    /// Called once per in-game minute: hunger and thirst grow (slower while
    /// resting), health wears with them and with tiredness, and sleep mends
    /// it.
    pub fn on_minute(&mut self, rest: Rest) {
        let (food_effort, water_effort) = match rest {
            Rest::Awake { .. } => (Self::AWAKE_EFFORT, Self::AWAKE_EFFORT),
            Rest::Asleep => (4, 12),
            Rest::Sheltered => (1, 4),
        };
        self.food_spent += food_effort;
        if self.food_spent >= Self::FOOD_DECAY_MINUTES * Self::AWAKE_EFFORT {
            self.food_spent -= Self::FOOD_DECAY_MINUTES * Self::AWAKE_EFFORT;
            self.food = self.food.saturating_sub(1);
        }
        self.water_spent += water_effort;
        if self.water_spent >= Self::WATER_DECAY_MINUTES * Self::AWAKE_EFFORT {
            self.water_spent -= Self::WATER_DECAY_MINUTES * Self::AWAKE_EFFORT;
            self.water = self.water.saturating_sub(1);
        }

        let mut wear = 0.0;
        if self.water == 0 {
            wear += self.points(1.0, Self::THIRST_MINUTES);
        }
        if self.food == 0 {
            wear += self.points(1.0, Self::HUNGER_MINUTES);
        }
        let mut mend = 0.0;
        if let Rest::Awake { ran } = rest {
            // Running tires faster, and is slept off like any tiredness.
            let extra = (Self::RUN_TIRE - 1) * ran.min(FRAMES_PER_MINUTE);
            self.run_tired += extra;
            self.tired += 1 + self.run_tired / FRAMES_PER_MINUTE;
            self.run_tired %= FRAMES_PER_MINUTE;
            wear += self.points(1.0 + extra as f32 / FRAMES_PER_MINUTE as f32, Self::TIRE_MINUTES);
        } else {
            let paid = self.tired.min(Self::SLEEP_PAYS);
            self.tired -= paid;
            mend = if self.nourished() {
                let per = if rest == Rest::Sheltered { Self::SHELTER_MEND_MINUTES } else { Self::SLEEP_MEND_MINUTES };
                self.points(1.0, per)
            } else {
                // Hungry or thirsty: sleep only makes up for being tired.
                self.points(paid as f32, Self::TIRE_MINUTES)
            };
        }
        self.wear += wear;
        let lost = self.wear as u32;
        self.wear -= lost as f32;
        self.hp = self.hp.saturating_sub(lost);
        if self.hp >= self.max_hp {
            self.mend = 0.0;
        } else {
            self.mend += mend;
            let gained = self.mend as u32;
            self.mend -= gained as f32;
            self.hp = (self.hp + gained).min(self.max_hp);
        }
    }

    pub fn eat(&mut self, amount: u32) {
        self.food = (self.food + amount).min(Self::MAX);
    }

    pub fn drink(&mut self, amount: u32) {
        self.water = (self.water + amount).min(Self::MAX);
    }

    /// Restores `points` (of a 64-point gauge) of health, up to full; only
    /// when fed and watered.
    pub fn heal(&mut self, points: u32) {
        if self.nourished() {
            let gained = points * self.max_hp / 64;
            self.hp = (self.hp + gained).min(self.max_hp);
        }
    }
}

impl Default for Vitals {
    fn default() -> Self {
        Self::new()
    }
}

/// How long a torch burns once lit (game minutes): about a whole night.
pub const TORCH_BURN_MINUTES: u32 = 12 * 60;
/// The last stretch of it, when the flame burns low.
const TORCH_DYING_MINUTES: u32 = 60;

/// What the castaway carries and wears.
#[derive(Clone, Debug)]
pub struct Inventory {
    pub bag: Bag,
    /// When the torch carried was lit (game minutes), if one is.
    pub torch: Option<u32>,
    /// The recipes thought of so far.
    pub recipes: RecipeBook,
    /// What the castaway is wearing.
    pub outfit: Outfit,
    pub gear: Gear,
}

/// What's held and worn. The tool in hand is one carried in the bag (it
/// still takes room there); a pack on the back is out of the bag and adds
/// rows to it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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
    /// Whether a lit torch is carried, lighting the surroundings.
    pub fn torch_lit(&self) -> bool {
        self.torch.is_some()
    }

    /// Takes up a freshly lit torch: one already carried is lit afresh,
    /// so it burns for the full time again.
    pub fn light_torch(&mut self, now: u32) {
        self.torch = Some(now);
    }

    /// Whether the torch is in its last hour, burning low.
    pub fn torch_dying(&self, now: u32) -> bool {
        self.torch.is_some_and(|lit| now + TORCH_DYING_MINUTES >= lit + TORCH_BURN_MINUTES)
    }

    /// Burns out the torch once it's had its time.
    pub fn tend_torch(&mut self, now: u32) {
        if self.torch.is_some_and(|lit| now >= lit + TORCH_BURN_MINUTES) {
            self.torch = None;
        }
    }

    /// Thinks of any recipe whose materials are now all carried; whether
    /// one came to mind.
    pub fn think(&mut self) -> bool {
        let bag = &self.bag;
        self.recipes.notice(|kind| bag.has(kind))
    }

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
        Self { bag: Bag::default(), torch: None, recipes: RecipeBook::default(), outfit: Outfit::CASTAWAY, gear: Gear::default() }
    }
}

#[cfg(test)]
mod tests {
    use super::super::bag::{BAG_W, Placed};
    use super::*;

    const HOUR: u32 = 60;

    fn pass(v: &mut Vitals, minutes: u32, rest: Rest) {
        for _ in 0..minutes {
            v.on_minute(rest);
        }
    }

    /// A castaway worn by a long day awake, but fed and watered.
    fn tired_out() -> Vitals {
        let mut v = Vitals::new();
        pass(&mut v, 16 * HOUR, Rest::AWAKE);
        v.eat(Vitals::MAX);
        v.drink(Vitals::MAX);
        v
    }

    #[test]
    fn the_clock_rolls_over_into_the_next_day() {
        let mut c = GameClock::new(1, 23);
        c.advance_minutes(90);
        assert_eq!((c.day, c.hour_minute()), (2, (0, 30)));
    }

    #[test]
    fn a_minute_passes_every_so_many_frames() {
        let mut c = GameClock::new(1, 7);
        let ticks: Vec<bool> = (0..FRAMES_PER_MINUTE * 2).map(|_| c.tick()).collect();
        assert_eq!(ticks.iter().filter(|&&t| t).count(), 2);
        assert!(ticks[FRAMES_PER_MINUTE as usize - 1]);
        assert_eq!(c.hour_minute(), (7, 2));
    }

    #[test]
    fn total_minutes_count_across_days() {
        let mut c = GameClock::new(1, 0);
        let before = c.total_minutes();
        c.advance_minutes(25 * HOUR);
        assert_eq!(c.total_minutes() - before, 25 * HOUR);
    }

    #[test]
    fn a_day_awake_wears_health_hunger_and_thirst() {
        let mut v = Vitals::new();
        pass(&mut v, 4 * HOUR, Rest::AWAKE);
        assert!(v.hp < v.max_hp);
        assert!(v.food < Vitals::MAX);
        assert!(v.water < Vitals::MAX);
        assert!(v.water < v.food, "thirst comes faster than hunger");
    }

    #[test]
    fn hunger_and_thirst_grow_slower_asleep_and_slower_still_sheltered() {
        let used = |rest| {
            let mut v = Vitals::new();
            pass(&mut v, 8 * HOUR, rest);
            (Vitals::MAX - v.food, Vitals::MAX - v.water)
        };
        let (awake, asleep, sheltered) = (used(Rest::AWAKE), used(Rest::Asleep), used(Rest::Sheltered));
        assert!(asleep.0 < awake.0 && asleep.1 < awake.1);
        assert!(sheltered.0 < asleep.0 && sheltered.1 < asleep.1);
    }

    #[test]
    fn sleep_mends_health_and_a_shelter_mends_it_faster() {
        let slept = |rest| {
            let mut v = tired_out();
            pass(&mut v, 2 * HOUR, rest);
            v.hp
        };
        let before = tired_out().hp;
        assert!(slept(Rest::Asleep) > before);
        assert!(slept(Rest::Sheltered) > slept(Rest::Asleep));
    }

    #[test]
    fn a_night_fed_and_watered_mends_health_fully() {
        let mut v = tired_out();
        pass(&mut v, 10 * HOUR, Rest::Sheltered);
        assert_eq!(v.hp, v.max_hp);
    }

    #[test]
    fn hungry_sleep_only_makes_up_for_being_tired() {
        let mut fed = tired_out();
        let mut hungry = tired_out();
        hungry.food = 0;
        pass(&mut fed, 10 * HOUR, Rest::Asleep);
        pass(&mut hungry, 10 * HOUR, Rest::Asleep);
        assert_eq!(fed.hp, fed.max_hp);
        assert!(hungry.hp < hungry.max_hp);
    }

    #[test]
    fn running_wears_health_faster_than_walking() {
        let worn = |rest| {
            let mut v = Vitals::new();
            pass(&mut v, 4 * HOUR, rest);
            v.max_hp - v.hp
        };
        let some_running = Rest::Awake { ran: FRAMES_PER_MINUTE / 2 };
        assert!(worn(some_running) > worn(Rest::AWAKE), "running some of the time");
        assert!(worn(Rest::RUNNING) > worn(some_running), "running all the time");
    }

    #[test]
    fn running_doesnt_make_the_castaway_hungrier_or_thirstier() {
        let used = |rest| {
            let mut v = Vitals::new();
            pass(&mut v, 8 * HOUR, rest);
            (v.food, v.water)
        };
        assert_eq!(used(Rest::RUNNING), used(Rest::AWAKE));
    }

    #[test]
    fn tiredness_from_running_is_slept_off_too() {
        // Hungry, sleep gives back only what tiredness took.
        let slept_back = |rest| {
            let mut v = Vitals::new();
            v.food = 0;
            pass(&mut v, 4 * HOUR, rest);
            let before = v.hp;
            pass(&mut v, 6 * HOUR, Rest::Asleep);
            v.hp - before
        };
        assert!(slept_back(Rest::RUNNING) > slept_back(Rest::AWAKE));
    }

    #[test]
    fn going_without_water_wears_health_faster_than_without_food() {
        // Each hour the castaway eats and drinks what they aren't going without.
        let lost = |food: bool, water: bool| {
            let mut v = Vitals::new();
            for _ in 0..12 {
                v.food = if food { Vitals::MAX } else { 0 };
                v.water = if water { Vitals::MAX } else { 0 };
                pass(&mut v, HOUR, Rest::AWAKE);
            }
            v.max_hp - v.hp
        };
        let (fine, no_food, no_water) = (lost(true, true), lost(false, true), lost(true, false));
        assert!(no_food > fine);
        assert!(no_water > no_food);
    }

    #[test]
    fn health_running_out_is_a_collapse() {
        let mut v = Vitals::new();
        v.water = 0;
        v.food = 0;
        for _ in 0..10 * 24 * HOUR {
            if v.collapsed() {
                break;
            }
            v.on_minute(Rest::AWAKE);
        }
        assert!(v.collapsed());
        assert!(v.starving());
    }

    #[test]
    fn eating_and_drinking_fill_up_to_the_top() {
        let mut v = Vitals::new();
        v.food = 10;
        v.water = 10;
        v.eat(5);
        v.drink(5);
        assert_eq!((v.food, v.water), (15, 15));
        v.eat(Vitals::MAX);
        v.drink(Vitals::MAX);
        assert_eq!((v.food, v.water), (Vitals::MAX, Vitals::MAX));
    }

    #[test]
    fn healing_works_only_when_fed_and_watered() {
        let mut v = Vitals::new();
        v.hp = 10;
        v.water = 0;
        v.heal(8);
        assert_eq!(v.hp, 10);
        v.drink(1);
        v.heal(8);
        assert!(v.hp > 10);
        v.heal(Vitals::MAX);
        assert_eq!(v.hp, v.max_hp);
    }

    #[test]
    fn a_torch_burns_low_then_out() {
        let mut inv = Inventory::default();
        inv.light_torch(100);
        assert!(inv.torch_lit() && !inv.torch_dying(100));
        let end = 100 + TORCH_BURN_MINUTES;
        assert!(inv.torch_dying(end - 1));
        inv.tend_torch(end - 1);
        assert!(inv.torch_lit());
        inv.tend_torch(end);
        assert!(!inv.torch_lit());
    }

    #[test]
    fn lighting_a_torch_again_makes_it_burn_the_full_time() {
        let mut inv = Inventory::default();
        inv.light_torch(0);
        inv.light_torch(TORCH_BURN_MINUTES - 10);
        inv.tend_torch(TORCH_BURN_MINUTES);
        assert!(inv.torch_lit());
    }

    fn pack() -> ItemKind {
        *ItemKind::ALL.iter().find(|k| k.slot() == Some(Slot::Back)).expect("a pack")
    }

    fn tool() -> ItemKind {
        *ItemKind::ALL.iter().find(|k| k.slot() == Some(Slot::Hand)).expect("a tool")
    }

    /// Bag index of the first of a kind.
    fn find(inv: &Inventory, kind: ItemKind) -> usize {
        inv.bag.items().iter().position(|p| p.kind == kind).unwrap()
    }

    /// Fills rows `ys` of the bag with stones.
    fn fill_rows(inv: &mut Inventory, ys: std::ops::Range<i32>) {
        for y in ys {
            for x in 0..BAG_W {
                inv.bag.put(Placed { kind: ItemKind::Stone, x, y, turned: false, count: 1 });
            }
        }
    }

    #[test]
    fn a_tool_obtained_with_empty_hands_is_taken_in_hand() {
        let mut inv = Inventory::default();
        inv.bag.stow_one(tool());
        inv.took(tool());
        assert_eq!(inv.gear.hand, Some(tool()));
    }

    #[test]
    fn a_tool_obtained_with_hands_full_goes_in_the_bag_only() {
        let others: Vec<ItemKind> = ItemKind::ALL.iter().copied().filter(|k| k.slot() == Some(Slot::Hand)).collect();
        let (held, new) = (others[0], others[1]);
        let mut inv = Inventory::default();
        inv.bag.stow_one(held);
        inv.took(held);
        inv.bag.stow_one(new);
        inv.took(new);
        assert_eq!(inv.gear.hand, Some(held));
    }

    #[test]
    fn what_isnt_a_tool_is_not_taken_in_hand() {
        let mut inv = Inventory::default();
        inv.bag.stow_one(ItemKind::Stone);
        inv.took(ItemKind::Stone);
        assert_eq!(inv.gear.hand, None);
    }

    #[test]
    fn a_tool_no_longer_carried_is_let_go_of() {
        let mut inv = Inventory::default();
        inv.bag.stow_one(tool());
        inv.took(tool());
        inv.bag.take_kind(tool());
        inv.settle_hand();
        assert_eq!(inv.gear.hand, None);
    }

    #[test]
    fn wearing_a_pack_takes_it_out_of_the_bag_and_adds_rows() {
        let mut inv = Inventory::default();
        inv.bag.stow_one(pack());
        assert!(inv.wear(find(&inv, pack())).is_ok());
        assert_eq!(inv.gear.back, Some(pack()));
        assert!(!inv.bag.has(pack()));
        assert_eq!(inv.bag.rows(), BASE_ROWS + pack().extra_rows());
    }

    #[test]
    fn taking_off_a_pack_puts_it_in_the_bag_which_shrinks_back() {
        let mut inv = Inventory::default();
        inv.bag.stow_one(pack());
        inv.wear(find(&inv, pack())).unwrap();
        assert!(inv.take_off(Slot::Back).is_ok());
        assert_eq!(inv.gear.back, None);
        assert!(inv.bag.has(pack()));
        assert_eq!(inv.bag.rows(), BASE_ROWS);
    }

    #[test]
    fn a_pack_cant_come_off_with_things_in_the_rows_it_adds() {
        let mut inv = Inventory::default();
        inv.bag.stow_one(pack());
        inv.wear(find(&inv, pack())).unwrap();
        let rows = inv.bag.rows();
        fill_rows(&mut inv, BASE_ROWS..rows);
        let before = inv.bag.clone();
        assert!(inv.take_off(Slot::Back).is_err());
        assert_eq!(inv.gear.back, Some(pack()));
        assert_eq!(inv.bag, before, "nothing changes");
    }

    #[test]
    fn a_pack_cant_come_off_with_no_room_in_the_bag_for_it() {
        let mut inv = Inventory::default();
        inv.bag.stow_one(pack());
        inv.wear(find(&inv, pack())).unwrap();
        // Only the rows the pack adds are empty: nowhere for the pack once
        // they're gone.
        fill_rows(&mut inv, 0..BASE_ROWS);
        let before = inv.bag.clone();
        assert!(inv.take_off(Slot::Back).is_err());
        assert_eq!(inv.gear.back, Some(pack()));
        assert_eq!(inv.bag, before, "nothing changes");
    }

    #[test]
    fn letting_go_of_a_tool_leaves_it_in_the_bag() {
        let mut inv = Inventory::default();
        inv.bag.stow_one(tool());
        inv.took(tool());
        assert!(inv.take_off(Slot::Hand).is_ok());
        assert_eq!(inv.gear.hand, None);
        assert!(inv.bag.has(tool()));
    }
}

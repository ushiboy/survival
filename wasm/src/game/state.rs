//! Survival-related state: time of day and the castaway's vitals.

use super::bag::{BASE_ROWS, Bag};
use crate::assets::character::Outfit;
use crate::crafting::{Recipe, RecipeBook};
use crate::items::{ItemKind, Slot};

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
    Awake {
        ran: u32,
    },
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

/// Something ailing the castaway for a while. Only an upset stomach for
/// now (from something eaten or drunk raw); others can be added here, each
/// saying how long it lasts and what it does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Ailment {
    UpsetStomach,
}

impl Ailment {
    /// How long it lasts (game minutes) from when it took hold.
    pub fn minutes(self) -> u32 {
        match self {
            Ailment::UpsetStomach => 6 * 60,
        }
    }

    /// How many times as fast thirst grows with it.
    fn thirst(self) -> u32 {
        match self {
            Ailment::UpsetStomach => 2,
        }
    }

    /// Minutes for a sixty-fourth of health to wear away with it (about 4
    /// over an upset stomach's time: it can't bring anyone down alone).
    fn wear_minutes(self) -> Option<f32> {
        match self {
            Ailment::UpsetStomach => Some(90.0),
        }
    }

    /// Whether it keeps the castaway from running.
    fn stops_running(self) -> bool {
        match self {
            Ailment::UpsetStomach => true,
        }
    }
}

/// An ailment and the minutes it has left.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Ailing {
    pub ailment: Ailment,
    pub left: u32,
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
    /// What ails the castaway, each for the minutes it has left.
    pub ailments: Vec<Ailing>,
    /// Raw water has been drunk before: the first draw is always safe.
    pub drunk_raw_water: bool,
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
        Self {
            hp: Self::MAX,
            max_hp: Self::MAX,
            food: Self::MAX,
            water: Self::MAX,
            food_spent: 0,
            water_spent: 0,
            tired: 0,
            wear: 0.0,
            mend: 0.0,
            run_tired: 0,
            ailments: Vec::new(),
            drunk_raw_water: false,
        }
    }

    /// Nothing left to eat: too weak to run.
    pub fn starving(&self) -> bool {
        self.food == 0
    }

    /// Fit to run: not starving, nor kept from it by an ailment.
    pub fn can_run(&self) -> bool {
        !self.starving() && !self.ailments.iter().any(|a| a.ailment.stops_running())
    }

    pub fn has(&self, ailment: Ailment) -> bool {
        self.ailments.iter().any(|a| a.ailment == ailment)
    }

    /// Falls to `ailment`: for its full time from now (taken again, it starts
    /// over, no worse).
    pub fn catch(&mut self, ailment: Ailment) {
        self.ailments.retain(|a| a.ailment != ailment);
        self.ailments.push(Ailing { ailment, left: ailment.minutes() });
    }

    /// Gets over every ailment at once.
    pub fn cure(&mut self) {
        self.ailments.clear();
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

    /// How much of the health gauge must be left at the end of a stretch of
    /// work for the castaway to be fit to set to it: work that would leave
    /// them close to collapse isn't taken on.
    pub const WORK_SPARE: f32 = 1.0 / 8.0;

    /// Whether the castaway is fit to set to `minutes` of work (awake, as
    /// building the mill is): worn as they would be minute by minute
    /// (tiredness, and hunger, thirst and ailments as they come), at least
    /// `WORK_SPARE` of the gauge left at the end.
    pub fn fit_to_work(&self, minutes: u32) -> bool {
        let mut v = self.clone();
        for _ in 0..minutes {
            v.on_minute(Rest::Awake { ran: 0 });
        }
        v.hp as f32 >= v.max_hp as f32 * Self::WORK_SPARE
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
        let thirst = self.ailments.iter().map(|a| a.ailment.thirst()).max().unwrap_or(1);
        self.water_spent += water_effort * thirst;
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
        for a in &self.ailments {
            if let Some(per) = a.ailment.wear_minutes() {
                wear += self.points(1.0, per);
            }
        }
        for a in &mut self.ailments {
            a.left -= 1;
        }
        self.ailments.retain(|a| a.left > 0);
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
    /// What the handcart is loaded with, once one has been made (its grid
    /// is its own size, `cart::CART_W` by `cart::CART_ROWS`).
    pub cart: Option<Bag>,
}

/// What's held and worn. The tool in hand is one carried in the bag or
/// hung on the belt (in the bag it still takes room); a pack on the back
/// is out of the bag and adds rows to it; a belt at the waist is out of
/// the bag, and so are the tools hung on its hooks.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Gear {
    pub hand: Option<ItemKind>,
    pub back: Option<ItemKind>,
    pub waist: Option<ItemKind>,
    /// The tools on the belt's hooks (as many hooks as the belt has).
    pub belt: [Option<ItemKind>; BELT_HOOKS],
}

/// Hooks on a tool belt.
pub const BELT_HOOKS: usize = 2;

/// Cells each hook of the belt has, side by side under the bag: a tool
/// hung there lies along them, so only one at most this long and one cell
/// wide hangs (`ItemKind::hangs_on_belt`).
pub const HOOK_CELLS: i32 = 3;

impl Gear {
    pub fn worn(&self, slot: Slot) -> Option<ItemKind> {
        match slot {
            Slot::Hand => self.hand,
            Slot::Back => self.back,
            Slot::Waist => self.waist,
        }
    }

    /// The hooks there are on what's worn at the waist.
    pub fn hooks(&self) -> usize {
        self.waist.map_or(0, |b| b.hooks().min(BELT_HOOKS))
    }

    /// Whether a tool of `kind` hangs on the belt.
    pub fn on_belt(&self, kind: ItemKind) -> bool {
        self.belt.contains(&Some(kind))
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

    /// Whether a kind is carried: in the bag or hung on the belt.
    pub fn carries(&self, kind: ItemKind) -> bool {
        self.bag.has(kind) || self.gear.on_belt(kind)
    }

    /// Whether everything a recipe needs is at hand: its materials in the
    /// bag, and its tools carried (in the bag or on the belt).
    pub fn can_make(&self, recipe: &Recipe) -> bool {
        recipe.needs.iter().all(|&(kind, n)| self.bag.count(kind) >= n) && recipe.tools.iter().all(|&kind| self.carries(kind))
    }

    /// Lets go of a tool in hand that's no longer carried.
    pub fn settle_hand(&mut self) {
        if self.gear.hand.is_some_and(|t| !self.carries(t)) {
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

    /// Puts on the gear at bag index `i`: a pack on the back, the bag
    /// growing (or shrinking) to suit it, or a belt at the waist. What was
    /// worn there before goes into the bag. Nothing changes if things would
    /// no longer fit.
    pub fn wear(&mut self, i: usize) -> Result<(), &'static str> {
        match self.bag.items()[i].kind.slot() {
            Some(Slot::Back) => self.wear_pack(i),
            Some(Slot::Waist) => self.wear_belt(i),
            _ => Ok(()),
        }
    }

    fn wear_pack(&mut self, i: usize) -> Result<(), &'static str> {
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

    fn wear_belt(&mut self, i: usize) -> Result<(), &'static str> {
        let (bag, gear) = (self.bag.clone(), self.gear);
        let belt = self.bag.remove(i);
        if self.gear.waist.is_some() && self.take_off(Slot::Waist).is_err() {
            (self.bag, self.gear) = (bag, gear);
            return Err("NO ROOM FOR THE OLD ONE");
        }
        self.gear.waist = Some(belt.kind);
        Ok(())
    }

    /// Hangs the tool at bag index `i` on a free hook of the belt, out of
    /// the bag.
    pub fn hang(&mut self, i: usize) -> Result<(), &'static str> {
        let Some(hook) = self.gear.belt[..self.gear.hooks()].iter().position(Option::is_none) else {
            return Err(if self.gear.hooks() == 0 { "NO BELT TO HANG IT ON" } else { "NO FREE HOOK" });
        };
        self.hang_at(i, hook)
    }

    /// Hangs the tool at bag index `i` on belt hook `hook`; a tool already
    /// there goes into the bag in its place.
    pub fn hang_at(&mut self, i: usize, hook: usize) -> Result<(), &'static str> {
        let kind = self.bag.items()[i].kind;
        if hook >= self.gear.hooks() {
            return Err("NO BELT TO HANG IT ON");
        }
        if !kind.hangs_on_belt() {
            return Err("IT WON'T HANG ON THE BELT");
        }
        let bag = self.bag.clone();
        self.bag.take_one(i);
        if let Some(old) = self.gear.belt[hook]
            && !self.bag.stow_one(old)
        {
            self.bag = bag;
            return Err("NO ROOM FOR THE OLD ONE");
        }
        self.gear.belt[hook] = Some(kind);
        Ok(())
    }

    /// Takes the tool on belt hook `hook` back into the bag.
    pub fn unhang(&mut self, hook: usize) -> Result<(), &'static str> {
        let Some(kind) = self.gear.belt[hook] else { return Ok(()) };
        if !self.bag.stow_one(kind) {
            return Err("NO ROOM IN THE BAG FOR IT");
        }
        self.gear.belt[hook] = None;
        Ok(())
    }

    /// The hook a kind carried hangs on, if it isn't in the bag.
    pub fn hook_of(&self, kind: ItemKind) -> Option<usize> {
        if self.bag.has(kind) {
            return None;
        }
        self.gear.belt.iter().position(|&k| k == Some(kind))
    }

    /// Puts `kind` in `slot`: one in the bag (or, to hold in hand, on the
    /// belt). What was there before goes back into the bag; nothing
    /// changes if it wouldn't fit.
    pub fn put_in(&mut self, slot: Slot, kind: ItemKind) -> Result<(), &'static str> {
        if !slot.holds(kind) {
            return Err("IT DOES NOT GO THERE");
        }
        if slot.keeps_in_bag() {
            if !self.carries(kind) {
                return Err("YOU DO NOT HAVE IT");
            }
            self.gear.hand = Some(kind);
            return Ok(());
        }
        let Some(i) = self.bag.items().iter().position(|p| p.kind == kind) else { return Err("YOU DO NOT HAVE IT") };
        self.wear(i)
    }

    /// Empties `slot`: takes off what's worn there into the bag (or just
    /// lets go of a tool in hand).
    pub fn empty(&mut self, slot: Slot) -> Result<(), &'static str> {
        self.take_off(slot)
    }

    /// Takes off what's worn in `slot`: a tool is just let go of; a pack
    /// goes into the bag, which shrinks back, if everything still fits; a
    /// belt goes into the bag with the tools on it, if there's room for
    /// them all.
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
            Slot::Waist => {
                let Some(belt) = self.gear.waist else { return Ok(()) };
                let mut bag = self.bag.clone();
                let all = self.gear.belt.iter().flatten().chain([&belt]).all(|&k| bag.stow_one(k));
                if !all {
                    return Err("NO ROOM IN THE BAG FOR IT");
                }
                self.bag = bag;
                self.gear.waist = None;
                self.gear.belt = [None; BELT_HOOKS];
            }
        }
        Ok(())
    }
}

impl Default for Inventory {
    fn default() -> Self {
        Self { bag: Bag::default(), torch: None, recipes: RecipeBook::default(), outfit: Outfit::CASTAWAY, gear: Gear::default(), cart: None }
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

    /// The longest work (in whole hours, up to a day) `v` is fit for.
    fn longest_work(v: &Vitals) -> u32 {
        (0..=24).take_while(|&h| v.fit_to_work(h * HOUR)).last().unwrap_or(0)
    }

    #[test]
    fn rested_and_fed_the_castaway_is_fit_for_a_long_days_work() {
        assert!(Vitals::new().fit_to_work(10 * HOUR));
    }

    #[test]
    fn weak_the_castaway_is_fit_only_for_shorter_work() {
        let mut weak = Vitals::new();
        weak.hp = weak.max_hp / 6;
        assert!(longest_work(&weak) < longest_work(&Vitals::new()));
        assert!(weak.fit_to_work(HOUR) && !weak.fit_to_work(10 * HOUR));
    }

    #[test]
    fn work_that_would_leave_the_castaway_close_to_collapse_isnt_fit_for() {
        let mut v = Vitals::new();
        v.hp = 3;
        assert!(!v.fit_to_work(HOUR), "it would leave next to nothing");
    }

    #[test]
    fn thirsty_the_castaway_is_fit_for_less_work_than_with_water() {
        let mut thirsty = Vitals::new();
        thirsty.hp = thirsty.max_hp / 2;
        let watered = thirsty.clone();
        thirsty.water = 0;
        assert!(longest_work(&thirsty) < longest_work(&watered));
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

    fn recipe(makes: ItemKind) -> &'static Recipe {
        crate::crafting::RECIPES.iter().find(|r| r.makes == makes).unwrap()
    }

    #[test]
    fn making_needs_every_material_in_number() {
        // Two stones make a stone knife.
        let knife = recipe(ItemKind::StoneKnife);
        let mut inv = Inventory::default();
        inv.bag.stow(ItemKind::Stone, 1);
        assert!(!inv.can_make(knife));
        inv.bag.stow(ItemKind::Stone, 1);
        assert!(inv.can_make(knife));
    }

    #[test]
    fn making_uses_up_the_materials_but_not_the_tools() {
        // A coconut is opened with a stone knife.
        let opened = recipe(ItemKind::OpenedCoconut);
        let mut inv = Inventory::default();
        inv.bag.stow(ItemKind::Coconut, 1);
        assert!(!inv.can_make(opened));
        inv.bag.stow(ItemKind::StoneKnife, 1);
        assert!(inv.can_make(opened));
        inv.bag.use_up(opened);
        assert!(!inv.bag.has(ItemKind::Coconut));
        assert!(inv.bag.has(ItemKind::StoneKnife));
    }

    // The tool belt, worn at the waist: tools hung on it are carried out
    // of the bag.

    /// Wearing a tool belt, with `tools` in the bag.
    fn belted(tools: &[ItemKind]) -> Inventory {
        let mut inv = Inventory::default();
        inv.bag.stow_one(ItemKind::ToolBelt);
        inv.wear(find(&inv, ItemKind::ToolBelt)).unwrap();
        for &t in tools {
            inv.bag.stow_one(t);
        }
        inv
    }

    fn hangable() -> Vec<ItemKind> {
        ItemKind::ALL.iter().copied().filter(|k| k.hangs_on_belt()).collect()
    }

    #[test]
    fn every_slot_is_filled_from_the_bag_and_emptied_the_same_way() {
        let mut inv = Inventory::default();
        for k in [tool(), pack(), ItemKind::ToolBelt] {
            inv.bag.stow_one(k);
        }
        for slot in Slot::ALL {
            let kind = [tool(), pack(), ItemKind::ToolBelt].into_iter().find(|&k| slot.holds(k)).unwrap();
            assert!(inv.put_in(slot, kind).is_ok(), "{slot:?}");
            assert_eq!(inv.gear.worn(slot), Some(kind));
            assert!(inv.empty(slot).is_ok(), "{slot:?}");
            assert_eq!(inv.gear.worn(slot), None);
            assert!(inv.bag.has(kind), "{slot:?} gave {kind:?} back to the bag");
        }
    }

    #[test]
    fn only_the_hand_keeps_what_it_holds_in_the_bag() {
        let mut inv = Inventory::default();
        inv.bag.stow_one(tool());
        inv.bag.stow_one(pack());
        inv.put_in(Slot::Hand, tool()).unwrap();
        assert!(inv.bag.has(tool()));
        inv.put_in(Slot::Back, pack()).unwrap();
        assert!(!inv.bag.has(pack()));
    }

    #[test]
    fn a_tool_on_a_hook_can_be_put_in_the_hand() {
        let tool = hangable()[0];
        let mut inv = belted(&[tool]);
        inv.hang(find(&inv, tool)).unwrap();
        assert!(inv.put_in(Slot::Hand, tool).is_ok());
        assert_eq!(inv.gear.hand, Some(tool));
        assert_eq!(inv.hook_of(tool), Some(0));
    }

    #[test]
    fn a_slot_holds_only_its_own_gear() {
        assert!(Slot::Back.holds(pack()));
        assert!(!Slot::Back.holds(tool()));
        assert!(Slot::Hand.holds(tool()));
    }

    #[test]
    fn every_tool_that_hangs_fits_on_a_hook_lying_down() {
        for kind in hangable() {
            let (w, h) = kind.size();
            assert!(w.min(h) == 1 && w.max(h) <= HOOK_CELLS, "{kind:?} is too big to hang");
        }
    }

    #[test]
    fn the_hooks_lie_side_by_side_under_the_bag() {
        assert!(ItemKind::ToolBelt.hooks() as i32 * HOOK_CELLS <= super::super::bag::BAG_W);
    }

    #[test]
    fn wearing_a_tool_belt_takes_it_out_of_the_bag_and_leaves_its_rows_alone() {
        let inv = belted(&[]);
        assert_eq!(inv.gear.waist, Some(ItemKind::ToolBelt));
        assert!(!inv.bag.has(ItemKind::ToolBelt));
        assert_eq!(inv.bag.rows(), BASE_ROWS);
    }

    #[test]
    fn a_tool_hung_on_the_belt_takes_no_room_in_the_bag_but_is_still_carried() {
        let tool = hangable()[0];
        let mut inv = belted(&[tool]);
        assert!(inv.hang(find(&inv, tool)).is_ok());
        assert!(!inv.bag.has(tool));
        assert!(inv.carries(tool));
    }

    #[test]
    fn a_tool_on_the_belt_can_be_held_and_stays_in_hand() {
        let tool = hangable()[0];
        let mut inv = belted(&[tool]);
        inv.took(tool);
        inv.hang(find(&inv, tool)).unwrap();
        inv.settle_hand();
        assert_eq!(inv.gear.hand, Some(tool));
    }

    #[test]
    fn the_belt_holds_two_tools_at_most() {
        let tools = hangable();
        let mut inv = belted(&tools[..3]);
        assert!(inv.hang(find(&inv, tools[0])).is_ok());
        assert!(inv.hang(find(&inv, tools[1])).is_ok());
        assert!(inv.hang(find(&inv, tools[2])).is_err());
        assert!(inv.bag.has(tools[2]), "the third stays in the bag");
    }

    #[test]
    fn big_tools_dont_hang_on_the_belt() {
        for big in [ItemKind::Ladder, ItemKind::LogBridge, ItemKind::WaterWheel] {
            assert!(!big.hangs_on_belt(), "{big:?}");
            let mut inv = belted(&[big]);
            assert!(inv.hang(find(&inv, big)).is_err());
        }
    }

    #[test]
    fn nothing_hangs_without_a_belt() {
        let tool = hangable()[0];
        let mut inv = Inventory::default();
        inv.bag.stow_one(tool);
        assert!(inv.hang(find(&inv, tool)).is_err());
        assert!(inv.bag.has(tool));
    }

    #[test]
    fn a_tool_taken_off_the_belt_goes_back_in_the_bag() {
        let tool = hangable()[0];
        let mut inv = belted(&[tool]);
        inv.hang(find(&inv, tool)).unwrap();
        assert!(inv.unhang(0).is_ok());
        assert!(inv.bag.has(tool));
        assert_eq!(inv.gear.belt, [None, None]);
    }

    #[test]
    fn a_tool_cant_come_off_the_belt_with_no_room_in_the_bag() {
        let tool = hangable()[0];
        let mut inv = belted(&[tool]);
        inv.hang(find(&inv, tool)).unwrap();
        fill_rows(&mut inv, 0..BASE_ROWS);
        assert!(inv.unhang(0).is_err());
        assert_eq!(inv.gear.belt[0], Some(tool));
    }

    #[test]
    fn taking_off_the_belt_puts_it_and_its_tools_in_the_bag() {
        let tools = hangable();
        let mut inv = belted(&tools[..2]);
        inv.hang(find(&inv, tools[0])).unwrap();
        inv.hang(find(&inv, tools[1])).unwrap();
        assert!(inv.take_off(Slot::Waist).is_ok());
        assert_eq!(inv.gear.waist, None);
        assert_eq!(inv.gear.belt, [None, None]);
        assert!(inv.bag.has(ItemKind::ToolBelt) && inv.bag.has(tools[0]) && inv.bag.has(tools[1]));
    }

    #[test]
    fn the_belt_cant_come_off_with_no_room_for_it_and_its_tools() {
        let tool = hangable()[0];
        let mut inv = belted(&[tool]);
        inv.hang(find(&inv, tool)).unwrap();
        fill_rows(&mut inv, 0..BASE_ROWS);
        let (bag, gear) = (inv.bag.clone(), inv.gear);
        assert!(inv.take_off(Slot::Waist).is_err());
        assert_eq!((inv.bag, inv.gear), (bag, gear), "nothing changes");
    }

    #[test]
    fn only_one_belt_is_worn_another_puts_the_first_back_in_the_bag_with_its_tools() {
        let tool = hangable()[0];
        let mut inv = belted(&[tool]);
        inv.hang(find(&inv, tool)).unwrap();
        inv.bag.stow_one(ItemKind::ToolBelt);
        assert!(inv.wear(find(&inv, ItemKind::ToolBelt)).is_ok());
        assert_eq!(inv.gear.waist, Some(ItemKind::ToolBelt));
        assert_eq!(inv.bag.count(ItemKind::ToolBelt), 1, "the first belt is back in the bag");
        assert_eq!(inv.gear.belt, [None, None], "the new belt has nothing on it");
        assert!(inv.bag.has(tool), "the first belt's tool is back in the bag");
    }

    #[test]
    fn another_belt_isnt_put_on_with_no_room_for_the_first_and_its_tools() {
        let tool = hangable()[0];
        let mut inv = belted(&[tool]);
        inv.hang(find(&inv, tool)).unwrap();
        // The bag full but for the second belt.
        inv.bag.put(Placed { kind: ItemKind::ToolBelt, x: 0, y: 0, turned: false, count: 1 });
        for y in 0..BASE_ROWS {
            for x in 0..BAG_W {
                if inv.bag.at(x, y).is_none() {
                    inv.bag.put(Placed { kind: ItemKind::Stone, x, y, turned: false, count: 1 });
                }
            }
        }
        let (bag, gear) = (inv.bag.clone(), inv.gear);
        assert!(inv.wear(find(&inv, ItemKind::ToolBelt)).is_err());
        assert_eq!((inv.bag, inv.gear), (bag, gear), "nothing changes");
    }

    #[test]
    fn a_tool_on_the_belt_serves_for_a_recipe() {
        let recipe = crate::crafting::RECIPES.iter().find(|r| r.tools.iter().any(|t| t.hangs_on_belt())).expect("a recipe with a tool that hangs");
        let tool = *recipe.tools.iter().find(|t| t.hangs_on_belt()).unwrap();
        let others: Vec<ItemKind> = recipe.tools.iter().copied().filter(|&t| t != tool).collect();
        let mut inv = belted(&[tool]);
        for &o in &others {
            inv.bag.stow_one(o);
        }
        for &(kind, n) in recipe.needs {
            inv.bag.stow(kind, n);
        }
        inv.hang(find(&inv, tool)).unwrap();
        assert!(inv.can_make(recipe));
    }

    // An upset stomach (from something eaten or drunk raw).

    /// A castaway fed and watered who has just upset their stomach.
    fn upset() -> Vitals {
        let mut v = Vitals::new();
        v.catch(Ailment::UpsetStomach);
        v
    }

    #[test]
    fn an_upset_stomach_makes_thirst_grow_faster() {
        for rest in [Rest::AWAKE, Rest::Asleep, Rest::Sheltered] {
            let (mut well, mut ill) = (Vitals::new(), upset());
            pass(&mut well, 2 * HOUR, rest);
            pass(&mut ill, 2 * HOUR, rest);
            assert!(ill.water < well.water, "{rest:?}");
            assert_eq!(ill.food, well.food, "hunger is as it was");
        }
    }

    #[test]
    fn an_upset_stomach_wears_health_away() {
        let (mut well, mut ill) = (Vitals::new(), upset());
        pass(&mut well, 4 * HOUR, Rest::AWAKE);
        pass(&mut ill, 4 * HOUR, Rest::AWAKE);
        assert!(ill.hp < well.hp);
    }

    #[test]
    fn an_upset_stomach_alone_does_not_bring_the_castaway_down() {
        let mut v = upset();
        pass(&mut v, Ailment::UpsetStomach.minutes(), Rest::AWAKE);
        assert!(v.hp > v.max_hp / 2);
    }

    #[test]
    fn an_upset_stomach_passes_with_time() {
        let mut v = upset();
        pass(&mut v, Ailment::UpsetStomach.minutes() - 1, Rest::AWAKE);
        assert!(v.has(Ailment::UpsetStomach));
        pass(&mut v, 1, Rest::AWAKE);
        assert!(!v.has(Ailment::UpsetStomach));
    }

    #[test]
    fn an_upset_stomach_passes_in_sleep_too() {
        let mut v = upset();
        pass(&mut v, Ailment::UpsetStomach.minutes(), Rest::Asleep);
        assert!(!v.has(Ailment::UpsetStomach));
    }

    #[test]
    fn upsetting_it_again_starts_it_over_but_no_worse() {
        let mut again = upset();
        pass(&mut again, 2 * HOUR, Rest::AWAKE);
        again.catch(Ailment::UpsetStomach);
        let mut once = upset();
        let (w0, h0) = (again.water, again.hp);
        pass(&mut again, HOUR, Rest::AWAKE);
        pass(&mut once, HOUR, Rest::AWAKE);
        assert_eq!(w0 - again.water, Vitals::MAX - once.water, "thirst grows as fast as with one");
        assert!(h0 - again.hp <= once.max_hp - once.hp + 1);
        pass(&mut again, Ailment::UpsetStomach.minutes() - HOUR - 1, Rest::AWAKE);
        assert!(again.has(Ailment::UpsetStomach), "it lasts its full time from the second");
    }

    #[test]
    fn with_an_upset_stomach_the_castaway_cannot_run() {
        assert!(Vitals::new().can_run());
        assert!(!upset().can_run());
        let mut starving = Vitals::new();
        starving.food = 0;
        assert!(!starving.can_run());
    }
}

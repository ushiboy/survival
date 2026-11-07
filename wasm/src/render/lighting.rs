//! Time-of-day lighting and water animation, done the way 16-bit games did:
//! recomputed palettes uploaded to CGRAM each frame, palette cycling, and at
//! night an HDMA-driven window with subtractive colour math that blacks out
//! everything outside the lantern's light.

use crate::assets::palettes::{HUD_LIGHT_MASK, OBJ_SHADOW, water_cycles};
use crate::ppu::{SCREEN_H, SCREEN_W};
use crate::ppu::effects::LineTables;
use crate::ppu::palette::{Cgram, Rgb555};

#[derive(Clone, Copy, Debug)]
struct Key {
    minute: f32,
    mul: [f32; 3],
    /// Added after multiplying, in 0..31 units.
    add: [f32; 3],
    darkness: f32,
}

/// Night palette tint. Kept fairly bright: it is what the lantern light
/// shows. The darkness itself comes from colour math (see `night_darkness`).
const NIGHT: ([f32; 3], [f32; 3], f32) = ([0.80, 0.64, 0.50], [1.0, 0.5, 0.0], 1.0);
const KEYS: [Key; 9] = [
    Key { minute: 0.0, mul: NIGHT.0, add: NIGHT.1, darkness: NIGHT.2 },
    Key { minute: 270.0, mul: NIGHT.0, add: NIGHT.1, darkness: NIGHT.2 },
    Key { minute: 330.0, mul: [0.80, 0.62, 0.74], add: [3.0, 0.0, 2.0], darkness: 0.4 },
    Key { minute: 390.0, mul: [1.0, 1.0, 1.0], add: [0.0, 0.0, 0.0], darkness: 0.0 },
    Key { minute: 960.0, mul: [1.0, 1.0, 1.0], add: [0.0, 0.0, 0.0], darkness: 0.0 },
    Key { minute: 1050.0, mul: [1.0, 0.84, 0.64], add: [3.0, 1.0, 0.0], darkness: 0.0 },
    Key { minute: 1110.0, mul: [0.68, 0.48, 0.62], add: [2.0, 0.0, 3.0], darkness: 0.45 },
    Key { minute: 1170.0, mul: NIGHT.0, add: NIGHT.1, darkness: NIGHT.2 },
    Key { minute: 1440.0, mul: NIGHT.0, add: NIGHT.1, darkness: NIGHT.2 },
];

/// Lighting state for a moment of the day.
#[derive(Clone, Copy, Debug)]
pub struct Lighting {
    mul: [f32; 3],
    add: [f32; 3],
    /// 0 = full daylight, 1 = deep night.
    pub darkness: f32,
}

impl Lighting {
    pub fn at(minute_of_day: f32) -> Self {
        let m = minute_of_day.rem_euclid(1440.0);
        let i = KEYS.windows(2).position(|w| m >= w[0].minute && m < w[1].minute).unwrap_or(0);
        let (a, b) = (KEYS[i], KEYS[i + 1]);
        let t = (m - a.minute) / (b.minute - a.minute);
        let lerp3 = |x: [f32; 3], y: [f32; 3]| std::array::from_fn(|k| x[k] + (y[k] - x[k]) * t);
        Self { mul: lerp3(a.mul, b.mul), add: lerp3(a.add, b.add), darkness: a.darkness + (b.darkness - a.darkness) * t }
    }

    /// Tints every BG/OBJ colour except the HUD and shadow palettes.
    pub fn apply(&self, cg: &mut Cgram) {
        let shadow = Cgram::obj_index(OBJ_SHADOW, 0)..Cgram::obj_index(OBJ_SHADOW + 1, 0);
        let hud_end = Cgram::bg_index(2, 0);
        for (i, c) in cg.colors.iter_mut().enumerate() {
            if i < hud_end || shadow.contains(&i) {
                continue;
            }
            *c = c.map(|ch, v| v * self.mul[ch] + self.add[ch]);
        }
        let backdrop = cg.colors[0].map(|ch, v| v * self.mul[ch] + self.add[ch]);
        cg.colors[0] = backdrop;
    }
}

/// Rotates the water colour ranges for shimmering caustics and rolling swell.
pub fn cycle_water(cg: &mut Cgram, clock: u32) {
    for (k, (start, len)) in water_cycles().into_iter().enumerate() {
        // Sea shimmer, deep swell, river current (fastest).
        let speed = [9, 14, 3][k];
        cg.rotate(start, len, (clock / speed) as usize);
    }
}

/// Colour subtracted at full darkness (5-bit units). Red/green go first so
/// the little that survives reads as moonlit blue.
const DARK: [f32; 3] = [23.0, 20.0, 13.0];
/// Darkness of light-mask levels 1 (centre) to 3 (rim), relative to DARK.
const MASK_LEVELS: [f32; 3] = [0.12, 0.38, 0.66];

/// Where the light mask sits in the BG3 map (px) and its centre.
pub const LIGHT_MASK_ORIGIN: (i32, i32) = (256, 256);
const LIGHT_MASK_CENTRE: i32 = (crate::assets::hud_art::LIGHT_MASK_TILES * 4) as i32;

/// A light source in screen space.
#[derive(Clone, Copy, Debug)]
pub struct Light {
    pub center: (i32, i32),
}

/// Night darkness. Colour math subtracts the sub-screen from the scene.
/// Below the HUD, HDMA switches BG3 from the main screen to the sub-screen
/// and scrolls it so the radial light mask sits on the light source; where
/// the mask is empty (or with no light at all) the fixed colour — full
/// darkness — is subtracted instead.
pub fn night(lines: &mut LineTables, cg: &mut Cgram, light: Option<Light>, hud_lines: usize, strength: f32, clock: u32) {
    lines.clear();
    let scale = |k: f32| {
        Rgb555::new(
            (DARK[0] * k * strength).round() as u8,
            (DARK[1] * k * strength).round() as u8,
            (DARK[2] * k * strength).round() as u8,
        )
    };
    lines.fixed_color.fill(scale(1.0));

    // A flame flickers: nudge the inner levels a little every frame.
    let flicker = 1.0 + ((clock as f32 * 0.23).sin() + (clock as f32 * 0.61).sin() * 0.5) * 0.08;
    let mask = [scale(MASK_LEVELS[0] * flicker), scale(MASK_LEVELS[1] * flicker), scale(MASK_LEVELS[2])];
    cg.set_hud_palette(HUD_LIGHT_MASK, mask);

    let Some(Light { center: (cx, cy) }) = light else { return };
    // BG3 wraps, so a mask scrolled far enough repeats a map's width away:
    // a light whose mask misses the screen would cast a copy elsewhere.
    // (One that reaches the screen can't: its copies lie a whole map
    // width, more than a screen, away.)
    let r = LIGHT_MASK_CENTRE;
    if cx + r <= 0 || cx - r >= SCREEN_W as i32 || cy + r <= hud_lines as i32 || cy - r >= SCREEN_H as i32 {
        return;
    }
    let scroll = (LIGHT_MASK_ORIGIN.0 + LIGHT_MASK_CENTRE - cx, LIGHT_MASK_ORIGIN.1 + LIGHT_MASK_CENTRE - cy);
    let mask_rows = LIGHT_MASK_ORIGIN.1..LIGHT_MASK_ORIGIN.1 + 2 * LIGHT_MASK_CENTRE;
    for y in hud_lines..SCREEN_H {
        lines.bg3_main[y] = false;
        // Only lines that cross the mask take BG3 as the sub-screen. Any
        // other line would show whatever its scroll lands on — far from the
        // light it wraps round the BG3 map onto the HUD — so it's left to
        // the fixed colour: full darkness.
        if mask_rows.contains(&(scroll.1 + y as i32)) {
            lines.bg3_sub[y] = true;
            lines.bg3_scroll[y] = scroll;
        }
    }
}

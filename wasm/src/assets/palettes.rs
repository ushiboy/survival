//! CGRAM allocation and colour ramps.
//!
//! Background palettes 0-1 are shared with BG3 (HUD), so terrain uses 2-7.
//! Each palette module documents what its indices mean; generators refer to
//! these names rather than raw numbers.

use super::character::Outfit;
use crate::ppu::palette::{Cgram, Rgb555};

const fn c(r: u8, g: u8, b: u8) -> Rgb555 {
    Rgb555::rgb8(r, g, b)
}

// ---------------------------------------------------------------- BG palette numbers
pub const PAL_SHORE: u8 = 2;
pub const PAL_DEEP: u8 = 3;
pub const PAL_SAND: u8 = 4;
pub const PAL_GRASS: u8 = 5;
pub const PAL_FOLIAGE: u8 = 6;
pub const PAL_ROCK: u8 = 7;

// ---------------------------------------------------------------- OBJ palette numbers
pub const OBJ_PLAYER: u8 = 0;
pub const OBJ_CRAB: u8 = 1;
pub const OBJ_GULL: u8 = 2;
/// Tools and effects: axe, wood chips, leaves, sparks.
pub const OBJ_FX: u8 = 3;
/// Palettes 4+ are blended by colour math.
pub const OBJ_SHADOW: u8 = 4;
pub const OBJ_WATER_FX: u8 = 5;
pub const OBJ_WADE: u8 = 6;
/// Copy of the crab palette in the colour-math range, used at night.
pub const OBJ_CRAB_NIGHT: u8 = 7;

// ---------------------------------------------------------------- HUD (BG3) palette numbers
pub const HUD_TEXT: u8 = 0;
pub const HUD_HEART: u8 = 1;
pub const HUD_FOOD: u8 = 2;
pub const HUD_WATER: u8 = 3;
pub const HUD_SKY: u8 = 4;
/// Greyed-out text (tabs not shown, things out of reach).
pub const HUD_DIM: u8 = 5;
/// Bag screen slots: fill, shadowed rim, lit rim.
pub const HUD_MENU: u8 = 6;
/// Darkness levels of the light mask (written each frame).
pub const HUD_LIGHT_MASK: u8 = 7;

/// Smoke rising from a signal fire, in the water effects palette (drawn
/// half see-through by day).
pub mod smoke {
    pub const LIGHT: u8 = 4;
    pub const MID: u8 = 5;
    pub const SHADE: u8 = 6;
}

/// Shallows and surf (shared water/sand so the shoreline fits one palette).
pub mod shore {
    /// Water, deep → light.
    pub const WATER: [u8; 4] = [1, 2, 3, 4];
    /// Colours rotated every few frames for shimmering caustics.
    pub const CYCLE: [u8; 3] = [5, 6, 7];
    pub const FOAM: u8 = 8;
    pub const FOAM_SHADE: u8 = 9;
    pub const WET_DARK: u8 = 10;
    pub const WET: u8 = 11;
    pub const SAND_DARK: u8 = 12;
    pub const SAND: u8 = 13;
    pub const SAND_LIGHT: u8 = 14;
    pub const SEABED: u8 = 15;
}

/// Open sea.
pub mod deep {
    pub const WATER: [u8; 4] = [1, 2, 3, 4];
    /// Same colours as `shore::WATER[0..2]`, for seamless transitions.
    pub const SHELF: [u8; 2] = [5, 6];
    pub const CYCLE: [u8; 3] = [7, 8, 9];
    pub const CREST: u8 = 10;
    /// River water (drawn on BG2): shade under the banks, the body, and a
    /// fast colour cycle that runs streaks of current downstream.
    pub const RIVER_SHADE: u8 = 11;
    pub const RIVER_BODY: u8 = 12;
    /// Rotated so each colour steps downstream: body, mid, bright.
    pub const RIVER_FLOW: [u8; 3] = [13, 14, 15];
}

/// Dry beach.
pub mod sand {
    pub const DEBRIS_DARK: u8 = 1;
    pub const DEBRIS: u8 = 2;
    /// Sand, dark → light.
    pub const RAMP: [u8; 3] = [3, 4, 5];
    pub const GLINT: u8 = 6;
    pub const SHADOW: [u8; 2] = [8, 7];
    pub const SHELL: [u8; 3] = [11, 9, 10];
    pub const WET: u8 = 12;
    pub const PEBBLE: [u8; 3] = [13, 14, 15];
}

pub mod grass {
    /// Greens, darkest → lightest.
    pub const RAMP: [u8; 7] = [1, 2, 3, 4, 5, 6, 7];
    pub const SAND: [u8; 3] = [8, 9, 10];
    pub const YELLOW: u8 = 11;
    pub const WHITE: u8 = 12;
    pub const PINK: u8 = 13;
    pub const RED: u8 = 14;
    pub const SOIL: u8 = 15;
}

/// Tree crowns & trunks, plus sand for palm trunks' base tiles. The leaf
/// greens are the grass greens at the same indices, so grass tiles show
/// through unchanged under broadleaf trunks.
pub mod foliage {
    pub const LEAF: [u8; 7] = [1, 2, 3, 4, 5, 6, 7];
    /// Trunk bark, dark → light.
    pub const BARK: [u8; 4] = [8, 9, 10, 11];
    pub const COCONUT: u8 = 12;
    pub const SAND_SHADOW: u8 = 13;
    pub const SAND: [u8; 2] = [14, 15];
}

/// Stone and wood on sand.
pub mod rock {
    pub const STONE: [u8; 6] = [1, 2, 3, 4, 5, 6];
    pub const MOSS: u8 = 7;
    pub const WOOD: [u8; 4] = [8, 9, 10, 11];
    pub const SAND_SHADOW: u8 = 12;
    pub const SAND: [u8; 3] = [13, 14, 15];
}

/// Shared sand colours (identical RGB in every palette that shows sand).
const SAND_DARK: Rgb555 = c(206, 174, 122);
const SAND_MID: Rgb555 = c(230, 202, 148);
const SAND_LIGHT: Rgb555 = c(246, 228, 184);
const SAND_SHADOW: Rgb555 = c(170, 136, 100);
/// Grass and foliage share their greens, so tree trunks (foliage palette)
/// can stand on grass tiles without a seam.
const GREENS: [Rgb555; 7] = [c(20, 52, 36), c(32, 78, 40), c(48, 110, 44), c(72, 142, 52), c(104, 172, 64), c(146, 202, 84), c(194, 228, 120)];
const WATER_EDGE: [Rgb555; 2] = [c(36, 116, 178), c(52, 150, 198)];

pub fn base_cgram() -> Cgram {
    let mut cg = Cgram::default();
    cg.colors[0] = c(8, 24, 56); // backdrop

    // HUD (BG3, 2bpp)
    cg.set_hud_palette(HUD_TEXT, [c(16, 16, 40), c(176, 186, 214), c(252, 252, 252)]);
    set_health_palette(&mut cg, false);
    cg.set_hud_palette(HUD_FOOD, [c(48, 24, 8), c(228, 128, 32), c(252, 216, 96)]);
    cg.set_hud_palette(HUD_WATER, [c(8, 24, 56), c(56, 136, 232), c(156, 220, 252)]);
    cg.set_hud_palette(HUD_DIM, [c(10, 14, 32), c(70, 80, 112), c(104, 116, 150)]);
    cg.set_hud_palette(HUD_MENU, [c(20, 30, 58), c(8, 12, 28), c(70, 86, 128)]);
    cg.set_hud_palette(HUD_SKY, [c(64, 40, 16), c(248, 184, 40), c(255, 244, 168)]);

    cg.set_bg_palette(
        PAL_SHORE,
        &[
            WATER_EDGE[0],
            WATER_EDGE[1],
            c(70, 182, 210),
            c(104, 202, 218),
            c(120, 210, 222),
            c(156, 226, 230),
            c(200, 242, 240),
            c(250, 254, 252),
            c(196, 230, 238),
            c(188, 156, 110),
            c(204, 176, 128),
            SAND_DARK,
            SAND_MID,
            SAND_LIGHT,
            c(124, 196, 196),
        ],
    );
    cg.set_bg_palette(
        PAL_DEEP,
        &[
            c(10, 34, 82),
            c(16, 54, 112),
            c(22, 74, 140),
            c(30, 96, 164),
            WATER_EDGE[0],
            WATER_EDGE[1],
            c(40, 92, 156),
            c(76, 140, 196),
            c(150, 204, 236),
            c(214, 240, 250),
            c(22, 60, 82),
            c(40, 98, 128),
            c(46, 110, 140),
            c(98, 166, 190),
            c(186, 228, 236),
        ],
    );
    cg.set_bg_palette(
        PAL_SAND,
        &[
            c(112, 80, 56),
            c(160, 120, 80),
            SAND_DARK,
            SAND_MID,
            SAND_LIGHT,
            c(255, 248, 224),
            c(190, 156, 112),
            SAND_SHADOW,
            c(240, 168, 150),
            c(255, 222, 204),
            c(184, 104, 100),
            c(192, 164, 116),
            c(112, 104, 100),
            c(160, 154, 144),
            c(206, 200, 188),
        ],
    );
    cg.set_bg_palette(
        PAL_GRASS,
        &[
            GREENS[0],
            GREENS[1],
            GREENS[2],
            GREENS[3],
            GREENS[4],
            GREENS[5],
            GREENS[6],
            SAND_DARK,
            SAND_MID,
            SAND_LIGHT,
            c(252, 216, 72),
            c(250, 250, 240),
            c(244, 132, 168),
            c(204, 52, 72),
            c(116, 84, 56),
        ],
    );
    cg.set_bg_palette(
        PAL_FOLIAGE,
        &[
            GREENS[0],
            GREENS[1],
            GREENS[2],
            GREENS[3],
            GREENS[4],
            GREENS[5],
            GREENS[6],
            c(64, 42, 30),
            c(104, 74, 46),
            c(148, 108, 62),
            c(194, 156, 98),
            c(110, 74, 36),
            SAND_SHADOW,
            SAND_MID,
            SAND_LIGHT,
        ],
    );
    cg.set_bg_palette(
        PAL_ROCK,
        &[
            c(44, 42, 54),
            c(76, 74, 84),
            c(110, 106, 112),
            c(146, 142, 140),
            c(182, 178, 170),
            c(220, 216, 204),
            c(98, 128, 60),
            c(84, 60, 44),
            c(128, 94, 62),
            c(170, 132, 90),
            c(208, 178, 132),
            SAND_SHADOW,
            SAND_DARK,
            SAND_MID,
            SAND_LIGHT,
        ],
    );

    // Sprites
    set_player_palettes(&mut cg, &Outfit::CASTAWAY, false);
    let crab = [
        c(56, 16, 20),
        c(136, 32, 28),
        c(200, 64, 40),
        c(240, 120, 72),
        c(252, 186, 130),
        c(248, 248, 248),
        c(24, 8, 8),
        // Items share the crab palettes (ITEM_LEGEND).
        c(40, 26, 18),
        c(92, 60, 34),
        c(140, 96, 52),
        c(186, 140, 84),
        c(228, 196, 140),
        // Coconut flesh, shaded and lit (also the light faces of stones).
        c(214, 206, 184),
        c(250, 248, 236),
        // Stone shade.
        c(122, 118, 112),
    ];
    cg.set_obj_palette(OBJ_CRAB, &crab);
    cg.set_obj_palette(OBJ_CRAB_NIGHT, &crab);
    cg.set_obj_palette(
        OBJ_GULL,
        &[
            c(44, 44, 60),
            c(112, 120, 142),
            c(170, 178, 196),
            c(232, 238, 246),
            c(255, 255, 255),
            c(244, 176, 48),
            c(176, 96, 24),
            c(32, 32, 44),
            // Lauan, the reddish hardwood (LAUAN_LEGEND, with the gull's outline).
            c(84, 40, 30),
            c(138, 70, 50),
            c(178, 100, 70),
            c(214, 146, 110),
        ],
    );
    cg.set_obj_palette(
        OBJ_FX,
        &[
            c(40, 24, 28), // o outline
            c(108, 68, 40),
            c(162, 112, 62), // w/W wood
            c(88, 94, 110),
            c(160, 168, 182),
            c(232, 238, 246), // m/M/L metal
            c(226, 186, 128), // c fresh-cut wood
            c(36, 96, 44),
            c(88, 158, 58),
            c(164, 212, 92), // g/G/h leaf
            c(255, 226, 112),
            c(255, 255, 240), // y/Y spark
            c(40, 112, 188),
            c(92, 172, 228),
            c(206, 240, 252), // b/l/i water
        ],
    );
    cg.set_obj_palette(
        OBJ_SHADOW,
        &[
            c(20, 28, 56),
            c(0, 0, 0),
            c(0, 0, 0),
            // Black smoke off a signal fire with fish oil on it, at the
            // same places as the pale smoke's (`smoke`): lit, mid, shaded.
            c(92, 86, 80),
            c(58, 54, 50),
            c(30, 28, 28),
        ],
    );
    cg.set_obj_palette(
        OBJ_WATER_FX,
        &[
            c(176, 232, 244),
            c(250, 255, 255),
            c(110, 190, 222),
            // Signal fire smoke (`smoke`): lit, mid, shaded.
            c(240, 240, 234),
            c(196, 196, 190),
            c(146, 146, 144),
        ],
    );
    cg
}

/// Skin gone pale and greyish (an upset stomach): dark, base, light.
const PALE_SKIN: [Rgb555; 3] = [c(120, 120, 120), c(184, 190, 180), c(224, 230, 216)];

/// Writes the castaway's palette for an outfit (`pale` with an upset
/// stomach: the skin drained of colour), plus the derived palette for
/// submerged legs (pulled toward sea green, then blended by colour math).
/// The heart and the health gauge: red, or a poisonous purple while
/// something ails the castaway.
pub fn set_health_palette(cg: &mut Cgram, ailing: bool) {
    let colors = if ailing { [c(40, 8, 56), c(156, 52, 208), c(224, 164, 255)] } else { [c(48, 8, 24), c(220, 40, 64), c(255, 160, 168)] };
    cg.set_hud_palette(HUD_HEART, colors);
}

pub fn set_player_palettes(cg: &mut Cgram, outfit: &Outfit, pale: bool) {
    let mut colors = outfit.palette();
    if pale {
        let skin = crate::assets::character::slot::SKIN[0] as usize - 1;
        colors[skin..skin + 3].copy_from_slice(&PALE_SKIN);
    }
    cg.set_obj_palette(OBJ_PLAYER, &colors);
    cg.set_obj_palette(OBJ_WADE, &colors.map(|p| p.lerp(c(60, 170, 200), 0.45)));
}

pub const CRAB_LEGEND: &[(char, u8)] = &[('o', 1), ('r', 2), ('R', 3), ('q', 4), ('l', 5), ('w', 6), ('e', 7)];
pub const ITEM_LEGEND: &[(char, u8)] = &[('o', 8), ('d', 9), ('w', 10), ('W', 11), ('h', 12), ('F', 13), ('f', 14), ('s', 15)];
pub const FX_LEGEND: &[(char, u8)] = &[
    ('o', 1),
    ('w', 2),
    ('W', 3),
    ('m', 4),
    ('M', 5),
    ('L', 6),
    ('c', 7),
    ('g', 8),
    ('G', 9),
    ('h', 10),
    ('y', 11),
    ('Y', 12),
    ('b', 13),
    ('l', 14),
    ('i', 15),
];
/// Lauan timber shares the gull palette: the same letters as `ITEM_LEGEND`'s
/// wood, so wood art can be recoloured by swapping legends.
pub const LAUAN_LEGEND: &[(char, u8)] = &[('o', 1), ('d', 9), ('w', 10), ('W', 11), ('h', 12)];
pub const GULL_LEGEND: &[(char, u8)] = &[('o', 1), ('d', 2), ('g', 3), ('w', 4), ('W', 5), ('y', 6), ('Y', 7), ('k', 8)];

/// Index ranges rotated for water animation: (CGRAM start, length).
pub fn water_cycles() -> [(usize, usize); 3] {
    [
        (Cgram::bg_index(PAL_SHORE, shore::CYCLE[0]), shore::CYCLE.len()),
        (Cgram::bg_index(PAL_DEEP, deep::CYCLE[0]), deep::CYCLE.len()),
        (Cgram::bg_index(PAL_DEEP, deep::RIVER_FLOW[0]), deep::RIVER_FLOW.len()),
    ]
}

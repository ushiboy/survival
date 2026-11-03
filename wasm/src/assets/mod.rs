//! Loads all artwork into VRAM banks and exposes typed handles to it.
//! Gameplay code refers to graphics only through these ids.

pub mod bg_art;
pub mod font;
pub mod palettes;
pub mod sprite_art;

use crate::ppu::Vram;
use crate::ppu::banks::{MetatileId, TileId};
use crate::ppu::pattern::PixelArt;
use palettes::*;

/// Frame periods for animated water (in video frames).
pub const WATER_PERIOD: u32 = 24;
pub const SURF_PERIOD: u32 = 12;

#[derive(Clone, Copy, Debug)]
pub struct PlayerFrame {
    pub normal: [TileId; 4],
    /// Same frame with the legs hidden (used while wading).
    pub wade: [TileId; 4],
}

#[derive(Clone, Copy, Debug)]
pub struct PlayerSprites {
    /// `[stand, step]` for each facing; left is `side` flipped.
    pub down: [PlayerFrame; 2],
    pub up: [PlayerFrame; 2],
    pub side: [PlayerFrame; 2],
}

#[derive(Clone, Copy, Debug)]
pub struct SpriteIds {
    pub player: PlayerSprites,
    pub crab: [[TileId; 4]; 2],
    pub ripple: [[TileId; 2]; 2],
    pub heart_full: TileId,
    pub heart_empty: TileId,
}

#[derive(Clone, Debug)]
pub struct HudTiles {
    pub blank: TileId,
    glyphs: Vec<(char, TileId)>,
    pub food_icon: TileId,
    pub water_icon: TileId,
    pub food_full: TileId,
    pub food_empty: TileId,
    pub water_full: TileId,
    pub water_empty: TileId,
}

impl HudTiles {
    pub fn glyph(&self, c: char) -> TileId {
        self.glyphs
            .iter()
            .find(|(g, _)| *g == c)
            .map(|(_, t)| *t)
            .unwrap_or(self.blank)
    }
}

/// Base terrain images, kept so the autotiler can compose transition tiles.
#[derive(Clone, Debug)]
pub struct TerrainArt {
    pub deep: Vec<PixelArt>,
    pub shallow: Vec<PixelArt>,
    pub grass: PixelArt,
}

#[derive(Clone, Copy, Debug)]
pub struct BgTiles {
    pub deep: MetatileId,
    pub shallow: MetatileId,
    pub sand: [MetatileId; 2],
    pub grass: [MetatileId; 2],
    pub flowers: MetatileId,
    pub bush: MetatileId,
    pub rock: MetatileId,
    pub driftwood: MetatileId,
    pub shell: MetatileId,
    /// TL, TR, BL, BR
    pub palm: [MetatileId; 4],
}

pub struct Assets {
    pub vram: Vram,
    pub sprites: SpriteIds,
    pub hud: HudTiles,
    pub bg: BgTiles,
    pub terrain_art: TerrainArt,
}

impl Assets {
    pub fn load() -> Self {
        let mut vram = Vram::default();
        let terrain_art = TerrainArt {
            deep: bg_art::deep_water_frames(),
            shallow: bg_art::shallow_water_frames(),
            grass: bg_art::grass_plain(),
        };
        let bg = load_bg(&mut vram, &terrain_art);
        let sprites = load_sprites(&mut vram);
        let hud = load_hud(&mut vram);
        Self { vram, sprites, hud, bg, terrain_art }
    }
}

fn load_bg(vram: &mut Vram, art: &TerrainArt) -> BgTiles {
    let mt = &mut vram.metatiles;
    let frames = |v: &[PixelArt]| v.iter().map(|a| a.to_tile16()).collect::<Vec<_>>();
    let (canopy, trunk) = bg_art::palm_left();
    BgTiles {
        deep: mt.add(BG_DEEP, frames(&art.deep), WATER_PERIOD),
        shallow: mt.add(BG_SHORE, frames(&art.shallow), WATER_PERIOD),
        sand: [
            mt.add_art(BG_SAND, &bg_art::sand_plain()),
            mt.add_art(BG_SAND, &bg_art::sand_ripple()),
        ],
        grass: [
            mt.add_art(BG_VEG, &art.grass),
            mt.add_art(BG_VEG, &bg_art::grass_tufts()),
        ],
        flowers: mt.add_art(BG_VEG, &bg_art::flowers()),
        bush: mt.add_art(BG_VEG, &bg_art::bush()),
        rock: mt.add_art(BG_SAND, &bg_art::rock()),
        driftwood: mt.add_art(BG_SAND, &bg_art::driftwood()),
        shell: mt.add_art(BG_SAND, &bg_art::shell()),
        palm: [
            mt.add_art(BG_VEG, &canopy),
            mt.add_art(BG_VEG, &canopy.flipped_h()),
            mt.add_art(BG_SAND, &trunk),
            mt.add_art(BG_SAND, &trunk.flipped_h()),
        ],
    }
}

/// Legs start at this row of the 16x16 player art; hidden while wading.
const WADE_CUT_ROW: usize = 12;

fn load_sprites(vram: &mut Vram) -> SpriteIds {
    let bank = &mut vram.sprites;
    let mut frame = |art: PixelArt| PlayerFrame {
        normal: bank.add_16x16(&art),
        wade: bank.add_16x16(&art.cleared_below(WADE_CUT_ROW)),
    };
    let player = PlayerSprites {
        down: [frame(sprite_art::player_down_stand()), frame(sprite_art::player_down_step())],
        up: [frame(sprite_art::player_up_stand()), frame(sprite_art::player_up_step())],
        side: [frame(sprite_art::player_side_stand()), frame(sprite_art::player_side_step())],
    };
    let [c0, c1] = sprite_art::crab_frames();
    let [r0, r1] = sprite_art::wade_ripple_frames();
    SpriteIds {
        player,
        crab: [bank.add_16x16(&c0), bank.add_16x16(&c1)],
        ripple: [bank.add_16x8(&r0), bank.add_16x8(&r1)],
        heart_full: bank.add(sprite_art::heart_full().to_tile8()),
        heart_empty: bank.add(sprite_art::heart_empty().to_tile8()),
    }
}

/// Palette index used for HUD text (white in BG_SHORE).
pub const HUD_TEXT_PALETTE: u8 = BG_SHORE;

fn load_hud(vram: &mut Vram) -> HudTiles {
    let bank = &mut vram.hud_tiles;
    let blank = bank.add(PixelArt::filled(8, 8, 0).to_tile8());
    let glyphs = font::GLYPHS
        .iter()
        .map(|(c, rows)| (*c, bank.add(font::glyph_art(rows, 2).to_tile8())))
        .collect();
    HudTiles {
        blank,
        glyphs,
        food_icon: bank.add(font::icon_food().to_tile8()),
        water_icon: bank.add(font::icon_water().to_tile8()),
        food_full: bank.add(font::gauge_segment(2, true).to_tile8()),
        food_empty: bank.add(font::gauge_segment(3, false).to_tile8()),
        water_full: bank.add(font::gauge_segment(1, true).to_tile8()),
        water_empty: bank.add(font::gauge_segment(1, false).to_tile8()),
    }
}

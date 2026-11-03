//! Loads all artwork into VRAM banks and exposes typed handles to it.
//! Gameplay code refers to graphics only through these ids.

pub mod hud_art;
pub mod paint;
pub mod palettes;
pub mod props;
pub mod sprite_art;
pub mod terrain;

use crate::ppu::Vram;
use crate::ppu::banks::{HudTileId, MetatileId, SpriteId};
use crate::ppu::pattern::PixelArt;
use palettes::PAL_FOLIAGE;
use props::{PalmArt, PropTile};
use terrain::TerrainTextures;

/// Frames per step of the surf animation.
pub const SURF_PERIOD: u32 = 7;
pub const SURF_FRAMES: usize = 8;

#[derive(Clone, Copy, Debug)]
pub struct PlayerFrame {
    pub full: SpriteId,
    /// Above the waterline (wading).
    pub top: SpriteId,
    /// Below the waterline, drawn with the translucent wading palette.
    pub legs: SpriteId,
}

#[derive(Clone, Copy, Debug)]
pub struct PlayerSprites {
    /// `[stand, step]`; the second step is the mirror image.
    pub down: [PlayerFrame; 2],
    pub up: [PlayerFrame; 2],
    /// `[stand, step A, step B]`, facing right.
    pub side: [PlayerFrame; 3],
}

#[derive(Clone, Copy, Debug)]
pub struct SpriteIds {
    pub player: PlayerSprites,
    pub crab: [SpriteId; 2],
    pub gull: [SpriteId; 2],
    pub shadow_player: SpriteId,
    pub shadow_crab: SpriteId,
    pub shadow_gull: SpriteId,
    pub ripples: [SpriteId; 3],
}

#[derive(Clone, Debug)]
pub struct HudTiles {
    glyphs: Vec<(char, HudTileId)>,
    pub heart_full: HudTileId,
    pub heart_empty: HudTileId,
    pub food: HudTileId,
    pub water: HudTileId,
    pub sun: HudTileId,
    pub moon: HudTileId,
    pub lantern: HudTileId,
    /// Gauge segments indexed by filled pixels (0..=8).
    pub gauge: [HudTileId; 9],
    pub gauge_cap_left: HudTileId,
    pub gauge_cap_right: HudTileId,
    /// Lantern light mask, `LIGHT_MASK_TILES`² tiles in row-major order.
    pub light_mask: Vec<HudTileId>,
}

impl HudTiles {
    pub const BLANK: HudTileId = 0;

    pub fn glyph(&self, c: char) -> HudTileId {
        self.glyphs.iter().find(|(g, _)| *g == c).map(|(_, t)| *t).unwrap_or(Self::BLANK)
    }
}

/// Background props, composited onto ground textures by the tileset builder.
pub struct PropSet {
    pub boulder: PropTile,
    pub pebbles: PropTile,
    pub driftwood: PropTile,
    pub shell: PropTile,
    pub bush: PropTile,
    pub flowers: PropTile,
    pub palm: PalmArt,
    /// Palm canopy + upper trunk on BG2: `[TL, TR, ML, MR]`.
    pub palm_upper: [MetatileId; 4],
}

pub struct Assets {
    pub vram: Vram,
    pub sprites: SpriteIds,
    pub hud: HudTiles,
    pub textures: TerrainTextures,
    pub props: PropSet,
}

impl Assets {
    pub fn load() -> Self {
        let mut vram = Vram::default();
        let sprites = load_sprites(&mut vram);
        let hud = load_hud(&mut vram);
        let props = load_props(&mut vram);
        Self { vram, sprites, hud, textures: TerrainTextures::generate(), props }
    }
}

fn load_props(vram: &mut Vram) -> PropSet {
    let palm = props::palm();
    let mut q = |x, y| vram.metatiles.add_art(PAL_FOLIAGE, &palm.upper.sub(x, y, 16, 16));
    let palm_upper = [q(0, 0), q(16, 0), q(0, 16), q(16, 16)];
    PropSet {
        boulder: props::boulder(),
        pebbles: props::pebbles(),
        driftwood: props::driftwood(),
        shell: props::shell(),
        bush: props::bush(),
        flowers: props::flowers(),
        palm,
        palm_upper,
    }
}

fn load_sprites(vram: &mut Vram) -> SpriteIds {
    use sprite_art::*;
    let bank = &mut vram.sprites;
    let mut frame = |art: PixelArt| PlayerFrame {
        full: bank.add(&art),
        top: bank.add(&art.keep_rows(0..PLAYER_WADE_ROW)),
        legs: bank.add(&art.keep_rows(PLAYER_WADE_ROW..art.h)),
    };
    let player = PlayerSprites {
        down: [frame(player_down_stand()), frame(player_down_step())],
        up: [frame(player_up_stand()), frame(player_up_step())],
        side: [frame(player_side_stand()), frame(player_side_step_a()), frame(player_side_step_b())],
    };
    let [c0, c1] = crab_frames();
    let [g0, g1] = gull_frames();
    let [r0, r1, r2] = wade_ripples();
    SpriteIds {
        player,
        crab: [bank.add(&c0), bank.add(&c1)],
        gull: [bank.add(&g0), bank.add(&g1)],
        shadow_player: bank.add(&shadow(16, 8, 6.5, 2.6)),
        shadow_crab: bank.add(&shadow(16, 8, 6.5, 2.2)),
        shadow_gull: bank.add(&shadow(16, 8, 5.0, 1.8)),
        ripples: [bank.add(&r0), bank.add(&r1), bank.add(&r2)],
    }
}

fn load_hud(vram: &mut Vram) -> HudTiles {
    let bank = &mut vram.hud_tiles;
    let blank = bank.add(&PixelArt::filled(8, 8, 0));
    debug_assert_eq!(blank, HudTiles::BLANK);
    let glyphs = hud_art::glyphs().into_iter().map(|(c, a)| (c, bank.add(&a))).collect();
    let gauge = std::array::from_fn(|i| bank.add(&hud_art::gauge_segment(i)));
    HudTiles {
        glyphs,
        heart_full: bank.add(&hud_art::heart_full()),
        heart_empty: bank.add(&hud_art::heart_empty()),
        food: bank.add(&hud_art::icon_food()),
        water: bank.add(&hud_art::icon_water()),
        sun: bank.add(&hud_art::icon_sun()),
        moon: bank.add(&hud_art::icon_moon()),
        lantern: bank.add(&hud_art::icon_lantern()),
        gauge,
        gauge_cap_left: bank.add(&hud_art::gauge_cap(false)),
        gauge_cap_right: bank.add(&hud_art::gauge_cap(true)),
        light_mask: {
            // Identical tiles (solid centre, empty corners) share one pattern.
            let mut seen: Vec<(Vec<u8>, HudTileId)> = Vec::new();
            hud_art::light_mask()
                .into_iter()
                .map(|t| match seen.iter().find(|(px, _)| *px == t.px) {
                    Some((_, id)) => *id,
                    None => {
                        let id = bank.add(&t);
                        seen.push((t.px.clone(), id));
                        id
                    }
                })
                .collect()
        },
    }
}

//! Loads all artwork into VRAM banks and exposes typed handles to it.
//! Gameplay code refers to graphics only through these ids.

pub mod character;
pub mod hud_art;
pub mod paint;
pub mod palettes;
pub mod props;
pub mod sprite_art;
pub mod terrain;

use crate::ppu::Vram;
use crate::ppu::banks::{HudTileId, MetatileId, SpriteBank, SpriteId};
use character::Outfit;
use character::pose::{Pose, View};
use crate::ppu::pattern::PixelArt;
use palettes::PAL_FOLIAGE;
use props::{PalmArt, PropTile};
use terrain::TerrainTextures;

/// Frames per step of the surf animation.
pub const SURF_PERIOD: u32 = 7;
pub const SURF_FRAMES: usize = 8;

/// One rendered castaway frame and the pieces cut from it.
#[derive(Clone, Copy, Debug)]
pub struct CharFrame {
    pub full: SpriteId,
    /// Above the waterline (wading).
    pub top: SpriteId,
    /// Below the waterline, drawn with the translucent wading palette.
    pub legs: SpriteId,
    /// Just the hands, drawn over a held tool.
    pub hands: SpriteId,
}

/// All castaway frames for the current outfit. Views are indexed
/// Front, Back, Side (facing right; left is mirrored).
#[derive(Clone, Debug)]
pub struct PlayerSprites {
    pub walk: [[CharFrame; 4]; 3],
    pub chop: [[CharFrame; 3]; 3],
    pub chop_poses: [[Pose; 3]; 3],
}

pub const VIEWS: [View; 3] = [View::Front, View::Back, View::Side];

fn render_frame(pose: &Pose, outfit: &Outfit) -> [PixelArt; 4] {
    let r = character::render::render(pose, outfit);
    let wade = character::WADE_ROW;
    [r.body.clone(), r.body.keep_rows(0..wade), r.body.keep_rows(wade..r.body.h), r.hands]
}

impl PlayerSprites {
    fn poses() -> ([[Pose; 4]; 3], [[Pose; 3]; 3]) {
        (
            VIEWS.map(|v| std::array::from_fn(|i| character::pose::walk(v, i))),
            VIEWS.map(|v| std::array::from_fn(|i| character::pose::chop(v, i))),
        )
    }

    pub fn build(bank: &mut SpriteBank, outfit: &Outfit) -> Self {
        let (walk, chop) = Self::poses();
        let mut add = |pose: &Pose| {
            let [full, top, legs, hands] = render_frame(pose, outfit).map(|a| bank.add(&a));
            CharFrame { full, top, legs, hands }
        };
        Self { walk: walk.map(|v| v.map(|p| add(&p))), chop: chop.map(|v| v.map(|p| add(&p))), chop_poses: chop }
    }

    /// Re-renders every frame in place for a new outfit (equipment change).
    pub fn rebuild(&self, bank: &mut SpriteBank, outfit: &Outfit) {
        let (walk, chop) = Self::poses();
        let pairs = walk.iter().flatten().zip(self.walk.iter().flatten()).chain(chop.iter().flatten().zip(self.chop.iter().flatten()));
        for (pose, f) in pairs {
            let arts = render_frame(pose, outfit);
            for (id, art) in [f.full, f.top, f.legs, f.hands].into_iter().zip(arts.iter()) {
                bank.replace(id, art);
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct SpriteIds {
    pub player: PlayerSprites,
    pub crab: [SpriteId; 2],
    pub gull: [SpriteId; 2],
    pub shadow_player: SpriteId,
    pub shadow_crab: SpriteId,
    pub shadow_gull: SpriteId,
    pub ripples: [SpriteId; 3],
    pub axe_side: SpriteId,
    pub axe_up: SpriteId,
    pub axe_diag: SpriteId,
    /// Swoosh arcs indexed by `Dir as usize` order: up, down, left, right.
    pub swoosh: [SpriteId; 4],
    pub chip: [SpriteId; 2],
    pub leaf: [SpriteId; 2],
    pub spark: [SpriteId; 2],
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
    pub wood: HudTileId,
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
    /// Stump left after felling a palm.
    pub stump: PropTile,
    /// Palm canopy + upper trunk on BG2, 3x3 cells in row-major order.
    pub palm_upper: [MetatileId; 9],
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
    let palm_upper = std::array::from_fn(|i| vram.metatiles.add_art(PAL_FOLIAGE, &palm.upper.sub(i % 3 * 16, i / 3 * 16, 16, 16)));
    PropSet {
        boulder: props::boulder(),
        pebbles: props::pebbles(),
        driftwood: props::driftwood(),
        shell: props::shell(),
        bush: props::bush(),
        flowers: props::flowers(),
        palm,
        stump: props::stump(),
        palm_upper,
    }
}

fn load_sprites(vram: &mut Vram) -> SpriteIds {
    use sprite_art::*;
    let bank = &mut vram.sprites;
    let player = PlayerSprites::build(bank, &Outfit::CASTAWAY);
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
        axe_side: bank.add(&axe_side()),
        axe_up: bank.add(&axe_up()),
        axe_diag: bank.add(&axe_diag()),
        swoosh: swooshes().map(|a| bank.add(&a)),
        chip: chip_frames().map(|a| bank.add(&a)),
        leaf: leaf_frames().map(|a| bank.add(&a)),
        spark: spark_frames().map(|a| bank.add(&a)),
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
        wood: bank.add(&hud_art::icon_wood()),
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

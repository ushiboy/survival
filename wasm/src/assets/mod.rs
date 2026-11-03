//! Loads all artwork into VRAM banks and exposes typed handles to it.
//! Gameplay code refers to graphics only through these ids.

pub mod character;
pub mod hud_art;
pub mod item_art;
pub mod paint;
pub mod palettes;
pub mod props;
pub mod sprite_art;
pub mod terrain;

use crate::ppu::Vram;
use crate::ppu::banks::{HudTileId, MetatileId, SpriteBank, SpriteId};
use crate::items::ItemKind;
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
    /// Shaking a tree trunk: push, pull.
    pub shake: [[SpriteId; 2]; 3],
    /// Crouching to pick something up (or to scoop water).
    pub crouch: [SpriteId; 3],
    /// Drinking from cupped hands.
    pub drink: [SpriteId; 3],
    /// Waking up on the beach: lying (eyes closed, open; 32x24, head left),
    /// sitting up, kneeling (24x32, facing right).
    pub lying: [SpriteId; 2],
    pub sitting: SpriteId,
    pub kneeling: SpriteId,
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
        let walk_frames = walk.map(|v| v.map(|p| add(&p)));
        let chop_frames = chop.map(|v| v.map(|p| add(&p)));
        let [lying, sitting, kneeling] = Self::wake_art(outfit);
        let lying = lying.map(|a| bank.add(&a));
        Self {
            walk: walk_frames,
            chop: chop_frames,
            chop_poses: chop,
            shake: Self::shake_art(outfit).map(|v| v.map(|a| bank.add(&a))),
            crouch: Self::crouch_art(outfit).map(|a| bank.add(&a)),
            drink: Self::drink_art(outfit).map(|a| bank.add(&a)),
            lying,
            sitting: bank.add(&sitting[0]),
            kneeling: bank.add(&kneeling[0]),
        }
    }

    fn shake_art(outfit: &Outfit) -> [[PixelArt; 2]; 3] {
        VIEWS.map(|v| std::array::from_fn(|i| character::render::render(&character::pose::shake(v, i), outfit).body))
    }

    fn crouch_art(outfit: &Outfit) -> [PixelArt; 3] {
        VIEWS.map(|v| character::render::render(&character::pose::crouch(v), outfit).body)
    }

    fn drink_art(outfit: &Outfit) -> [PixelArt; 3] {
        VIEWS.map(|v| character::render::render(&character::pose::drink(v), outfit).body)
    }

    fn wake_art(outfit: &Outfit) -> [[PixelArt; 2]; 3] {
        use character::pose::{kneeling, sitting};
        use character::render::render;
        let blank = PixelArt::filled(8, 8, 0);
        [
            [character::render_lying(outfit, false), character::render_lying(outfit, true)],
            [render(&sitting(), outfit).body, blank.clone()],
            [render(&kneeling(), outfit).body, blank],
        ]
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
        for (ids, arts) in self.shake.iter().zip(Self::shake_art(outfit)) {
            for (id, art) in ids.iter().zip(arts.iter()) {
                bank.replace(*id, art);
            }
        }
        for (id, art) in self.crouch.iter().zip(Self::crouch_art(outfit)).chain(self.drink.iter().zip(Self::drink_art(outfit))) {
            bank.replace(*id, &art);
        }
        let [lying, sitting, kneeling] = Self::wake_art(outfit);
        bank.replace(self.lying[0], &lying[0]);
        bank.replace(self.lying[1], &lying[1]);
        bank.replace(self.sitting, &sitting[0]);
        bank.replace(self.kneeling, &kneeling[0]);
    }
}

/// Bag-screen icons and cursor.
#[derive(Clone, Debug)]
pub struct BagSprites {
    pub coconut: SpriteId,
    pub timber: SpriteId,
    pub raw_water: SpriteId,
    /// Upright, turned.
    pub stone_axe: [SpriteId; 2],
    /// Upright, turned.
    pub branch: [SpriteId; 2],
    pub stone: SpriteId,
    pub vine: SpriteId,
    /// Top-left bracket; flipped for the other corners.
    pub corner: SpriteId,
}

impl BagSprites {
    /// The icon for an item as laid in the bag, and its sprite palette.
    pub fn icon(&self, kind: ItemKind, turned: bool) -> (SpriteId, u8) {
        match kind {
            ItemKind::Coconut => (self.coconut, palettes::OBJ_CRAB),
            ItemKind::Timber => (self.timber, palettes::OBJ_CRAB),
            ItemKind::RawWater => (self.raw_water, palettes::OBJ_FX),
            ItemKind::StoneAxe => (self.stone_axe[turned as usize], palettes::OBJ_FX),
            ItemKind::Branch => (self.branch[turned as usize], palettes::OBJ_CRAB),
            ItemKind::Stone => (self.stone, palettes::OBJ_CRAB),
            ItemKind::Vine => (self.vine, palettes::OBJ_FX),
        }
    }
}

#[derive(Clone, Debug)]
pub struct SpriteIds {
    pub bag: BagSprites,
    pub player: PlayerSprites,
    pub crab: [SpriteId; 2],
    pub gull: [SpriteId; 2],
    pub shadow_player: SpriteId,
    /// Under the castaway while lying on the sand.
    pub shadow_lying: SpriteId,
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
    /// A coconut: resting, and turned while tumbling.
    pub coconut: [SpriteId; 2],
    pub shadow_coconut: SpriteId,
    /// Water carried in cupped hands to drink: level, tipped.
    pub handful: [SpriteId; 2],
    /// A coconut being eaten, after 1, 2 and 3 bites.
    pub coconut_bitten: [SpriteId; 3],
    pub timber: SpriteId,
    /// Ground items: a dead branch, a stone, a coiled vine.
    pub branch: SpriteId,
    pub stone: SpriteId,
    pub vine_coil: SpriteId,
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
    /// A bag cell's four 8x8 quarters: top-left, top-right, bottom-left,
    /// bottom-right.
    pub bag_cell: [HudTileId; 4],
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
    pub vine: PropTile,
    pub vine_stake: PropTile,
    pub palm: PalmArt,
    /// Stump left after felling a palm.
    pub stump: PropTile,
    /// Palm canopy tiles for each sway offset (`dx + PALM_SWAY_MAX`).
    pub palm_tiles: [PalmTiles; 2 * PALM_SWAY_MAX as usize + 1],
}

/// Furthest a canopy shifts (px) when the tree sways.
pub const PALM_SWAY_MAX: i32 = 2;

/// Palm canopy + upper trunk on BG2, shifted by one sway offset.
pub struct PalmTiles {
    /// 3x3 cells in row-major order (no coconuts).
    pub upper: [MetatileId; 9],
    /// The centre cell with 0..=PALM_COCONUTS coconuts hanging.
    pub crowns: [MetatileId; props::PALM_COCONUTS + 1],
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
    // Swaying is tile animation: each offset has its own pre-shifted
    // tiles, swapped into just the swaying tree's cells.
    let palm_tiles = std::array::from_fn(|i| {
        let dx = i as i32 - PALM_SWAY_MAX;
        let upper = palm.swayed(0, dx);
        PalmTiles {
            upper: std::array::from_fn(|c| vram.metatiles.add_art(PAL_FOLIAGE, &upper.sub(c % 3 * 16, c / 3 * 16, 16, 16))),
            crowns: std::array::from_fn(|n| vram.metatiles.add_art(PAL_FOLIAGE, &palm.swayed(n, dx).sub(16, 16, 16, 16))),
        }
    });
    PropSet {
        boulder: props::boulder(),
        pebbles: props::pebbles(),
        driftwood: props::driftwood(),
        shell: props::shell(),
        bush: props::bush(),
        flowers: props::flowers(),
        vine: props::vine(true),
        vine_stake: props::vine(false),
        palm,
        stump: props::stump(),
        palm_tiles,
    }
}

fn load_sprites(vram: &mut Vram) -> SpriteIds {
    use sprite_art::*;
    let bank = &mut vram.sprites;
    let player = PlayerSprites::build(bank, &Outfit::CASTAWAY);
    let [c0, c1] = crab_frames();
    let [g0, g1] = gull_frames();
    let [r0, r1, r2] = wade_ripples();
    let stone_axe = item_art::icon_stone_axe();
    let bag = BagSprites {
        coconut: bank.add(&item_art::icon_coconut()),
        timber: bank.add(&item_art::icon_timber()),
        raw_water: bank.add(&item_art::icon_raw_water()),
        stone_axe: [bank.add(&stone_axe), bank.add(&stone_axe.rotated_cw())],
        branch: {
            let b = item_art::icon_branch();
            [bank.add(&b), bank.add(&b.rotated_cw())]
        },
        stone: bank.add(&item_art::icon_stone()),
        vine: bank.add(&item_art::icon_vine()),
        corner: bank.add(&item_art::cursor_corner()),
    };
    SpriteIds {
        bag,
        player,
        crab: [bank.add(&c0), bank.add(&c1)],
        gull: [bank.add(&g0), bank.add(&g1)],
        shadow_player: bank.add(&shadow(16, 8, 6.5, 2.6)),
        shadow_lying: bank.add(&shadow(32, 8, 14.0, 2.4)),
        shadow_crab: bank.add(&shadow(16, 8, 6.5, 2.2)),
        shadow_gull: bank.add(&shadow(16, 8, 5.0, 1.8)),
        ripples: [bank.add(&r0), bank.add(&r1), bank.add(&r2)],
        axe_side: bank.add(&stone_axe_side()),
        axe_up: bank.add(&axe_up(stone_axe_side())),
        axe_diag: bank.add(&axe_diag(stone_axe_side())),
        swoosh: swooshes().map(|a| bank.add(&a)),
        chip: chip_frames().map(|a| bank.add(&a)),
        leaf: leaf_frames().map(|a| bank.add(&a)),
        spark: spark_frames().map(|a| bank.add(&a)),
        coconut: coconut_frames().map(|a| bank.add(&a)),
        shadow_coconut: bank.add(&shadow(8, 8, 3.2, 1.4)),
        coconut_bitten: std::array::from_fn(|i| bank.add(&bitten_coconut(i + 1))),
        handful: handful_frames().map(|a| bank.add(&a)),
        timber: bank.add(&timber()),
        branch: bank.add(&branch()),
        stone: bank.add(&stone()),
        vine_coil: bank.add(&vine_coil()),
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
        bag_cell: hud_art::bag_cell().map(|a| bank.add(&a)),
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

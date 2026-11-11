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
    pub walk_poses: [[Pose; 4]; 3],
    pub chop: [[CharFrame; 3]; 3],
    pub chop_poses: [[Pose; 3]; 3],
    /// Spear thrust: drawn back, lunging.
    pub thrust: [[CharFrame; 2]; 3],
    pub thrust_poses: [[Pose; 2]; 3],
    /// Shaking a tree trunk: push, pull.
    pub shake: [[SpriteId; 2]; 3],
    /// Crouching to pick something up (or to scoop water).
    pub crouch: [SpriteId; 3],
    /// Drinking from cupped hands.
    pub drink: [SpriteId; 3],
    /// Waking up on the beach: lying (eyes closed, open; 32x24, head left),
    /// sitting up, kneeling (24x32, facing right).
    pub lying: [SpriteId; 2],
    /// Curled up asleep (32x24, head left).
    pub sleeping: SpriteId,
    pub sitting: SpriteId,
    pub kneeling: SpriteId,
    /// Climbing a ladder, seen from behind: left hand up, right hand up.
    pub climb: [SpriteId; 2],
}

pub const VIEWS: [View; 3] = [View::Front, View::Back, View::Side];

fn render_frame(pose: &Pose, outfit: &Outfit) -> [PixelArt; 4] {
    let r = character::render::render(pose, outfit);
    let wade = character::WADE_ROW;
    [r.body.clone(), r.body.keep_rows(0..wade), r.body.keep_rows(wade..r.body.h), r.hands]
}

impl PlayerSprites {
    fn poses() -> ([[Pose; 4]; 3], [[Pose; 3]; 3], [[Pose; 2]; 3]) {
        (
            VIEWS.map(|v| std::array::from_fn(|i| character::pose::walk(v, i))),
            VIEWS.map(|v| std::array::from_fn(|i| character::pose::chop(v, i))),
            VIEWS.map(|v| std::array::from_fn(|i| character::pose::thrust(v, i))),
        )
    }

    pub fn build(bank: &mut SpriteBank, outfit: &Outfit) -> Self {
        let (walk, chop, thrust) = Self::poses();
        let mut add = |pose: &Pose| {
            let [full, top, legs, hands] = render_frame(pose, outfit).map(|a| bank.add(&a));
            CharFrame { full, top, legs, hands }
        };
        let walk_frames = walk.map(|v| v.map(|p| add(&p)));
        let chop_frames = chop.map(|v| v.map(|p| add(&p)));
        let thrust_frames = thrust.map(|v| v.map(|p| add(&p)));
        let [lying, sitting, kneeling] = Self::wake_art(outfit);
        let lying = lying.map(|a| bank.add(&a));
        let sleeping = bank.add(&character::render_sleeping(outfit));
        Self {
            walk: walk_frames,
            walk_poses: walk,
            chop: chop_frames,
            chop_poses: chop,
            thrust: thrust_frames,
            thrust_poses: thrust,
            shake: Self::shake_art(outfit).map(|v| v.map(|a| bank.add(&a))),
            crouch: Self::crouch_art(outfit).map(|a| bank.add(&a)),
            drink: Self::drink_art(outfit).map(|a| bank.add(&a)),
            lying,
            sleeping,
            sitting: bank.add(&sitting[0]),
            kneeling: bank.add(&kneeling[0]),
            climb: std::array::from_fn(|i| bank.add(&character::render::render(&character::pose::climb(i), outfit).body)),
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
        let (walk, chop, thrust) = Self::poses();
        let pairs = walk
            .iter()
            .flatten()
            .zip(self.walk.iter().flatten())
            .chain(chop.iter().flatten().zip(self.chop.iter().flatten()))
            .chain(thrust.iter().flatten().zip(self.thrust.iter().flatten()));
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
        bank.replace(self.sleeping, &character::render_sleeping(outfit));
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
    pub fire_drill: SpriteId,
    /// Upright, turned.
    pub wood_spear: [SpriteId; 2],
    pub crab: SpriteId,
    pub grilled_crab: SpriteId,
    pub leaf: SpriteId,
    pub bark: SpriteId,
    pub lauan_timber: SpriteId,
    pub pine_timber: SpriteId,
    pub bark_container: SpriteId,
    pub boiled_water: SpriteId,
    /// Upright, turned.
    pub pack_frame: [SpriteId; 2],
    pub husk: SpriteId,
    pub rope: SpriteId,
    pub stone_knife: SpriteId,
    pub opened_coconut: SpriteId,
    /// Upright, turned.
    pub palm_frond: [SpriteId; 2],
    /// Flat, turned.
    pub thatch: [SpriteId; 2],
    pub shelter: SpriteId,
    /// Upright, turned.
    pub torch: [SpriteId; 2],
    pub sea_shell: SpriteId,
    pub shell_hook: SpriteId,
    /// Upright, turned.
    pub fishing_rod: [SpriteId; 2],
    /// Upright, turned.
    pub ladder: [SpriteId; 2],
    pub fish: SpriteId,
    pub grilled_fish: SpriteId,
    pub pine_cone: SpriteId,
    /// Stack-count digits 0-9 (effects palette).
    pub digits: [SpriteId; 10],
    /// "E" on the tool held in hand.
    pub equipped: SpriteId,
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
            ItemKind::OpenedCoconut => (self.opened_coconut, palettes::OBJ_CRAB),
            ItemKind::Timber => (self.timber, palettes::OBJ_CRAB),
            ItemKind::RawWater => (self.raw_water, palettes::OBJ_FX),
            ItemKind::StoneAxe => (self.stone_axe[turned as usize], palettes::OBJ_FX),
            ItemKind::FireDrill => (self.fire_drill, palettes::OBJ_FX),
            ItemKind::WoodSpear => (self.wood_spear[turned as usize], palettes::OBJ_FX),
            ItemKind::Crab => (self.crab, palettes::OBJ_CRAB),
            ItemKind::GrilledCrab => (self.grilled_crab, palettes::OBJ_CRAB),
            ItemKind::Leaf => (self.leaf, palettes::OBJ_FX),
            ItemKind::Bark => (self.bark, palettes::OBJ_CRAB),
            ItemKind::LauanTimber => (self.lauan_timber, palettes::OBJ_GULL),
            ItemKind::BarkContainer => (self.bark_container, palettes::OBJ_FX),
            ItemKind::BoiledWater => (self.boiled_water, palettes::OBJ_FX),
            ItemKind::PackFrame => (self.pack_frame[turned as usize], palettes::OBJ_FX),
            ItemKind::Branch => (self.branch[turned as usize], palettes::OBJ_CRAB),
            ItemKind::Stone => (self.stone, palettes::OBJ_CRAB),
            ItemKind::Vine => (self.vine, palettes::OBJ_FX),
            ItemKind::Husk => (self.husk, palettes::OBJ_CRAB),
            ItemKind::Rope => (self.rope, palettes::OBJ_CRAB),
            ItemKind::StoneKnife => (self.stone_knife, palettes::OBJ_FX),
            ItemKind::PalmFrond => (self.palm_frond[turned as usize], palettes::OBJ_FX),
            ItemKind::Thatch => (self.thatch[turned as usize], palettes::OBJ_FX),
            ItemKind::Shelter => (self.shelter, palettes::OBJ_FX),
            ItemKind::Torch => (self.torch[turned as usize], palettes::OBJ_CRAB),
            ItemKind::SeaShell => (self.sea_shell, palettes::OBJ_CRAB),
            ItemKind::ShellHook => (self.shell_hook, palettes::OBJ_CRAB),
            ItemKind::FishingRod => (self.fishing_rod[turned as usize], palettes::OBJ_FX),
            ItemKind::Fish => (self.fish, palettes::OBJ_FX),
            ItemKind::GrilledFish => (self.grilled_fish, palettes::OBJ_CRAB),
            ItemKind::Ladder => (self.ladder[turned as usize], palettes::OBJ_FX),
            ItemKind::PineCone => (self.pine_cone, palettes::OBJ_CRAB),
            ItemKind::PineTimber => (self.pine_timber, palettes::OBJ_CRAB),
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
    /// The wooden spear: lying point right, point up, point down.
    pub spear_side: SpriteId,
    /// A ladder lying on the ground.
    pub ladder_lying: SpriteId,
    pub spear_up: SpriteId,
    pub spear_down: SpriteId,
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
    pub fire_drill: SpriteId,
    pub crab_caught: SpriteId,
    pub crab_grilled: SpriteId,
    pub leaves: SpriteId,
    pub bark: SpriteId,
    pub lauan_timber: SpriteId,
    pub pine_timber: SpriteId,
    pub bark_pot: SpriteId,
    pub pack_frame: SpriteId,
    pub husk: SpriteId,
    pub rope: SpriteId,
    pub palm_frond: SpriteId,
    pub thatch: SpriteId,
    pub stone_knife: SpriteId,
    pub opened_coconut: SpriteId,
    pub shelter_roll: SpriteId,
    /// A thought bubble with one, two, three dots.
    pub thought: [SpriteId; 3],
    /// A speech bubble with a "!": a recipe came to mind.
    pub eureka: SpriteId,
    /// A lit torch, 16x32, three flame frames; and burning low.
    pub torch: [SpriteId; 3],
    pub torch_low: [SpriteId; 3],
    /// The fishing rod held out: level, tip up, tip down.
    pub rod_side: SpriteId,
    pub rod_up: SpriteId,
    pub rod_down: SpriteId,
    /// The float riding, bobbing, pulled under; a dot of line.
    pub float: [SpriteId; 3],
    pub line_dot: SpriteId,
    /// A drop of water from a splash.
    pub droplet: SpriteId,
    /// A fish's shape under the water (two tail frames), and lying caught
    /// or grilled on the ground.
    pub fish_shadow: [SpriteId; 2],
    pub fish: SpriteId,
    pub grilled_fish: SpriteId,
    pub sea_shell: SpriteId,
    pub shell_hook: SpriteId,
    pub pine_cone: SpriteId,
    /// The shelter pitched over a sleeper (who is hidden in it).
    pub lean_to: SpriteId,
    pub fire: FireSprites,
    /// A Z drifting up from a sleeper.
    pub snore: SpriteId,
    /// A puff of dust: whole, breaking up.
    pub dust: [SpriteId; 2],
}

/// A campfire: the stone ring (16x8) and flame frames (16x16), lively and
/// dying down.
#[derive(Clone, Debug)]
pub struct FireSprites {
    pub ring: SpriteId,
    pub flames: [SpriteId; 3],
    pub small: [SpriteId; 3],
}

#[derive(Clone, Debug)]
pub struct HudTiles {
    glyphs: Vec<(char, HudTileId)>,
    /// Letters of the title, at twice the size.
    title_glyphs: Vec<(char, [HudTileId; 4])>,
    pub heart: HudTileId,
    pub food: HudTileId,
    pub water: HudTileId,
    pub sun: HudTileId,
    pub moon: HudTileId,
    /// A bag cell's four 8x8 quarters: top-left, top-right, bottom-left,
    /// bottom-right.
    pub bag_cell: [HudTileId; 4],
    /// Gauge segments indexed by filled pixels (0..=8).
    pub gauge: [HudTileId; 9],
    pub gauge_cap_left: HudTileId,
    pub gauge_cap_right: HudTileId,
    /// Light mask, `LIGHT_MASK_TILES`² tiles in row-major order.
    pub light_mask: Vec<HudTileId>,
    /// Its tiles' pixels, and which are all dark (nothing to lay) and
    /// which all fully lit.
    pub light_mask_art: Vec<PixelArt>,
    pub light_mask_empty: Vec<bool>,
    pub light_mask_solid: Vec<bool>,
    /// Spare tiles rewritten each frame where lights overlap.
    pub light_scratch: Vec<HudTileId>,
}

impl HudTiles {
    pub const BLANK: HudTileId = 0;

    pub fn glyph(&self, c: char) -> HudTileId {
        self.glyphs.iter().find(|(g, _)| *g == c).map(|(_, t)| *t).unwrap_or(Self::BLANK)
    }

    /// A title letter's four tiles (top-left, top-right, bottom-left,
    /// bottom-right); blank for a space or a letter not in the title.
    pub fn title_glyph(&self, c: char) -> [HudTileId; 4] {
        self.title_glyphs.iter().find(|(g, _)| *g == c).map(|(_, t)| *t).unwrap_or([Self::BLANK; 4])
    }
}

/// Background props, composited onto ground textures by the tileset builder.
pub struct PropSet {
    pub boulder: PropTile,
    /// A driftwood log's left and right cell.
    pub driftwood: [PropTile; 2],
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
    /// A broadleaf tree's crown and upper trunk (BG2), 3x3 cells in
    /// row-major order.
    pub broadleaf_upper: [MetatileId; 9],
    /// Its crown's shadow and the trunk's foot (with its bark, and
    /// stripped) on the grass, and the stump left once felled (BG1).
    pub broadleaf_shade: [PropTile; 3],
    pub broadleaf_base: [[PropTile; 3]; 2],
    pub broadleaf_stump: PropTile,
    /// A pine's tiers and upper trunk (BG2), 3x3 cells in row-major order;
    /// their shadow and the trunk's foot on the grass (BG1).
    pub pine_upper: [MetatileId; 9],
    pub pine_shade: [PropTile; 3],
    pub pine_base: [PropTile; 3],
    pub pine_cone: PropTile,
    pub pine_stump: PropTile,
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
    let tree = props::broadleaf();
    let pine = props::pine();
    PropSet {
        broadleaf_upper: std::array::from_fn(|c| vram.metatiles.add_art(PAL_FOLIAGE, &tree.upper.sub(c % 3 * 16, c / 3 * 16, 16, 16))),
        broadleaf_shade: tree.shade,
        broadleaf_base: tree.base,
        broadleaf_stump: tree.stump,
        pine_upper: std::array::from_fn(|c| vram.metatiles.add_art(PAL_FOLIAGE, &pine.upper.sub(c % 3 * 16, c / 3 * 16, 16, 16))),
        pine_shade: pine.shade,
        pine_base: pine.base,
        pine_cone: props::pine_cone(),
        pine_stump: pine.stump,
        boulder: props::boulder(),
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
        fire_drill: bank.add(&item_art::fire_drill(16)),
        wood_spear: {
            let s = item_art::icon_wood_spear();
            [bank.add(&s), bank.add(&s.rotated_cw())]
        },
        crab: bank.add(&c0),
        grilled_crab: bank.add(&grilled(&c0)),
        leaf: bank.add(&item_art::icon_leaf()),
        bark: bank.add(&item_art::icon_bark()),
        lauan_timber: bank.add(&as_lauan(&item_art::icon_timber())),
        pine_timber: bank.add(&as_pine(&item_art::icon_timber())),
        bark_container: bank.add(&item_art::icon_bark_container()),
        boiled_water: bank.add(&item_art::icon_boiled_water()),
        pack_frame: {
            let f = item_art::icon_pack_frame();
            [bank.add(&f), bank.add(&f.rotated_cw())]
        },
        husk: bank.add(&item_art::icon_husk()),
        rope: bank.add(&item_art::icon_rope()),
        stone_knife: bank.add(&item_art::icon_stone_knife()),
        opened_coconut: bank.add(&item_art::icon_opened_coconut()),
        palm_frond: {
            let f = item_art::icon_palm_frond();
            [bank.add(&f), bank.add(&f.rotated_cw())]
        },
        thatch: {
            let t = item_art::icon_thatch();
            [bank.add(&t), bank.add(&t.rotated_cw())]
        },
        shelter: bank.add(&item_art::icon_shelter()),
        sea_shell: bank.add(&item_art::icon_sea_shell()),
        shell_hook: bank.add(&item_art::icon_shell_hook()),
        pine_cone: bank.add(&item_art::icon_pine_cone()),
        fishing_rod: {
            let r = item_art::icon_fishing_rod();
            [bank.add(&r), bank.add(&r.rotated_cw())]
        },
        ladder: {
            let l = item_art::icon_ladder();
            [bank.add(&l), bank.add(&l.rotated_cw())]
        },
        fish: bank.add(&item_art::icon_fish()),
        grilled_fish: bank.add(&item_art::icon_grilled_fish()),
        torch: {
            let t = &torch_frames(false)[0];
            [bank.add(t), bank.add(&t.rotated_cw())]
        },
        digits: item_art::count_digits().map(|a| bank.add(&a)),
        equipped: bank.add(&item_art::equipped_mark()),
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
        spear_side: bank.add(&spear_side()),
        ladder_lying: bank.add(&item_art::ladder_lying()),
        spear_up: bank.add(&spear_side().transposed().flipped_v()),
        spear_down: bank.add(&spear_side().transposed()),
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
        fire_drill: bank.add(&item_art::fire_drill(16)),
        crab_caught: bank.add(&crab_caught()),
        crab_grilled: bank.add(&grilled(&crab_caught())),
        leaves: bank.add(&leaves()),
        bark: bank.add(&bark()),
        lauan_timber: bank.add(&as_lauan(&timber())),
        pine_timber: bank.add(&as_pine(&timber())),
        bark_pot: bank.add(&bark_pot()),
        pack_frame: bank.add(&pack_frame()),
        husk: bank.add(&husk()),
        rope: bank.add(&rope_coil()),
        palm_frond: bank.add(&palm_frond()),
        thatch: bank.add(&thatch()),
        stone_knife: bank.add(&stone_knife()),
        opened_coconut: bank.add(&opened_coconut()),
        shelter_roll: bank.add(&shelter_roll()),
        lean_to: bank.add(&lean_to()),
        thought: thought_frames().map(|a| bank.add(&a)),
        eureka: bank.add(&eureka_bubble()),
        torch: torch_frames(false).map(|a| bank.add(&a)),
        torch_low: torch_frames(true).map(|a| bank.add(&a)),
        rod_side: bank.add(&rod_side()),
        rod_up: bank.add(&rod_side().transposed().flipped_v()),
        rod_down: bank.add(&rod_side().transposed()),
        float: float_frames().map(|a| bank.add(&a)),
        line_dot: bank.add(&line_dot()),
        droplet: bank.add(&droplet()),
        fish_shadow: fish_shadow_frames().map(|a| bank.add(&a)),
        fish: bank.add(&item_art::icon_fish()),
        grilled_fish: bank.add(&item_art::icon_grilled_fish()),
        sea_shell: bank.add(&item_art::icon_sea_shell()),
        shell_hook: bank.add(&item_art::icon_shell_hook()),
        pine_cone: bank.add(&pine_cone()),
        snore: bank.add(&snore()),
        dust: dust_frames().map(|a| bank.add(&a)),
        fire: FireSprites {
            ring: bank.add(&fire_ring()),
            flames: flames(12.0, 4.6).map(|a| bank.add(&a)),
            small: flames(6.0, 3.0).map(|a| bank.add(&a)),
        },
    }
}

/// The game's name, as the title screen shows it.
pub const TITLE: &str = "CASTAWAY ISLAND";

fn load_hud(vram: &mut Vram) -> HudTiles {
    let bank = &mut vram.hud_tiles;
    let blank = bank.add(&PixelArt::filled(8, 8, 0));
    debug_assert_eq!(blank, HudTiles::BLANK);
    let glyphs = hud_art::glyphs().into_iter().map(|(c, a)| (c, bank.add(&a))).collect();
    let mut title_glyphs: Vec<(char, [HudTileId; 4])> = Vec::new();
    for c in TITLE.chars().filter(|c| *c != ' ') {
        if title_glyphs.iter().any(|(g, _)| *g == c) {
            continue;
        }
        let art = hud_art::title_glyph(c).expect("title letter missing from the font");
        title_glyphs.push((c, [(0, 0), (8, 0), (0, 8), (8, 8)].map(|(x, y)| bank.add(&art.sub(x, y, 8, 8)))));
    }
    let mask = hud_art::light_mask();
    let gauge = std::array::from_fn(|i| bank.add(&hud_art::gauge_segment(i)));
    HudTiles {
        glyphs,
        title_glyphs,
        heart: bank.add(&hud_art::icon_heart()),
        food: bank.add(&hud_art::icon_food()),
        water: bank.add(&hud_art::icon_water()),
        sun: bank.add(&hud_art::icon_sun()),
        moon: bank.add(&hud_art::icon_moon()),
        bag_cell: hud_art::bag_cell().map(|a| bank.add(&a)),
        gauge,
        gauge_cap_left: bank.add(&hud_art::gauge_cap(false)),
        gauge_cap_right: bank.add(&hud_art::gauge_cap(true)),
        light_mask: {
            // Identical tiles (solid centre, empty corners) share one pattern.
            let mut seen: Vec<(Vec<u8>, HudTileId)> = Vec::new();
            mask.iter()
                .map(|t| match seen.iter().find(|(px, _)| *px == t.px) {
                    Some((_, id)) => *id,
                    None => {
                        let id = bank.add(t);
                        seen.push((t.px.clone(), id));
                        id
                    }
                })
                .collect()
        },
        light_mask_empty: mask.iter().map(|t| t.px.iter().all(|&v| v == 0)).collect(),
        light_mask_solid: mask.iter().map(|t| t.px.iter().all(|&v| v == 1)).collect(),
        light_scratch: (0..crate::render::light_canvas::SCRATCH_TILES).map(|_| bank.add(&PixelArt::filled(8, 8, 0))).collect(),
        light_mask_art: mask,
    }
}

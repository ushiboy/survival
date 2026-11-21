//! Dev tool: a sheet of the island's props and every item's art, for
//! checking and comparing graphics at a glance.
//!
//! Usage (from the repository): `pnpm art -- [options] [NAME...]`, or
//! `cargo run --profile tools --example art -- [options] [NAME...]` here.
//! Names keep only the cards whose name contains one of them (any case;
//! `pnpm art -- palm "pine cone"`). Options:
//!
//! - `--scale N`: pixels per art pixel (default 3).
//! - `--set-base`: keeps this run as the base later runs are compared
//!   with; `--clear-base` goes back to comparing with the run before.
//! - `--out DIR`: where to write (default `wasm/target/art`).
//!
//! Everything is drawn by the game's own tileset and PPU, in daylight:
//!
//! - Props stand on their ground (grass or sand) on a small map of their
//!   own, trees whole: the canopy on BG2 over the trunk's foot and the
//!   shade on BG1.
//! - Each item shows its bag icon on the bag's cells (upright, then turned
//!   when it turns), then lying on sand and on grass with its shadow.
//!
//! It writes `sheet.png`, and `sheet-diff.png` with the cards that changed
//! since the base (or the last run), before and after side by side.

// Shared with the map tool, which uses all of it.
#[allow(dead_code)]
#[path = "map_parts/image.rs"]
mod image;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use image::{Image, Rgb, text_width};
use survival_core::assets::Assets;
use survival_core::assets::palettes::{HUD_MENU, base_cgram};
use survival_core::assets::props::Ground;
use survival_core::common::math::Rect;
use survival_core::entity::item::Item;
use survival_core::items::ItemKind;
use survival_core::ppu::effects::{ColorMath, MathOp, MathSource, WindowRegion, layer};
use survival_core::ppu::{FrameState, Ppu, SCREEN_H, SCREEN_W};
use survival_core::render::camera::Camera;
use survival_core::render::sprites::DrawList;
use survival_core::render::tileset::{Tileset, VisualMap};
use survival_core::world::harvest::BARK_STRIPS;
use survival_core::world::island::SAND_BELOW;
use survival_core::world::terrain::{BambooPart, BroadleafPart, ClayPart, DriftwoodPart, PalmPart, PinePart, StumpPart};
use survival_core::world::{CHUNK_CELLS, Feature, TILE_SIZE, Terrain, WorldMap};

const BG: Rgb = [24, 24, 32];
const CARD_BG: Rgb = [40, 40, 52];
const LABEL: Rgb = [235, 235, 235];
const NOTE: Rgb = [150, 150, 165];
const CHANGED: Rgb = [255, 0, 255];
/// Font pixel size of labels.
const FONT: i32 = 2;
const GAP: i32 = 8;
const PAD: i32 = 4;
const TITLE_H: i32 = 14;

struct Options {
    out: PathBuf,
    scale: i32,
    filter: Vec<String>,
    set_base: bool,
    clear_base: bool,
}

fn options() -> Options {
    let mut o = Options { out: Path::new(env!("CARGO_MANIFEST_DIR")).join("target/art"), scale: 3, filter: Vec::new(), set_base: false, clear_base: false };
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--set-base" => o.set_base = true,
            "--clear-base" => o.clear_base = true,
            "--out" => o.out = PathBuf::from(args.next().expect("--out needs a directory")),
            "--scale" => o.scale = args.next().and_then(|v| v.parse::<i32>().ok()).expect("--scale needs a number").clamp(1, 8),
            "--" => {}
            other if other.starts_with("--") => panic!("unknown option {other} (see the top of wasm/examples/art.rs)"),
            name => o.filter.push(name.to_uppercase()),
        }
    }
    o
}

// --- Props ----------------------------------------------------------------

/// A prop on the sheet: what stands in its cells (from its top-left), and
/// any fruit or bark the tree carries.
struct Specimen {
    name: &'static str,
    ground: Ground,
    parts: Vec<((i32, i32), Feature)>,
    fruit: Option<u8>,
    bark: Option<u8>,
}

impl Specimen {
    fn one(name: &'static str, ground: Ground, f: Feature) -> Self {
        Self { name, ground, parts: vec![((0, 0), f)], fruit: None, bark: None }
    }

    fn size(&self) -> (i32, i32) {
        let w = self.parts.iter().map(|p| p.0.0).max().unwrap_or(0) + 1;
        let h = self.parts.iter().map(|p| p.0.1).max().unwrap_or(0) + 1;
        (w, h)
    }
}

fn tree(w: i32, h: i32, part: impl Fn(u8, u8) -> Feature) -> Vec<((i32, i32), Feature)> {
    (0..h).flat_map(|dy| (0..w).map(move |dx| (dx, dy))).map(|(dx, dy)| ((dx, dy), part(dx as u8, dy as u8))).collect()
}

/// Every prop the island's cells can hold.
fn specimens(a: &Assets) -> Vec<Specimen> {
    let p = &a.props;
    let palm = |flip| tree(PalmPart::W, PalmPart::H, move |dx, dy| Feature::Palm(PalmPart { dx, dy, flip }));
    let broadleaf = || tree(BroadleafPart::W, BroadleafPart::H, |dx, dy| Feature::Broadleaf(BroadleafPart { dx, dy, flip: false }));
    let pine = tree(PinePart::W, PinePart::H, |dx, dy| Feature::Pine(PinePart { dx, dy, flip: false }));
    let stump = StumpPart { flip: false };
    let log = |flip| tree(DriftwoodPart::W, 1, move |dx, _| Feature::Driftwood(DriftwoodPart { dx, flip }));
    let culm = |v| tree(1, BambooPart::H, move |_, dy| Feature::Bamboo(BambooPart { dy, v, flip: false }));
    let clay = |dug| tree(2, 2, move |dx, dy| Feature::ClayBed(ClayPart { part: dx + 2 * dy, dug }));
    let v = vec![
        Specimen { name: "PALM", ground: p.palm.lower[1].ground, parts: palm(false), fruit: Some(3), bark: None },
        Specimen { name: "PALM NO NUTS FLIPPED", ground: p.palm.lower[1].ground, parts: palm(true), fruit: Some(0), bark: None },
        Specimen::one("PALM STUMP", p.stump.ground, Feature::Stump(stump)),
        Specimen { name: "BROADLEAF", ground: p.broadleaf_base[0][1].ground, parts: broadleaf(), fruit: None, bark: Some(BARK_STRIPS) },
        Specimen { name: "BROADLEAF STRIPPED", ground: p.broadleaf_base[1][1].ground, parts: broadleaf(), fruit: None, bark: Some(0) },
        Specimen::one("BROADLEAF STUMP", p.broadleaf_stump.ground, Feature::BroadleafStump(stump)),
        Specimen { name: "PINE", ground: p.pine_base[1].ground, parts: pine, fruit: None, bark: None },
        Specimen::one("PINE STUMP", p.pine_stump.ground, Feature::PineStump(stump)),
        Specimen::one("PINE CONE", p.pine_cone.ground, Feature::PineCone),
        Specimen { name: "BAMBOO 1", ground: p.bamboo_base[0].ground, parts: culm(0), fruit: None, bark: None },
        Specimen { name: "BAMBOO 2", ground: p.bamboo_base[1].ground, parts: culm(1), fruit: None, bark: None },
        Specimen { name: "BAMBOO 3", ground: p.bamboo_base[2].ground, parts: culm(2), fruit: None, bark: None },
        Specimen::one("BAMBOO STUMP", p.bamboo_stump.ground, Feature::BambooStump(stump)),
        Specimen::one("BUSH", p.bush.ground, Feature::Bush),
        Specimen::one("VINE", p.vine.ground, Feature::Vine),
        Specimen::one("VINE STAKE", p.vine_stake.ground, Feature::VineStake),
        Specimen::one("BOULDER", p.boulder.ground, Feature::Rock),
        Specimen { name: "DRIFTWOOD", ground: p.driftwood[0].ground, parts: log(false), fruit: None, bark: None },
        Specimen { name: "DRIFTWOOD FLIPPED", ground: p.driftwood[0].ground, parts: log(true), fruit: None, bark: None },
        Specimen::one("SHELL", p.shell.ground, Feature::Shell),
        Specimen::one("FLOWERS", p.flowers.ground, Feature::Flower),
        Specimen { name: "CLAY BED", ground: Ground::Sand, parts: clay(false), fruit: None, bark: None },
        Specimen { name: "CLAY BED DUG", ground: Ground::Sand, parts: clay(true), fruit: None, bark: None },
        Specimen { name: "BARE EARTH", ground: Ground::Grass, parts: tree(2, 2, |dx, dy| Feature::BareEarth(dx + 2 * dy)), fruit: None, bark: None },
        Specimen::one("SITE BENCH", Ground::Grass, Feature::SiteBench),
    ];
    for s in &v {
        for (_, f) in &s.parts {
            assert!(on_sheet(*f), "{} uses a feature the sheet says it leaves out", s.name);
        }
    }
    v
}

/// Whether the sheet draws this kind of cell. Adding a feature to the game
/// stops this compiling until the sheet is told about it: add a specimen
/// above, or `false` with the reason.
fn on_sheet(f: Feature) -> bool {
    match f {
        Feature::Palm(_)
        | Feature::Stump(_)
        | Feature::Broadleaf(_)
        | Feature::BroadleafStump(_)
        | Feature::Rock
        | Feature::Bush
        | Feature::Driftwood(_)
        | Feature::Shell
        | Feature::Flower
        | Feature::Vine
        | Feature::VineStake
        | Feature::Pine(_)
        | Feature::PineStump(_)
        | Feature::PineCone
        | Feature::Bamboo(_)
        | Feature::BambooStump(_)
        | Feature::ClayBed(_)
        | Feature::BareEarth(_)
        | Feature::SiteBench => true,
        // Bare ground; the ladder, drawn with the cliff it stands against,
        // and the log bridge and water wheel, drawn with the river they're
        // in (see `pnpm map`, `pnpm map --bridge` and `pnpm map --wheel`);
        // and the signal fire, a sprite standing on bare ground (its
        // picture is the item's, on the items sheet); and the mill's ground
        // marked out and its foundation, drawn round the lathe (`pnpm map
        // --stakes`, `--foundation`, `--frame`, `--walls`, `--roof`).
        Feature::None
        | Feature::Ladder(_)
        | Feature::Bridge(_)
        | Feature::WaterWheel(_)
        | Feature::Lathe(_)
        | Feature::SignalFire(_)
        | Feature::Marking(_)
        | Feature::Foundation(_)
        | Feature::Framed(_)
        | Feature::MillWall
        | Feature::MillDoor => false,
        // The mill's room, kept off the island: seen in snapshots inside it
        // (`SNAP_INSIDE=1`).
        Feature::MillRoomWall => false,
    }
}

/// Each specimen (and an empty patch of sand and of grass for items) gets
/// a slot of this many cells, the prop at `SLOT_AT` in it. The slot's
/// outer ring may blend with its neighbours, so it isn't shown.
const SLOT: (i32, i32) = (7, 8);
const SLOT_AT: (i32, i32) = (2, 2);
const SLOTS_PER_ROW: i32 = 8;
/// Cells of plain sand round the slots, away from the sea at the edge.
const BORDER: i32 = 2;

/// The game's tileset and PPU, drawing a little map of its own.
struct Stage {
    assets: Assets,
    map: WorldMap,
    tileset: Tileset,
    visual: VisualMap,
    ppu: Ppu,
    frame: FrameState,
    list: DrawList,
}

fn slot_origin(i: usize) -> (i32, i32) {
    let (col, row) = (i as i32 % SLOTS_PER_ROW, i as i32 / SLOTS_PER_ROW);
    (BORDER + col * SLOT.0, BORDER + row * SLOT.1)
}

fn round_up(n: i32) -> i32 {
    (n + CHUNK_CELLS - 1) / CHUNK_CELLS * CHUNK_CELLS
}

/// The ground's terrain, and a height well inside its band (the tileset
/// draws shores and grass edges from the heights).
fn terrain_of(g: Ground) -> (Terrain, f32) {
    match g {
        Ground::Sand => (Terrain::Sand, SAND_BELOW / 2.0),
        Ground::Grass => (Terrain::Grass, SAND_BELOW + 5.0),
    }
}

impl Stage {
    /// Lays out `grounds.len()` slots, each on its ground.
    fn new(mut assets: Assets, grounds: &[Ground]) -> Self {
        let rows = (grounds.len() as i32 + SLOTS_PER_ROW - 1) / SLOTS_PER_ROW;
        let (w, h) = (round_up(2 * BORDER + SLOTS_PER_ROW * SLOT.0), round_up(2 * BORDER + rows * SLOT.1));
        let mut map = WorldMap::new(w, h, Terrain::Sand);
        let mut heights = vec![terrain_of(Ground::Sand).1; (w * h) as usize];
        for (i, &g) in grounds.iter().enumerate() {
            let (ox, oy) = slot_origin(i);
            let (t, e) = terrain_of(g);
            for y in oy..oy + SLOT.1 {
                for x in ox..ox + SLOT.0 {
                    map.set_terrain(x, y, t);
                    heights[(y * w + x) as usize] = e;
                }
            }
        }
        map.set_elevation(heights);
        let (tileset, visual) = Tileset::new(&map, &mut assets);
        Self { assets, map, tileset, visual, ppu: Ppu::new(), frame: FrameState::default(), list: DrawList::default() }
    }

    fn place(&mut self, i: usize, s: &Specimen) {
        let (ox, oy) = slot_origin(i);
        let (ax, ay) = (ox + SLOT_AT.0, oy + SLOT_AT.1);
        for &((dx, dy), f) in &s.parts {
            self.map.set_feature(ax + dx, ay + dy, f);
        }
        if let Some(n) = s.fruit {
            self.map.set_fruit((ax, ay), n);
        }
        if let Some(n) = s.bark {
            self.map.set_bark((ax, ay), n);
        }
    }

    /// The screen with its top-left at world px `at`: the ground layers in
    /// daylight, with `items` lying on them.
    fn world(&mut self, at: (i32, i32), items: &[Item]) -> Image {
        let (sw, sh) = (SCREEN_W as i32, SCREEN_H as i32);
        self.tileset.stream(&self.map, &mut self.assets, &mut self.visual, Rect::new(at.0, at.1, sw, sh));
        let camera = Camera { x: at.0, y: at.1, w: sw, h: sh, screen_top: 0 };
        let f = &mut self.frame;
        f.clock = 0;
        f.scroll_x = at.0;
        f.scroll_y = at.1;
        f.brightness = 15;
        f.cgram = base_cgram();
        f.lines.clear();
        // As the game draws daylight: shadows at half strength.
        f.math = ColorMath {
            layers: layer::OBJ,
            op: MathOp::Add,
            half: true,
            source: MathSource::SubScreen,
            region: WindowRegion::Everywhere,
            sub_layers: layer::BG1 | layer::BG2,
        };
        f.main_layers = layer::BG1 | layer::BG2 | layer::OBJ;
        f.bg3_priority = false;
        self.list.clear();
        for item in items {
            item.draw(&mut self.list, &camera, &self.assets.sprites, false);
        }
        f.oam.clear();
        self.list.flush(&mut f.oam);
        self.ppu.render(&self.assets.vram, &self.frame, &self.visual.ground, &self.visual.overhead);
        self.grab()
    }

    /// The bag screen's cells, `cells` of them, with `kind`'s icon in them.
    fn bag(&mut self, kind: ItemKind, turned: bool, cells: (i32, i32)) -> Image {
        // As the bag screen: BG3 cells behind the icon sprites.
        const COL: usize = 2;
        const ROW: usize = 2;
        let f = &mut self.frame;
        f.cgram = base_cgram();
        f.lines.clear();
        f.math = ColorMath::OFF;
        f.main_layers = layer::BG3 | layer::OBJ;
        f.bg3_priority = false;
        f.bg3.clear_rect(0, 0, 32, 28);
        for y in 0..cells.1 as usize {
            for x in 0..cells.0 as usize {
                for q in 0..4 {
                    f.bg3.put(COL + x * 2 + q % 2, ROW + y * 2 + q / 2, self.assets.hud.bag_cell[q], HUD_MENU);
                }
            }
        }
        self.list.clear();
        let (gfx, pal) = self.assets.sprites.bag.icon(kind, turned);
        self.list.sky(survival_core::ppu::oam::ObjEntry::new(COL as i32 * 8, ROW as i32 * 8, gfx, pal, 3));
        f.oam.clear();
        self.list.flush(&mut f.oam);
        self.ppu.render(&self.assets.vram, &self.frame, &self.visual.ground, &self.visual.overhead);
        crop(&self.grab(), COL as i32 * 8, ROW as i32 * 8, cells.0 * 16, cells.1 * 16)
    }

    fn grab(&self) -> Image {
        let (sw, sh) = (SCREEN_W as i32, SCREEN_H as i32);
        let rgba = self.ppu.rgba();
        let mut img = Image::new(sw, sh, [0; 3]);
        for y in 0..sh {
            for x in 0..sw {
                let i = ((y * sw + x) * 4) as usize;
                img.set(x, y, [rgba[i], rgba[i + 1], rgba[i + 2]]);
            }
        }
        img
    }
}

fn crop(img: &Image, x: i32, y: i32, w: i32, h: i32) -> Image {
    let mut out = Image::new(w, h, [0; 3]);
    for yy in 0..h {
        for xx in 0..w {
            if (0..img.w).contains(&(x + xx)) && (0..img.h).contains(&(y + yy)) {
                out.set(xx, yy, img.get(x + xx, y + yy));
            }
        }
    }
    out
}

fn scaled(img: &Image, s: i32) -> Image {
    let mut out = Image::new(img.w * s, img.h * s, [0; 3]);
    for y in 0..img.h {
        for x in 0..img.w {
            out.fill_rect(x * s, y * s, s, s, img.get(x, y));
        }
    }
    out
}

// --- The sheet ------------------------------------------------------------

/// One named picture on the sheet: its panels side by side (art px).
struct Card {
    name: String,
    panels: Vec<Image>,
}

impl Card {
    /// The panels, scaled, a gap apart.
    fn art(&self, s: i32) -> Image {
        let w = self.panels.iter().map(|p| p.w * s).sum::<i32>() + PAD * (self.panels.len() as i32 - 1).max(0);
        let h = self.panels.iter().map(|p| p.h * s).max().unwrap_or(0);
        let mut out = Image::new(w, h, CARD_BG);
        let mut x = 0;
        for p in &self.panels {
            out.paste(&scaled(p, s), x, 0);
            x += p.w * s + PAD;
        }
        out
    }

    /// Changes whenever any pixel of the art does.
    fn hash(&self) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        let mut eat = |v: u64| h = (h ^ v).wrapping_mul(0x100_0000_01b3);
        for p in &self.panels {
            eat(p.w as u64);
            eat(p.h as u64);
            for y in 0..p.h {
                for x in 0..p.w {
                    let [r, g, b] = p.get(x, y);
                    eat(((r as u64) << 16) | ((g as u64) << 8) | b as u64);
                }
            }
        }
        h
    }
}

/// A rectangle on the sheet (px): x, y, width, height.
type Area = (i32, i32, i32, i32);

/// Where a card landed on the sheet, with its hash.
struct Placed {
    name: String,
    hash: u64,
    rect: Area,
}

struct Section {
    title: &'static str,
    note: &'static str,
    cards: Vec<Card>,
}

/// The sections one under another, their cards in rows `width` px wide.
fn layout(sections: &[Section], s: i32, width: i32) -> (Image, Vec<Placed>) {
    struct Item<'a> {
        section: &'static str,
        card: &'a Card,
        art: Image,
        at: (i32, i32),
        size: (i32, i32),
    }
    let mut items = Vec::new();
    let mut heads = Vec::new();
    let mut y = GAP;
    for sec in sections {
        heads.push((sec.title, sec.note, y));
        y += TITLE_H + 12;
        let (mut x, mut row_h) = (GAP, 0);
        for card in &sec.cards {
            let art = card.art(s);
            let size = (art.w.max(text_width(FONT, &card.name)) + 2 * PAD, art.h + TITLE_H + 2 * PAD);
            if x > GAP && x + size.0 > width - GAP {
                (x, y) = (GAP, y + row_h + GAP);
                row_h = 0;
            }
            items.push(Item { section: sec.title, card, art, at: (x, y), size });
            x += size.0 + GAP;
            row_h = row_h.max(size.1);
        }
        y += row_h + GAP * 2;
    }
    // As wide as what's on it (up to `width`, or the widest card).
    let heads_w = sections.iter().map(|s| GAP * 2 + 12 + text_width(FONT + 1, s.title) + text_width(FONT, s.note));
    let w = items.iter().map(|i| i.at.0 + i.size.0 + GAP).chain(heads_w).max().unwrap_or(width);
    let mut sheet = Image::new(w, y, BG);
    for (title, note, hy) in heads {
        let tw = sheet.text(GAP, hy, FONT + 1, LABEL, title);
        sheet.text(GAP + tw + 12, hy + 4, FONT, NOTE, note);
    }
    let mut placed = Vec::new();
    for i in &items {
        sheet.fill_rect(i.at.0, i.at.1, i.size.0, i.size.1, CARD_BG);
        sheet.text(i.at.0 + PAD, i.at.1 + PAD, FONT, LABEL, &i.card.name);
        sheet.paste(&i.art, i.at.0 + PAD, i.at.1 + PAD + TITLE_H);
        placed.push(Placed { name: format!("{}: {}", i.section, i.card.name), hash: i.card.hash(), rect: (i.at.0, i.at.1, i.size.0, i.size.1) });
    }
    (sheet, placed)
}

fn manifest(placed: &[Placed]) -> String {
    placed.iter().map(|p| format!("{:016x} {} {} {} {} {}\n", p.hash, p.rect.0, p.rect.1, p.rect.2, p.rect.3, p.name)).collect()
}

fn read_manifest(text: &str) -> HashMap<String, (u64, Area)> {
    text.lines()
        .filter_map(|l| {
            let mut f = l.splitn(6, ' ');
            let hash = u64::from_str_radix(f.next()?, 16).ok()?;
            let mut n = || f.next()?.parse::<i32>().ok();
            let rect = (n()?, n()?, n()?, n()?);
            Some((f.next()?.to_string(), (hash, rect)))
        })
        .collect()
}

/// The changed cards, each as it was (left) and as it is now (right).
fn diff_sheet(old: &Image, new: &Image, changed: &[(&Placed, Area)]) -> Image {
    let rows: Vec<(Image, Image)> = changed.iter().map(|(p, r)| (crop(old, r.0, r.1, r.2, r.3), crop(new, p.rect.0, p.rect.1, p.rect.2, p.rect.3))).collect();
    let col_w = rows.iter().map(|(a, b)| a.w.max(b.w)).max().unwrap_or(0);
    let h: i32 = rows.iter().map(|(a, b)| a.h.max(b.h) + GAP).sum();
    let mut out = Image::new(GAP * 3 + col_w * 2, h + TITLE_H + GAP * 2, BG);
    out.text(GAP, GAP, FONT, LABEL, "BEFORE");
    out.text(GAP * 2 + col_w, GAP, FONT, LABEL, "AFTER");
    let mut y = GAP + TITLE_H;
    for (a, b) in &rows {
        out.paste(a, GAP, y);
        out.paste(b, GAP * 2 + col_w, y);
        out.frame(GAP * 2 + col_w - 2, y - 2, b.w + 4, b.h + 4, 2, CHANGED);
        y += a.h.max(b.h) + GAP;
    }
    out
}

fn main() {
    let o = options();
    std::fs::create_dir_all(&o.out).expect("make the output directory");
    let base = o.out.join("base");
    if o.clear_base {
        let _ = std::fs::remove_dir_all(&base);
    }
    let against = if base.is_dir() { base.clone() } else { o.out.clone() };
    let against_name = if base.is_dir() { "the base" } else { "the last run" };
    let wanted = |name: &str| o.filter.is_empty() || o.filter.iter().any(|f| name.contains(f.as_str()));

    let t = Instant::now();
    let assets = Assets::load();
    // Every specimen keeps its slot whatever is asked for: the ground's
    // texture and some props' mirroring follow the cell, so a card looks
    // the same in every run.
    let specimens = specimens(&assets);
    let mut grounds: Vec<Ground> = specimens.iter().map(|s| s.ground).collect();
    let (sand_slot, grass_slot) = (grounds.len(), grounds.len() + 1);
    grounds.extend([Ground::Sand, Ground::Grass]);
    let mut stage = Stage::new(assets, &grounds);
    for (i, s) in specimens.iter().enumerate() {
        stage.place(i, s);
    }

    let mut props = Vec::new();
    for (i, s) in specimens.iter().enumerate().filter(|(_, s)| wanted(s.name)) {
        let (ox, oy) = slot_origin(i);
        let (w, h) = s.size();
        // The prop with a cell of its ground all round.
        let (x0, y0) = ((ox + SLOT_AT.0 - 1) * TILE_SIZE, (oy + SLOT_AT.1 - 1) * TILE_SIZE);
        let screen = stage.world((x0, y0), &[]);
        props.push(Card { name: s.name.to_string(), panels: vec![crop(&screen, 0, 0, (w + 2) * TILE_SIZE, (h + 2) * TILE_SIZE)] });
    }

    let mut items = Vec::new();
    for kind in ItemKind::ALL.into_iter().filter(|k| wanted(k.name())) {
        let (w, h) = kind.size();
        let mut panels = vec![stage.bag(kind, false, (w, h))];
        if stage.assets.sprites.bag.icon(kind, true).0 != stage.assets.sprites.bag.icon(kind, false).0 {
            panels.push(stage.bag(kind, true, (h, w)));
        }
        for slot in [sand_slot, grass_slot] {
            let (ox, oy) = slot_origin(slot);
            let at = ((ox + 1) * TILE_SIZE, (oy + 1) * TILE_SIZE);
            let spot = (at.0 + 24, at.1 + 16);
            let screen = stage.world(at, &[Item::lying(kind, (spot.0 as f32, spot.1 as f32))]);
            panels.push(crop(&screen, 0, 0, 48, 24));
        }
        items.push(Card { name: kind.name().to_string(), panels });
    }

    let sections = [
        Section { title: "PROPS", note: "ON THEIR GROUND, WITH A CELL ROUND THEM", cards: props },
        Section { title: "ITEMS", note: "BAG ICON (AND TURNED) / LYING ON SAND / ON GRASS", cards: items },
    ];
    let count: usize = sections.iter().map(|s| s.cards.len()).sum();
    if count == 0 {
        println!("nothing matches {:?}", o.filter);
        return;
    }
    let (sheet, placed) = layout(&sections, o.scale, 400 * o.scale);

    // What changed since the base (or the last run): cards drawn at the
    // same scale whose pixels differ.
    let meta = format!("scale {}\n", o.scale);
    let old_meta = std::fs::read_to_string(against.join("sheet.txt")).ok();
    let old = (old_meta.as_deref() == Some(meta.as_str()))
        .then(|| Some((Image::load_png(&against.join("sheet.png"))?, read_manifest(&std::fs::read_to_string(against.join("sheet.cards")).ok()?))))
        .flatten();
    sheet.save_png(&o.out.join("sheet.png"));
    std::fs::write(o.out.join("sheet.cards"), manifest(&placed)).expect("write sheet.cards");
    std::fs::write(o.out.join("sheet.txt"), &meta).expect("write sheet.txt");
    let diff = o.out.join("sheet-diff.png");
    let _ = std::fs::remove_file(&diff);
    match old {
        Some((old_sheet, old_cards)) => {
            let changed: Vec<_> = placed.iter().filter_map(|p| old_cards.get(&p.name).filter(|(h, _)| *h != p.hash).map(|&(_, r)| (p, r))).collect();
            let new: Vec<_> = placed.iter().filter(|p| !old_cards.contains_key(&p.name)).map(|p| p.name.as_str()).collect();
            if changed.is_empty() {
                println!("no card changed since {against_name}");
            } else {
                let names: Vec<_> = changed.iter().map(|(p, _)| p.name.as_str()).collect();
                println!("changed since {against_name}: {} (see sheet-diff.png)", names.join(", "));
                diff_sheet(&old_sheet, &sheet, &changed).save_png(&diff);
            }
            match new.len() {
                0 => {}
                1..=8 => println!("not in {against_name}: {}", new.join(", ")),
                n => println!("{n} cards not in {against_name}"),
            }
        }
        None if old_meta.is_some() => println!("not compared ({against_name} was drawn at another scale)"),
        None => {}
    }
    println!("wrote sheet.png: {count} cards, {}x{} px ({} ms)", sheet.w, sheet.h, t.elapsed().as_millis());

    if o.set_base {
        std::fs::create_dir_all(&base).expect("make base/");
        for f in ["sheet.png", "sheet.cards", "sheet.txt"] {
            let _ = std::fs::copy(o.out.join(f), base.join(f));
        }
        println!("kept this run as the base");
    }
    println!("in {}", o.out.display());
}

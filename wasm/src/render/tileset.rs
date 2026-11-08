//! Builds the BG1 (ground) and BG2 (overhead) tile maps from world data.
//!
//! Ground textures are 64x64 seamless canvases sliced per cell. Where a
//! terrain boundary passes through a cell, the tile is painted per pixel from
//! the smooth elevation field, so coastlines curve freely instead of
//! following the 16px grid. Every tile is composed once at load time; the
//! runtime cost is a table lookup.

use std::collections::HashMap;

use crate::assets::palettes::{PAL_DEEP, PAL_GRASS, PAL_SAND, PAL_SHORE, deep, grass, shore};
use crate::common::rng::hash_unit;
use crate::world::river::River;
use crate::assets::paint::{bayer, noise_px};
use crate::assets::props::{Ground, PropTile, composite};
use crate::assets::terrain::{TerrainTextures, sand_to, sand_to_shore, slice};
use crate::assets::{Assets, PALM_SWAY_MAX, PropSet, SURF_FRAMES, SURF_PERIOD};
use crate::common::math::Rect;
use crate::ppu::banks::{MetatileBank, MetatileId};
use crate::ppu::pattern::Tile16;
use crate::world::chunk::chunks_touching;
use crate::ppu::pattern::PixelArt;
use crate::ppu::{BgCell, BgLayer};
use crate::world::island::{DEEP_BELOW, SAND_BELOW};
use crate::world::noise::value_noise;
use crate::world::harvest::BARK_STRIPS;
use crate::world::terrain::{BroadleafPart, PalmPart};
use crate::world::{CHUNK_CELLS, CHUNK_PX, ChunkId, Feature, TILE_SIZE, Terrain, WorldMap};

/// A layer's tile map.
pub struct TileGrid {
    w: i32,
    h: i32,
    cells: Vec<BgCell>,
    border: BgCell,
}

impl TileGrid {
    fn new(w: i32, h: i32, border: BgCell) -> Self {
        Self { w, h, cells: vec![BgCell::EMPTY; (w * h) as usize], border }
    }

    fn set(&mut self, x: i32, y: i32, c: BgCell) {
        self.cells[(y * self.w + x) as usize] = c;
    }

    fn get(&self, x: i32, y: i32) -> BgCell {
        self.cells[(y * self.w + x) as usize]
    }
}

impl BgLayer for TileGrid {
    #[inline]
    fn cell(&self, mx: i32, my: i32) -> BgCell {
        if mx < 0 || my < 0 || mx >= self.w || my >= self.h {
            self.border
        } else {
            self.cells[(my * self.w + mx) as usize]
        }
    }
}

pub struct VisualMap {
    pub ground: TileGrid,
    pub overhead: TileGrid,
}

// ---------------------------------------------------------------- elevation field

/// Approximate pixels per elevation unit near the contours.
const PX_PER_UNIT: f32 = 14.0;

/// Elevation for every pixel of one cell, with a little wobble so contours
/// look hand-drawn rather than mathematically smooth.
struct CellField {
    h: [f32; 256],
    min: f32,
    max: f32,
}

impl CellField {
    fn sample(map: &WorldMap, cx: i32, cy: i32) -> Self {
        let mut h = [0.0; 256];
        for py in 0..16 {
            for px in 0..16 {
                let (wx, wy) = ((cx * TILE_SIZE + px) as f32 + 0.5, (cy * TILE_SIZE + py) as f32 + 0.5);
                h[(py * 16 + px) as usize] = map.elevation_at_px(wx, wy) + value_noise(wx / 5.0, wy / 5.0, 77) * 0.14;
            }
        }
        let min = h.iter().copied().fold(f32::MAX, f32::min);
        let max = h.iter().copied().fold(f32::MIN, f32::max);
        Self { h, min, max }
    }

    #[inline]
    fn at(&self, x: usize, y: usize) -> f32 {
        self.h[y * 16 + x]
    }

    fn crosses(&self, level: f32, below: f32, above: f32) -> bool {
        self.min < level + above && self.max > level - below
    }
}

// ---------------------------------------------------------------- river field

/// Where every pixel of one cell lies relative to the river: how far inside
/// the water (px, negative on land) and its flow coordinates (px).
struct RiverField {
    depth: [f32; 256],
    along: [f32; 256],
    across: [f32; 256],
    max: f32,
}

impl RiverField {
    /// The cell's river field, if the water or its bank reaches into it.
    fn sample(map: &WorldMap, cx: i32, cy: i32) -> Option<Self> {
        let river = map.river.as_ref().filter(|_| map.is_riverside(cx, cy))?;
        let ts = TILE_SIZE as f32;
        let mut r = Self { depth: [0.0; 256], along: [0.0; 256], across: [0.0; 256], max: f32::MIN };
        for i in 0..256 {
            let (wx, wy) = ((cx * TILE_SIZE) as f32 + (i % 16) as f32 + 0.5, (cy * TILE_SIZE) as f32 + (i / 16) as f32 + 0.5);
            let p = river.at_px(wx, wy);
            // A slight wobble so the banks don't look ruled.
            let wobble = value_noise(wx / 6.0, wy / 6.0, 91) * 1.4;
            r.depth[i] = (River::half_width(p.t) - p.dist) * ts + wobble;
            r.along[i] = p.along * ts;
            r.across[i] = p.across * ts;
            r.max = r.max.max(r.depth[i]);
        }
        // Cells the trampled landing reaches are drawn as bank too.
        let near_spot = map.water_spot.is_some_and(|(sx, sy)| {
            let (dx, dy) = ((cx * TILE_SIZE + 8 - sx) as f32, (cy * TILE_SIZE + 8 - sy) as f32);
            dx.abs() < LANDING_RADIUS + 8.0 && dy.abs() < LANDING_RADIUS + 8.0
        });
        (r.max > BANK_LIP || near_spot).then_some(r)
    }
}

/// Depth (below the waterline) down to which the river churns the surf
/// where it meets the sea; cells all deeper are plain sea.
const RIVER_CHURN_BELOW: f32 = -1.4;

/// Bank drawn this far (px) from the water's edge.
const BANK_LIP: f32 = -4.5;
/// Radius (px) of the trampled patch at the water-drawing spot.
const LANDING_RADIUS: f32 = 11.0;

fn compose(f: impl Fn(usize, usize) -> u8) -> PixelArt {
    let mut out = PixelArt::filled(16, 16, 0);
    for y in 0..16 {
        for x in 0..16 {
            out.set(x, y, f(x, y));
        }
    }
    out
}

/// Steps a water colour lighter (+) or darker (-) along the shore ramp.
fn shift_water(v: u8, steps: i32) -> u8 {
    match shore::WATER.iter().position(|&c| c == v) {
        Some(i) => shore::WATER[(i as i32 + steps).clamp(0, 3) as usize],
        None => v,
    }
}

/// Water colour for a given depth in the shallows palette (the deep palette
/// carries the two darkest shades as `SHELF` so the two meet seamlessly).
fn shallow_depth(v: u8, h: f32, x: i32, y: i32) -> u8 {
    let b = bayer(x, y);
    if h < DEEP_BELOW + 0.4 {
        shore::WATER[0]
    } else if h < DEEP_BELOW + 1.0 {
        if b < 0.5 { shore::WATER[0] } else { shore::WATER[1] }
    } else if h < DEEP_BELOW + 1.7 {
        shift_water(v, -1)
    } else {
        v
    }
}

// ---------------------------------------------------------------- builder

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Key {
    Interior(Terrain, i32, i32),
    Prop { kind: u8, flip: bool, vx: i32, vy: i32 },
}

struct Builder<'a> {
    tex: &'a TerrainTextures,
    bank: &'a mut MetatileBank,
    cache: &'a mut HashMap<Key, MetatileId>,
    /// Blocks made by this builder, with their cache key if shared.
    made: Vec<(MetatileId, Option<Key>)>,
}

impl Builder<'_> {
    fn get(&mut self, key: Key, make: impl FnOnce(&TerrainTextures, &mut MetatileBank) -> MetatileId) -> MetatileId {
        if let Some(&id) = self.cache.get(&key) {
            return id;
        }
        let id = make(self.tex, self.bank);
        self.cache.insert(key, id);
        self.made.push((id, Some(key)));
        id
    }

    /// A block made for one cell only (not shared).
    fn unique(&mut self, palette: u8, frames: Vec<Tile16>, period: u32) -> MetatileId {
        let id = self.bank.add(palette, frames, period);
        self.made.push((id, None));
        id
    }

    fn unique_art(&mut self, palette: u8, art: &PixelArt) -> MetatileId {
        self.unique(palette, vec![art.to_tile16()], 1)
    }

    fn interior(&mut self, t: Terrain, x: i32, y: i32) -> MetatileId {
        let (vx, vy) = (x.rem_euclid(4), y.rem_euclid(4));
        self.get(Key::Interior(t, vx, vy), |tex, bank| match t {
            Terrain::DeepWater => bank.add_art(PAL_DEEP, &slice(&tex.deep, vx, vy)),
            Terrain::ShallowWater => bank.add_art(PAL_SHORE, &slice(&tex.shallow, vx, vy)),
            Terrain::Sand => bank.add_art(PAL_SAND, &slice(&tex.sand, vx, vy)),
            Terrain::Grass => bank.add_art(PAL_GRASS, &slice(&tex.grass, vx, vy)),
            // Transparent: the river is drawn on BG2.
            Terrain::River => bank.add_art(PAL_DEEP, &PixelArt::filled(16, 16, 0)),
        })
    }

    /// Beach meeting the sea: dry sand, wet sand, lapping foam, then water
    /// darkening toward the reef. Animated by swapping frames (DMA).
    fn coast(&mut self, f: &CellField, x: i32, y: i32, river: Option<&RiverField>) -> MetatileId {
        let water = slice(&self.tex.shallow, x, y);
        let sand = slice(&self.tex.sand, x, y).map(sand_to_shore);
        let frames = (0..SURF_FRAMES)
            .map(|fr| {
                let phase = fr as f32 / SURF_FRAMES as f32 * std::f32::consts::TAU;
                // Where the surf line is this frame, in px from the waterline.
                let front = -1.5 + 4.0 * (0.5 - 0.5 * phase.cos());
                compose(|px, py| {
                    let h = f.at(px, py);
                    let d = -h * PX_PER_UNIT;
                    let (ix, iy) = (px as i32, py as i32);
                    // Where the river runs out over the beach, BG2 shows its
                    // water through this layer.
                    let in_river = river.map_or(f32::MIN, |r| r.depth[py * 16 + px]);
                    if in_river > 0.0 && h > -0.6 {
                        0
                    } else if in_river > 0.0 && h > RIVER_CHURN_BELOW {
                        // Churn where the current meets the sea.
                        if bayer(ix, iy) < 0.5 { shore::FOAM } else { shore::FOAM_SHADE }
                    } else if in_river > -2.0 && h > 0.0 {
                        shore::WET_DARK
                    } else if d < -1.5 {
                        sand.get(px, py)
                    } else if d < front {
                        if d < -0.5 && bayer(ix, iy) < 0.5 { shore::WET_DARK } else { shore::WET }
                    } else if d < front + 1.3 {
                        shore::FOAM
                    } else if d < front + 2.3 {
                        if bayer(ix, iy) < 0.5 { shore::FOAM_SHADE } else { shore::WATER[3] }
                    } else if d < front + 5.0 {
                        shallow_depth(shift_water(water.get(px, py), 1), h, ix, iy)
                    } else {
                        shallow_depth(water.get(px, py), h, ix, iy)
                    }
                })
                .to_tile16()
            })
            .collect();
        self.unique(PAL_SHORE, frames, SURF_PERIOD)
    }

    /// Open shallows deepening toward the reef edge.
    fn shallows(&mut self, f: &CellField, x: i32, y: i32) -> MetatileId {
        let water = slice(&self.tex.shallow, x, y);
        let art = compose(|px, py| shallow_depth(water.get(px, py), f.at(px, py), px as i32, py as i32));
        self.unique_art(PAL_SHORE, &art)
    }

    /// Deep water rising toward the shallows.
    fn reef(&mut self, f: &CellField, x: i32, y: i32) -> MetatileId {
        let base = slice(&self.tex.deep, x, y);
        let art = compose(|px, py| {
            let h = f.at(px, py);
            let b = bayer(px as i32, py as i32);
            if h > DEEP_BELOW + 0.4 {
                deep::SHELF[1]
            } else if h > DEEP_BELOW - 0.4 {
                deep::SHELF[0]
            } else if h > DEEP_BELOW - 1.0 && b < 0.5 {
                deep::SHELF[0]
            } else {
                base.get(px, py)
            }
        });
        self.unique_art(PAL_DEEP, &art)
    }

    /// Grass giving way to sand with a ragged fringe.
    fn grass_edge(&mut self, f: &CellField, x: i32, y: i32) -> MetatileId {
        let (g, s) = self.grass_and_sand(x, y);
        let art = compose(|px, py| grass_or_sand(f, &g, &s, x, y, px, py));
        self.unique_art(PAL_GRASS, &art)
    }

    /// Grass and sand texture slices for a cell, both in the grass palette.
    fn grass_and_sand(&self, x: i32, y: i32) -> (PixelArt, PixelArt) {
        let g = slice(&self.tex.grass, x, y);
        let s = slice(&self.tex.sand, x, y).map(|v| sand_to(v, grass::SAND[0], grass::SAND[1], grass::SAND[2]));
        (g, s)
    }

    /// Land along the river (BG1). The water itself is left transparent so
    /// BG2's river shows through; the grass (or beach) runs right to it and
    /// drops to an earthen lip at the waterline. Around the water-drawing
    /// spot the bank is trampled bare, with flat stones at the edge.
    fn river_bank(&mut self, map: &WorldMap, f: &CellField, r: &RiverField, x: i32, y: i32) -> MetatileId {
        if f.max < RIVER_CHURN_BELOW {
            // Off the mouth, out at sea: the water as anywhere else (the
            // river's own water is on BG2).
            return self.terrain(map, x, y);
        }
        if f.min < 0.6 {
            // Out on the beach the river runs into the surf.
            return self.coast(f, x, y, Some(r));
        }
        let (g, s) = self.grass_and_sand(x, y);
        let ramp = grass::RAMP;
        let spot = map.water_spot.map(|(sx, sy)| (sx as f32, sy as f32));
        let art = compose(|px, py| {
            let i = py * 16 + px;
            let d = r.depth[i];
            if d > 0.0 {
                return 0;
            }
            let (wx, wy) = (x * 16 + px as i32, y * 16 + py as i32);
            let grassy = (f.at(px, py) - SAND_BELOW) * PX_PER_UNIT + noise_px(wx, wy, 0x6EED) * 1.6 >= 0.0;
            let trodden = spot.map_or(f32::MAX, |(sx, sy)| ((wx as f32 - sx).powi(2) + (wy as f32 - sy).powi(2)).sqrt());
            let b = bayer(wx, wy);
            let edge = LANDING_RADIUS + noise_px(wx / 2, wy / 2, 0x7A0D) * 4.0;
            if trodden < edge {
                if d > -7.0 && trodden < LANDING_RADIUS - 1.0 {
                    // Flat stones set along the water's edge to kneel on.
                    let (gx, gy) = (wx.div_euclid(6), wy.div_euclid(6));
                    let (cxs, cys) = (gx as f32 * 6.0 + 3.0 + hash_unit(gx, gy, 0x5701) - 0.5, gy as f32 * 6.0 + 3.0 + hash_unit(gx, gy, 0x5702) - 0.5);
                    let sd = ((wx as f32 + 0.5 - cxs).powi(2) + (wy as f32 + 0.5 - cys).powi(2)).sqrt();
                    if sd < 2.9 && hash_unit(gx, gy, 0x5703) < 0.8 {
                        // Lit from the top-left, dark rim.
                        let lit = (wx as f32 + 0.5 - cxs) + (wy as f32 + 0.5 - cys) < -0.5;
                        return if sd > 2.1 { grass::SAND[0] } else if lit { grass::WHITE } else { grass::SAND[2] };
                    }
                }
                // Bare, trampled earth, fraying into the grass.
                return if trodden > edge - 2.0 && b < 0.5 {
                    ramp[1]
                } else if b < 0.35 {
                    grass::SOIL
                } else if b < 0.85 {
                    grass::SAND[0]
                } else {
                    grass::SAND[1]
                };
            }
            if d > -1.5 {
                if grassy { grass::SOIL } else { grass::SAND[0] }
            } else if grassy && d > -3.0 {
                // The turf overhangs the cut bank in a dark edge.
                if b < 0.5 { ramp[0] } else { ramp[1] }
            } else if grassy && d > -BANK_LIP.abs() {
                if b < 0.5 { ramp[1] } else { grass_or_sand(f, &g, &s, x, y, px, py) }
            } else {
                grass_or_sand(f, &g, &s, x, y, px, py)
            }
        });
        self.unique_art(PAL_GRASS, &art)
    }

    /// River water (BG2, under BG1's transparent water pixels): dark under
    /// the banks, deeper in the middle, with streaks of current whose
    /// colours cycle so they race downstream.
    fn river_water(&mut self, r: &RiverField, x: i32, y: i32) -> MetatileId {
        let art = compose(|px, py| {
            let i = py * 16 + px;
            let d = r.depth[i];
            let (wx, wy) = (x * 16 + px as i32, y * 16 + py as i32);
            let b = bayer(wx, wy);
            if d <= 0.0 {
                return 0;
            }
            if d < 1.5 || (d < 3.0 && b < 0.5) {
                return deep::RIVER_SHADE;
            }
            // Lanes of current 2px wide, broken into dashes along the flow.
            let lane = (r.across[i] / 2.0).floor() as i32;
            let along = r.along[i] + hash_unit(lane, 0, 0xF10E) * 40.0;
            let dash = (along / 16.0).floor() as i32;
            let on = d > 3.5 && hash_unit(lane, dash, 0xF10F) < 0.22 && along.rem_euclid(16.0) < 8.0;
            if on {
                // Colour index steps with distance downstream; the palette
                // rotation then moves each colour one step down per tick.
                return deep::RIVER_FLOW[(r.along[i] / 2.0).floor().rem_euclid(3.0) as usize];
            }
            if d > 12.0 && b < 0.3 { deep::RIVER_SHADE } else { deep::RIVER_BODY }
        });
        self.unique_art(PAL_DEEP, &art)
    }

    fn prop(&mut self, kind: u8, tile: &PropTile, flip: bool, x: i32, y: i32) -> MetatileId {
        let (vx, vy) = (x.rem_euclid(4), y.rem_euclid(4));
        self.get(Key::Prop { kind, flip, vx, vy }, |tex, bank| {
            let ground = match tile.ground {
                Ground::Sand => slice(&tex.sand, vx, vy),
                Ground::Grass => slice(&tex.grass, vx, vy),
            };
            let t = if flip { PropTile { overlay: tile.overlay.flipped_h(), ..tile.clone() } } else { tile.clone() };
            bank.add_art(t.palette, &composite(&t, &ground))
        })
    }

    /// Ground tile for a cell without props.
    fn terrain(&mut self, map: &WorldMap, x: i32, y: i32) -> MetatileId {
        let t = map.terrain(x, y);
        let f = CellField::sample(map, x, y);
        match t {
            Terrain::Sand | Terrain::ShallowWater if f.crosses(0.0, 0.75, 0.45) => self.coast(&f, x, y, None),
            // Shallows dipping past the deep sea's edge are drawn with the
            // deep palette's shelf (the shallows' palette has no deep
            // colours), so the edge between them follows the depth instead
            // of the cell.
            Terrain::ShallowWater if f.min < DEEP_BELOW - 0.4 && f.max < DEEP_BELOW + 1.7 => self.reef(&f, x, y),
            Terrain::ShallowWater if f.min < DEEP_BELOW + 1.7 => self.shallows(&f, x, y),
            Terrain::DeepWater if f.max > DEEP_BELOW - 1.0 => self.reef(&f, x, y),
            Terrain::Sand | Terrain::Grass if f.crosses(SAND_BELOW, 0.35, 0.3) => self.grass_edge(&f, x, y),
            _ => self.interior(t, x, y),
        }
    }
}

/// One pixel of land where grass gives way to sand (grass palette).
fn grass_or_sand(f: &CellField, g: &PixelArt, s: &PixelArt, x: i32, y: i32, px: usize, py: usize) -> u8 {
    let r = grass::RAMP;
    let (wx, wy) = (x * 16 + px as i32, y * 16 + py as i32);
    let into = (f.at(px, py) - SAND_BELOW) * PX_PER_UNIT + noise_px(wx, wy, 0x6EED) * 1.6;
    if into < 0.0 {
        // Stray blades poking out over the sand.
        if into > -1.2 && noise_px(wx, wy, 0xB1AD) < 0.3 { r[4] } else { s.get(px, py) }
    } else if into < 1.3 {
        r[1]
    } else if into < 2.4 {
        if bayer(wx, wy) < 0.5 { r[2] } else { g.get(px, py) }
    } else {
        g.get(px, py)
    }
}

/// How far around the screen (px) ground must be ready before it is drawn,
/// how far ahead of the screen chunks are built a little each frame, and
/// past what distance built chunks are put away (their blocks freed).
const NEEDED_MARGIN: i32 = 16;
const PREFETCH_MARGIN: i32 = CHUNK_PX / 2;
const KEEP_MARGIN: i32 = CHUNK_PX;
/// Rows of a chunk built per frame while building ahead.
const PREFETCH_ROWS: i32 = 4;

/// How far a chunk's tiles have been built.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Built {
    No,
    /// Rows above this one (chunk-relative) are built.
    Rows(i32),
    Yes,
}

/// Builds the ground for the chunks near the screen and puts away the rest,
/// the way a game streams map tiles and CHR into VRAM as it scrolls. Also
/// re-derives cells when the world changes (e.g. a tree is felled).
///
/// Blocks it makes are counted by the cells using them: shared ones (a
/// texture's slice) are kept in a cache while any cell uses them, and a
/// block no cell uses any longer is freed from VRAM.
pub struct Tileset {
    cache: HashMap<Key, MetatileId>,
    /// Blocks made here (ids from `first_made` on): cells using each, and
    /// its cache key if it is shared.
    uses: HashMap<MetatileId, (u32, Option<Key>)>,
    /// Ids below this are the fixed blocks loaded with the assets.
    first_made: usize,
    chunks: (i32, i32),
    built: Vec<Built>,
}

impl Tileset {
    /// Sets up the (still empty) visual map for `map`; ground is built as
    /// it comes near the screen (`stream`).
    pub fn new(map: &WorldMap, assets: &mut Assets) -> (Self, VisualMap) {
        let chunks = map.chunk_count();
        let mut tileset = Self {
            cache: HashMap::new(),
            uses: HashMap::new(),
            first_made: assets.vram.metatiles.slots(),
            chunks,
            built: vec![Built::No; (chunks.0 * chunks.1) as usize],
        };
        let mut b = Builder { tex: &assets.textures, bank: &mut assets.vram.metatiles, cache: &mut tileset.cache, made: Vec::new() };
        let border = BgCell::new(b.interior(Terrain::DeepWater, 0, 0));
        let made = std::mem::take(&mut b.made);
        tileset.note_made(made);
        // The sea beyond the map's edge is always shown.
        tileset.retain(border.tile);
        let visual = VisualMap {
            ground: TileGrid::new(map.width(), map.height(), border),
            overhead: TileGrid::new(map.width(), map.height(), BgCell::EMPTY),
        };
        (tileset, visual)
    }

    fn slot(&self, id: ChunkId) -> Option<usize> {
        let ok = (0..self.chunks.0).contains(&id.cx) && (0..self.chunks.1).contains(&id.cy);
        ok.then(|| (id.cy * self.chunks.0 + id.cx) as usize)
    }

    /// Whether a cell's tiles are built (so it may be redrawn in place).
    fn is_built(&self, x: i32, y: i32) -> bool {
        let id = ChunkId::of_cell(x, y);
        match self.slot(id).map(|i| self.built[i]) {
            Some(Built::Yes) => true,
            Some(Built::Rows(r)) => y - id.origin().1 < r,
            _ => false,
        }
    }

    /// Blocks held for the map's ground now (for the dev tools).
    pub fn resident(&self) -> usize {
        self.uses.len()
    }

    /// Keeps the ground around the screen (`view`, world px) built: what
    /// is on screen straight away, what lies just ahead a few rows a frame,
    /// and puts away chunks left far behind.
    pub fn stream(&mut self, map: &WorldMap, assets: &mut Assets, visual: &mut VisualMap, view: Rect) {
        let grow = |m: i32| Rect::new(view.x - m, view.y - m, view.w + 2 * m, view.h + 2 * m);
        let keep = grow(KEEP_MARGIN);
        for cy in 0..self.chunks.1 {
            for cx in 0..self.chunks.0 {
                let id = ChunkId::new(cx, cy);
                let i = (cy * self.chunks.0 + cx) as usize;
                if self.built[i] != Built::No && !id.rect().overlaps(&keep) {
                    self.put_away(map, visual, &mut assets.vram.metatiles, id);
                }
            }
        }
        for id in chunks_touching(grow(NEEDED_MARGIN), self.chunks).collect::<Vec<_>>() {
            self.build_rows(map, assets, visual, id, CHUNK_CELLS);
        }
        let ahead = chunks_touching(grow(PREFETCH_MARGIN), self.chunks)
            .find(|&id| self.slot(id).is_some_and(|i| self.built[i] != Built::Yes));
        if let Some(id) = ahead {
            self.build_rows(map, assets, visual, id, PREFETCH_ROWS);
        }
    }

    /// Builds up to `rows` more rows of a chunk.
    fn build_rows(&mut self, map: &WorldMap, assets: &mut Assets, visual: &mut VisualMap, id: ChunkId, rows: i32) {
        let Some(i) = self.slot(id) else { return };
        let done = match self.built[i] {
            Built::Yes => return,
            Built::No => 0,
            Built::Rows(r) => r,
        };
        let until = (done + rows).min(CHUNK_CELLS);
        let y0 = id.origin().1;
        let cells: Vec<(i32, i32)> = map.chunk_cells(id).filter(|&(_, y)| (y0 + done..y0 + until).contains(&y)).collect();
        self.built[i] = if until >= CHUNK_CELLS { Built::Yes } else { Built::Rows(until) };
        self.draw_cells(map, assets, visual, &cells);
    }

    /// Clears a chunk's cells, freeing the blocks nothing else uses.
    fn put_away(&mut self, map: &WorldMap, visual: &mut VisualMap, bank: &mut MetatileBank, id: ChunkId) {
        if let Some(i) = self.slot(id) {
            self.built[i] = Built::No;
        }
        for (x, y) in map.chunk_cells(id) {
            let (g, o) = (visual.ground.get(x, y), visual.overhead.get(x, y));
            visual.ground.set(x, y, BgCell::EMPTY);
            visual.overhead.set(x, y, BgCell::EMPTY);
            self.release(bank, g.tile, !g.is_empty());
            self.release(bank, o.tile, !o.is_empty());
        }
    }

    /// Re-derives the tiles of the given cells from the current world state
    /// (cells not built yet will be built from it when they come near).
    pub fn refresh(&mut self, map: &WorldMap, assets: &mut Assets, visual: &mut VisualMap, cells: &[(i32, i32)]) {
        let cells: Vec<(i32, i32)> = cells.iter().copied().filter(|&(x, y)| map.in_bounds(x, y) && self.is_built(x, y)).collect();
        self.draw_cells(map, assets, visual, &cells);
    }

    fn draw_cells(&mut self, map: &WorldMap, assets: &mut Assets, visual: &mut VisualMap, cells: &[(i32, i32)]) {
        let props: &PropSet = &assets.props;
        let mut b = Builder { tex: &assets.textures, bank: &mut assets.vram.metatiles, cache: &mut self.cache, made: Vec::new() };
        let mut placed = Vec::with_capacity(cells.len());
        for &(x, y) in cells {
            placed.push((x, y, cell_tiles(&mut b, props, map, x, y)));
        }
        let made = std::mem::take(&mut b.made);
        self.note_made(made);
        let bank = &mut assets.vram.metatiles;
        for (x, y, (g, o)) in placed {
            let (old_g, old_o) = (visual.ground.get(x, y), visual.overhead.get(x, y));
            visual.ground.set(x, y, g);
            visual.overhead.set(x, y, o);
            self.retain_cell(g);
            self.retain_cell(o);
            self.release(bank, old_g.tile, !old_g.is_empty());
            self.release(bank, old_o.tile, !old_o.is_empty());
        }
    }

    /// Records blocks just made (with no cells using them yet).
    fn note_made(&mut self, made: Vec<(MetatileId, Option<Key>)>) {
        for (id, key) in made {
            self.uses.insert(id, (0, key));
        }
    }

    fn retain_cell(&mut self, c: BgCell) {
        if !c.is_empty() {
            self.retain(c.tile);
        }
    }

    fn retain(&mut self, id: MetatileId) {
        if let Some((n, _)) = self.uses.get_mut(&id) {
            *n += 1;
        }
    }

    /// One cell fewer uses the block; freed once none do. Fixed blocks
    /// (loaded with the assets) are never freed.
    fn release(&mut self, bank: &mut MetatileBank, id: MetatileId, used: bool) {
        if !used || (id as usize) < self.first_made {
            return;
        }
        let Some((n, key)) = self.uses.get_mut(&id) else { return };
        *n = n.saturating_sub(1);
        if *n == 0 {
            if let Some(key) = key.take() {
                self.cache.remove(&key);
            }
            self.uses.remove(&id);
            bank.remove(id);
        }
    }
}

/// BG1 and BG2 cells for one map cell.
fn cell_tiles(b: &mut Builder, props: &PropSet, map: &WorldMap, x: i32, y: i32) -> (BgCell, BgCell) {
    let mut overhead = BgCell::EMPTY;
    let river = RiverField::sample(map, x, y);
    let tile = match map.feature(x, y) {
        Feature::Palm(p) => {
            let col = if p.flip { PalmPart::W as u8 - 1 - p.dx } else { p.dx } as usize;
            overhead = palm_overhead(props, map, x, y, p, 0);
            match p.dy {
                0 | 1 => b.terrain(map, x, y),
                2 => b.prop(10 + col as u8, &props.palm.shade[col], p.flip, x, y),
                _ => b.prop(13 + col as u8, &props.palm.lower[col], p.flip, x, y),
            }
        }
        Feature::Stump(p) => b.prop(16, &props.stump, p.flip, x, y),
        Feature::Broadleaf(p) => {
            // Crown and upper trunk on BG2 in front of sprites; the trunk's
            // foot and the shadows on the grass (BG1).
            let col = if p.flip { BroadleafPart::W as u8 - 1 - p.dx } else { p.dx } as usize;
            match p.dy {
                3 => {
                    let stripped = map.bark(p.anchor(x, y)) < BARK_STRIPS;
                    b.prop(22 + col as u8 + 3 * stripped as u8, &props.broadleaf_base[stripped as usize][col], p.flip, x, y)
                }
                dy => {
                    overhead = BgCell { tile: props.broadleaf_upper[dy as usize * 3 + col], hflip: p.flip, high: true };
                    if dy == 2 { b.prop(19 + col as u8, &props.broadleaf_shade[col], p.flip, x, y) } else { b.terrain(map, x, y) }
                }
            }
        }
        Feature::BroadleafStump(s) => b.prop(28, &props.broadleaf_stump, s.flip, x, y),
        Feature::Rock => b.prop(0, &props.boulder, false, x, y),
        Feature::Driftwood(p) => {
            // A mirrored log swaps its halves (and flips each).
            let half = if p.flip { 1 - p.dx } else { p.dx } as usize;
            b.prop([2, 1][half], &props.driftwood[half], p.flip, x, y)
        }
        Feature::Shell => b.prop(3, &props.shell, y % 2 == 0, x, y),
        Feature::Bush => b.prop(4, &props.bush, false, x, y),
        Feature::Vine => b.prop(17, &props.vine, x % 2 == 0, x, y),
        Feature::VineStake => b.prop(18, &props.vine_stake, x % 2 == 0, x, y),
        Feature::Flower => b.prop(5, &props.flowers, x % 2 == 1, x, y),
        Feature::None => match &river {
            Some(r) => b.river_bank(map, &CellField::sample(map, x, y), r, x, y),
            None => b.terrain(map, x, y),
        },
    };
    if let Some(r) = river.filter(|r| r.max > 0.0 && overhead.is_empty()) {
        overhead = BgCell { tile: b.river_water(&r, x, y), hflip: false, high: false };
    }
    (BgCell::new(tile), overhead)
}

/// BG2 cell of a palm part, with the canopy shifted `sway` px (the row
/// above the trunk base bends half as far). Mirrored trees swap columns and
/// use the PPU's h-flip, so they take the opposite shift.
fn palm_overhead(props: &PropSet, map: &WorldMap, x: i32, y: i32, p: PalmPart, sway: i32) -> BgCell {
    let dy = p.dy as usize;
    if dy >= 3 {
        return BgCell::EMPTY;
    }
    let col = if p.flip { PalmPart::W as u8 - 1 - p.dx } else { p.dx } as usize;
    let dx = if dy == 2 { sway / 2 } else { sway };
    let dx = (if p.flip { -dx } else { dx }).clamp(-PALM_SWAY_MAX, PALM_SWAY_MAX);
    let tiles = &props.palm_tiles[(dx + PALM_SWAY_MAX) as usize];
    let tile = if p.is_crown() {
        tiles.crowns[(map.fruit(p.anchor(x, y)) as usize).min(tiles.crowns.len() - 1)]
    } else {
        tiles.upper[dy * 3 + col]
    };
    BgCell { tile, hflip: p.flip, high: true }
}

/// Redraws the canopy of the palm at `anchor` shifted `sway` px, leaving
/// every other tree alone.
pub fn sway_palm(map: &WorldMap, props: &PropSet, visual: &mut VisualMap, anchor: (i32, i32), sway: i32) {
    for dy in 0..3 {
        for dx in 0..PalmPart::W {
            let (x, y) = (anchor.0 + dx, anchor.1 + dy);
            if let Feature::Palm(p) = map.feature(x, y) {
                visual.overhead.set(x, y, palm_overhead(props, map, x, y, p, sway));
            }
        }
    }
}

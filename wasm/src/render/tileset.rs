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
use crate::ppu::banks::{MetatileBank, MetatileId};
use crate::ppu::pattern::PixelArt;
use crate::ppu::{BgCell, BgLayer};
use crate::world::island::{DEEP_BELOW, SAND_BELOW};
use crate::world::noise::value_noise;
use crate::world::harvest::BARK_STRIPS;
use crate::world::terrain::{BroadleafPart, PalmPart};
use crate::world::{Feature, TILE_SIZE, Terrain, WorldMap};

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
}

impl Builder<'_> {
    fn get(&mut self, key: Key, make: impl FnOnce(&TerrainTextures, &mut MetatileBank) -> MetatileId) -> MetatileId {
        if let Some(&id) = self.cache.get(&key) {
            return id;
        }
        let id = make(self.tex, self.bank);
        self.cache.insert(key, id);
        id
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
                    } else if in_river > 0.0 && h > -1.4 {
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
        self.bank.add(PAL_SHORE, frames, SURF_PERIOD)
    }

    /// Open shallows deepening toward the reef edge.
    fn shallows(&mut self, f: &CellField, x: i32, y: i32) -> MetatileId {
        let water = slice(&self.tex.shallow, x, y);
        let art = compose(|px, py| shallow_depth(water.get(px, py), f.at(px, py), px as i32, py as i32));
        self.bank.add_art(PAL_SHORE, &art)
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
        self.bank.add_art(PAL_DEEP, &art)
    }

    /// Grass giving way to sand with a ragged fringe.
    fn grass_edge(&mut self, f: &CellField, x: i32, y: i32) -> MetatileId {
        let (g, s) = self.grass_and_sand(x, y);
        let art = compose(|px, py| grass_or_sand(f, &g, &s, x, y, px, py));
        self.bank.add_art(PAL_GRASS, &art)
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
        self.bank.add_art(PAL_GRASS, &art)
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
        self.bank.add_art(PAL_DEEP, &art)
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

/// Owns the tile cache so cells can be rebuilt when the world changes
/// (e.g. a tree is felled); new tiles go into VRAM like streamed CHR.
#[derive(Default)]
pub struct Tileset {
    cache: HashMap<Key, MetatileId>,
}

impl Tileset {
    pub fn build(&mut self, map: &WorldMap, assets: &mut Assets) -> VisualMap {
        let props: &PropSet = &assets.props;
        let mut b = Builder { tex: &assets.textures, bank: &mut assets.vram.metatiles, cache: &mut self.cache };
        let border = BgCell::new(b.interior(Terrain::DeepWater, 0, 0));
        let mut visual = VisualMap {
            ground: TileGrid::new(map.width(), map.height(), border),
            overhead: TileGrid::new(map.width(), map.height(), BgCell::EMPTY),
        };
        for y in 0..map.height() {
            for x in 0..map.width() {
                let (g, o) = cell_tiles(&mut b, props, map, x, y);
                visual.ground.set(x, y, g);
                visual.overhead.set(x, y, o);
            }
        }
        visual
    }

    /// Re-derives the tiles of the given cells from the current world state.
    pub fn refresh(&mut self, map: &WorldMap, assets: &mut Assets, visual: &mut VisualMap, cells: &[(i32, i32)]) {
        let props: &PropSet = &assets.props;
        let mut b = Builder { tex: &assets.textures, bank: &mut assets.vram.metatiles, cache: &mut self.cache };
        for &(x, y) in cells {
            if map.in_bounds(x, y) {
                let (g, o) = cell_tiles(&mut b, props, map, x, y);
                visual.ground.set(x, y, g);
                visual.overhead.set(x, y, o);
            }
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

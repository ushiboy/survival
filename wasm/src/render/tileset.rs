//! Builds the BG1 (ground) and BG2 (overhead) tile maps from world data.
//!
//! Ground textures are 64x64 seamless canvases sliced per cell. Where a
//! terrain boundary passes through a cell, the tile is painted per pixel from
//! the smooth elevation field, so coastlines curve freely instead of
//! following the 16px grid. Every tile is composed once at load time; the
//! runtime cost is a table lookup.

use std::collections::HashMap;

use crate::assets::paint::{bayer, noise_px};
use crate::assets::palettes::{PAL_DEEP, PAL_FOLIAGE, PAL_GRASS, PAL_ROCK, PAL_SAND, PAL_SHORE, deep, foliage, grass, rock, sand, shore};
use crate::assets::props::{Ground, PropTile, composite};
use crate::assets::terrain::{TerrainTextures, sand_to, sand_to_shore, slice};
use crate::assets::{Assets, PALM_SWAY_MAX, PropSet, SURF_FRAMES, SURF_PERIOD};
use crate::common::math::Rect;
use crate::common::rng::hash_unit;
use crate::game::mill::post_head;
use crate::ppu::banks::{MetatileBank, MetatileId};
use crate::ppu::pattern::PixelArt;
use crate::ppu::pattern::Tile16;
use crate::ppu::{BgCell, BgLayer};
use crate::world::chunk::chunks_touching;
use crate::world::harvest::BARK_STRIPS;
use crate::world::island::{DEEP_BELOW, SAND_BELOW};
use crate::world::map::LADDER_ROWS;
use crate::world::noise::value_noise;
use crate::world::terrain::{BroadleafPart, ClayPart, Marking, PalmPart, PinePart, Soil, patch_anchor};
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
        if mx < 0 || my < 0 || mx >= self.w || my >= self.h { self.border } else { self.cells[(my * self.w + mx) as usize] }
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
    /// Depth in the channel alone (the pool under the falls left out).
    channel: [f32; 256],
    along: [f32; 256],
    across: [f32; 256],
    max: f32,
}

impl RiverField {
    /// The cell's river field, if the water or its bank reaches into it.
    fn sample(map: &WorldMap, cx: i32, cy: i32) -> Option<Self> {
        let river = map.river.as_ref().filter(|_| map.is_riverside(cx, cy))?;
        let ts = TILE_SIZE as f32;
        let mut r = Self { depth: [0.0; 256], channel: [0.0; 256], along: [0.0; 256], across: [0.0; 256], max: f32::MIN };
        // Every pixel centre, in the cells the river's path is laid out in.
        let (x, y) = (cx as f32, cy as f32);
        let near = river.around((x - 0.5, y - 0.5), (x + 0.5, y + 0.5));
        for i in 0..256 {
            let (wx, wy) = ((cx * TILE_SIZE) as f32 + (i % 16) as f32 + 0.5, (cy * TILE_SIZE) as f32 + (i / 16) as f32 + 0.5);
            let p = near.at_px(wx, wy);
            // A slight wobble so the banks don't look ruled.
            let wobble = value_noise(wx / 6.0, wy / 6.0, 91) * 1.4;
            let pool = river.pool_depth(wx / ts - 0.5, wy / ts - 0.5);
            r.channel[i] = p.depth() * ts + wobble;
            r.depth[i] = p.depth().max(pool) * ts + wobble;
            r.along[i] = p.along * ts;
            r.across[i] = p.across * ts;
            r.max = r.max.max(r.depth[i]);
        }
        // Cells the trampled landing reaches are drawn as bank too.
        let near_spot = map.water_spots.iter().any(|&(sx, sy)| {
            let (dx, dy) = ((cx * TILE_SIZE + 8 - sx) as f32, (cy * TILE_SIZE + 8 - sy) as f32);
            dx.abs() < LANDING_RADIUS + 8.0 && dy.abs() < LANDING_RADIUS + 8.0
        });
        (r.max > BANK_LIP || near_spot).then_some(r)
    }
}

/// Depth (below the waterline) down to which the river churns the surf
/// where it meets the sea; cells all deeper are plain sea.
const RIVER_CHURN_BELOW: f32 = -1.4;
/// Depth below which the river's water turns to the dark sea of the current
/// (fully so by `RIVER_CHURN_BELOW`).
const SEA_CURRENT_FROM: f32 = -0.4;

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
    /// A cliff face: row from the top, rows in all, open to the left and
    /// right, water at its foot, and the column.
    Cliff {
        row: i32,
        rows: i32,
        left: bool,
        right: bool,
        wet: bool,
        grassy: bool,
        vx: i32,
    },
    /// The edge of the high ground: which sides drop away (N, E, S, W).
    CliffTop {
        open: [bool; 4],
        vx: i32,
        vy: i32,
    },
    /// The side of the high ground seen from above, dropping away to the
    /// left or right.
    CliffSide {
        left: bool,
        right: bool,
        vx: i32,
        vy: i32,
    },
    /// The same side as a narrow band of rock along the drop, clear
    /// elsewhere for the grass on BG2 to show through.
    CliffSideBand {
        left: bool,
        right: bool,
        vx: i32,
        vy: i32,
    },
    Prop {
        kind: u8,
        flip: bool,
        vx: i32,
        vy: i32,
    },
    /// The lake shore's wet soil, away from the water and the grass.
    Mud(i32, i32),
    /// The top of a crag: which sides drop away (N, E, S, W), its tier, and
    /// which of its sides (N, W) a higher tier shades.
    Crag {
        open: [bool; 4],
        tier: u8,
        shade: [bool; 2],
        slope: [bool; 2],
        vx: i32,
        vy: i32,
    },
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
            Terrain::Grass | Terrain::Highland | Terrain::CliffTop => bank.add_art(PAL_GRASS, &slice(&tex.grass, vx, vy)),
            Terrain::Cliff | Terrain::Crag => bank.add_art(PAL_ROCK, &PixelArt::filled(16, 16, SANDSTONE[2])),
            Terrain::Slope => bank.add_art(PAL_GRASS, &PixelArt::filled(16, 16, grass::SOIL)),
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
                        // Churn where the current meets the sea, broken up.
                        let wx = x * 16 + ix;
                        let wy = y * 16 + iy;
                        // Between the foam, the dark current (BG2) shows.
                        let n = noise_px(wx / 2, wy / 2, 0xC4A2);
                        if n < 0.3 {
                            shore::FOAM
                        } else if n < 0.45 {
                            shore::FOAM_SHADE
                        } else {
                            0
                        }
                    } else if in_river > 0.0 {
                        // The current runs on out to sea on BG2.
                        0
                    } else if h > 0.0 && river_wet(in_river, x * 16 + ix, y * 16 + iy) > -WET_BANK {
                        // Sand soaked by the river, darkest at the water.
                        let wet = river_wet(in_river, x * 16 + ix, y * 16 + iy);
                        if wet > -4.0 || (wet > -WET_BANK + 2.0 && bayer(ix, iy) < 0.5) { shore::WET_DARK } else { shore::WET }
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
        let art = compose(|px, py| reef_px(base.get(px, py), f.at(px, py), px as i32, py as i32));
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
            // Off the mouth, out at sea: the water as anywhere else, with
            // the current running through it where it is too strong to
            // wade.
            let t = map.terrain(x, y);
            return if t == Terrain::River || (t == Terrain::DeepWater && r.max > 0.0) { self.current(map, f, r, x, y) } else { self.terrain(map, x, y) };
        }
        if f.min < 0.6 {
            // Out on the beach the river runs into the surf.
            return self.coast(f, x, y, Some(r));
        }
        let spot = map.nearest_water_spot((x * 16 + 8) as f32, (y * 16 + 8) as f32);
        let grassy = |px: usize, py: usize| {
            let (wx, wy) = (x * 16 + px as i32, y * 16 + py as i32);
            (f.at(px, py) - SAND_BELOW) * PX_PER_UNIT + noise_px(wx, wy, 0x6EED) * 1.6 >= 0.0
        };
        let far_from_spot = spot.is_none_or(|(sx, sy)| (sx - (x * 16 + 8) as f32).abs().max((sy - (y * 16 + 8) as f32).abs()) > 40.0);
        if far_from_spot && (0..16).all(|py| (0..16).all(|px| !grassy(px, py))) {
            if map.is_high(x, y) && near_mud(map, x, y) {
                return self.muddy_bank(r, x, y);
            }
            return self.sandy_bank(r, x, y);
        }
        let art = self.grass_bank_art(map, f, r, x, y);
        self.unique_art(PAL_GRASS, &art)
    }

    /// A grassy river bank (grass palette), the water left clear for the
    /// river drawn on BG2.
    fn grass_bank_art(&mut self, map: &WorldMap, f: &CellField, r: &RiverField, x: i32, y: i32) -> PixelArt {
        let spot = map.nearest_water_spot((x * 16 + 8) as f32, (y * 16 + 8) as f32);
        let grassy = |px: usize, py: usize| {
            let (wx, wy) = (x * 16 + px as i32, y * 16 + py as i32);
            (f.at(px, py) - SAND_BELOW) * PX_PER_UNIT + noise_px(wx, wy, 0x6EED) * 1.6 >= 0.0
        };
        let (g, s) = self.grass_and_sand(x, y);
        // Through the lake's shore the bank is soil, not sand.
        let muddy = map.is_high(x, y) && near_mud(map, x, y);
        let s = if muddy { s.map(|_| grass::SOIL) } else { s };
        // The bank's lip: dark soil by the lake, sand elsewhere.
        let lip = if muddy { [grass::RAMP[0], grass::SOIL] } else { [grass::SAND[0], grass::SAND[1]] };
        let ramp = grass::RAMP;
        compose(|px, py| {
            let i = py * 16 + px;
            let d = r.depth[i];
            if d > 0.0 {
                return 0;
            }
            let (wx, wy) = (x * 16 + px as i32, y * 16 + py as i32);
            let grassy = grassy(px, py);
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
                        return if sd > 2.1 {
                            grass::SAND[0]
                        } else if lit {
                            grass::WHITE
                        } else {
                            grass::SAND[2]
                        };
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
                if grassy { grass::SOIL } else { lip[0] }
            } else if !grassy && river_wet(d, wx, wy) > -WET_BANK {
                // Sand (or soil) soaked by the river.
                if river_wet(d, wx, wy) > -2.5 || b < 0.5 { lip[0] } else { lip[1] }
            } else if grassy && d > -3.0 {
                // The turf overhangs the cut bank in a dark edge.
                if b < 0.5 { ramp[0] } else { ramp[1] }
            } else if grassy && d > -BANK_LIP.abs() {
                if b < 0.5 { ramp[1] } else { grass_or_sand(f, &g, &s, x, y, px, py) }
            } else {
                grass_or_sand(f, &g, &s, x, y, px, py)
            }
        })
    }

    /// A log bridge laid across the river (grass palette, in its earth and
    /// sand browns, over the bank and the water left clear for BG2): `part`
    /// `0` its west end on the bank, `1` over the water, `2` its east end.
    fn bridge(&mut self, map: &WorldMap, river: Option<&RiverField>, x: i32, y: i32, part: u8) -> MetatileId {
        let mut art = match river {
            Some(r) => self.grass_bank_art(map, &CellField::sample(map, x, y), r, x, y),
            None => slice(&self.tex.grass, x.rem_euclid(4), y.rem_euclid(4)),
        };
        for py in 0..16 {
            for px in 0..16 {
                match bridge_pixel(px as i32, py as i32, part) {
                    Some(v) => art.set(px, py, v),
                    // Its shadow below it, dithered over the water.
                    None if py as i32 == BRIDGE_ROWS.1 + 1
                        && bridge_pixel(px as i32, BRIDGE_ROWS.1, part).is_some()
                        && (art.get(px, py) != 0 || (px + py) % 2 == 0) =>
                    {
                        art.set(px, py, grass::RAMP[0])
                    }
                    None => {}
                }
            }
        }
        self.unique_art(PAL_GRASS, &art)
    }

    /// A quarter of a bed of clay in the lake shore's mud (rock palette):
    /// a grey, wet-sheened slab, or, dug today, a dark hole cut in it.
    fn clay_bed(&mut self, x: i32, y: i32, c: ClayPart) -> MetatileId {
        let mud = slice(&self.tex.mud, x.rem_euclid(4), y.rem_euclid(4));
        let anchor = patch_anchor(x, y, c.part);
        let (ox, oy) = ((c.part % 2) as i32 * 16, (c.part / 2) as i32 * 16);
        let art = compose(|px, py| clay_bed_pixel(anchor, ox + px as i32, oy + py as i32, c.dug, mud.get(px, py)));
        self.unique_art(PAL_ROCK, &art)
    }

    /// A quarter of a patch of bare earth in the grass: the grass (BG1)
    /// worn through where the earth shows, and the earth (BG2, rock
    /// palette, whose browns the grass palette lacks) beneath it.
    fn bare_earth(&mut self, x: i32, y: i32, part: u8) -> (MetatileId, MetatileId) {
        let grass = slice(&self.tex.grass, x.rem_euclid(4), y.rem_euclid(4));
        let anchor = patch_anchor(x, y, part);
        let (ox, oy) = ((part % 2) as i32 * 16, (part / 2) as i32 * 16);
        let top = compose(|px, py| if earth_shows(anchor, ox + px as i32, oy + py as i32) { 0 } else { grass.get(px, py) });
        let earth = compose(|px, py| bare_earth_pixel(anchor, ox + px as i32, oy + py as i32));
        (self.unique_art(PAL_GRASS, &top), self.unique_art(PAL_ROCK, &earth))
    }

    /// A quarter of the water wheel standing in the river (grass palette,
    /// over the river's banks as a bridge is). Its axle runs east to west,
    /// so the wheel turns with the current: seen from above, two rims of
    /// split bamboo run along the river with the paddles across between
    /// them, the paddles on top going round upstream frame by frame (the
    /// ones in the water below are pushed downstream), and the axle reaches
    /// back to the west bank.
    fn water_wheel(&mut self, map: &WorldMap, river: Option<&RiverField>, x: i32, y: i32, part: u8) -> MetatileId {
        let base = match river {
            Some(r) => self.grass_bank_art(map, &CellField::sample(map, x, y), r, x, y),
            None => slice(&self.tex.grass, x.rem_euclid(4), y.rem_euclid(4)),
        };
        let (ox, oy) = ((part % 2) as i32 * 16, (part / 2) as i32 * 16);
        let frames = (0..WHEEL_FRAMES)
            .map(|f| {
                let mut art = base.clone();
                for py in 0..16 {
                    for px in 0..16 {
                        let (bx, by) = (ox + px as i32, oy + py as i32);
                        let c = water_wheel_top(bx, by, f).or_else(|| {
                            // Outlined against the water.
                            [(-1, 0), (1, 0), (0, -1), (0, 1)].iter().any(|&(dx, dy)| water_wheel_top(bx + dx, by + dy, f).is_some()).then_some(grass::RAMP[0])
                        });
                        if let Some(c) = c {
                            art.set(px, py, c);
                        }
                    }
                }
                art.to_tile16()
            })
            .collect();
        self.unique(PAL_GRASS, frames, WHEEL_PERIOD)
    }

    /// A quarter of the wood lathe on the bank beside the water wheel
    /// (grass palette, over the bank as the wheel is), seen from above at
    /// the slant everything is: its bed running along the bank's edge, the
    /// tailstock and the headstock holding a billet between them, and the
    /// rope drive running from a pulley on the wheel's axle to one on the
    /// headstock's spindle.
    fn lathe(&mut self, map: &WorldMap, river: Option<&RiverField>, x: i32, y: i32, part: u8) -> MetatileId {
        let mut art = match river {
            Some(r) => self.grass_bank_art(map, &CellField::sample(map, x, y), r, x, y),
            None => slice(&self.tex.grass, x.rem_euclid(4), y.rem_euclid(4)),
        };
        // Once the mill's foundation is laid round it, it stands on the
        // tamped earth.
        let laid = map.spots.mill.is_some_and(|site| {
            let (x0, y0, _, _) = crate::world::map::mill_plot(site);
            matches!(map.feature(x0, y0), Feature::Foundation(_) | Feature::Framed(_))
        });
        if laid {
            tamp(&mut art, map, x, y);
        }
        let (ox, oy) = ((part % 2) as i32 * 16, (part / 2) as i32 * 16);
        for py in 0..16 {
            for px in 0..16 {
                let (bx, by) = (ox + px as i32, oy + py as i32);
                let c = lathe_top(bx, by).or_else(|| {
                    let edge = [(-1, 0), (1, 0), (0, -1), (0, 1)].iter().any(|&(dx, dy)| lathe_top(bx + dx, by + dy).is_some());
                    // Outlined, its shadow cast down onto the grass below.
                    let shadow = (1..=2).any(|d| lathe_top(bx, by - d).is_some()) && (bx + by) % 2 == 0;
                    (edge || shadow).then_some(grass::RAMP[0])
                });
                if let Some(c) = c {
                    art.set(px, py, c);
                }
            }
        }
        // The rope marking out the mill's ground runs on along its east
        // edge over the lathe (it stands on that edge), unbroken from the
        // stakes north and south of it.
        let on_edge = map.spots.mill.is_some_and(|site| x == crate::world::map::mill_plot(site).2);
        if on_edge && map.is_staked() {
            let rope = Marking { stake: false, ropes: Marking::NORTH | Marking::SOUTH };
            for py in 0..16 {
                for px in 0..16 {
                    if let Some(c) = marking_rope(rope, px as i32, py as i32) {
                        art.set(px, py, c);
                    }
                }
            }
        }
        self.unique_art(PAL_GRASS, &art)
    }

    /// The mill's ground marked out (grass palette): rope lying along the
    /// grass through the cell's middle, out each way it runs, and at a
    /// corner the stake it's tied to, outlined, its shadow on the grass.
    fn marking(&mut self, map: &WorldMap, river: Option<&RiverField>, x: i32, y: i32, m: Marking) -> MetatileId {
        let mut art = match river {
            Some(r) => self.grass_bank_art(map, &CellField::sample(map, x, y), r, x, y),
            None => slice(&self.tex.grass, x.rem_euclid(4), y.rem_euclid(4)),
        };
        let top = |px: i32, py: i32| marking_top(m, px, py);
        for py in 0..16 {
            for px in 0..16 {
                let c = top(px, py).or_else(|| {
                    let edge = m.stake && [(-1, 0), (1, 0), (0, -1), (0, 1)].iter().any(|&(dx, dy)| marking_stake(px + dx, py + dy).is_some());
                    // The rope's shadow just under it, the stake's cast
                    // down to the right.
                    let shadow = top(px, py - 1).is_some() && marking_rope(m, px, py - 1).is_some() && marking_stake(px, py - 1).is_none()
                        || m.stake && (1..=2).any(|d| marking_stake(px - d, py - d).is_some()) && (px + py) % 2 == 0;
                    (edge || shadow).then_some(grass::RAMP[0])
                });
                if let Some(c) = c {
                    art.set(px as usize, py as usize, c);
                }
            }
        }
        self.unique_art(PAL_GRASS, &art)
    }

    /// A cell of the mill's ground once its frame is up (grass palette): the
    /// foundation as it was, and where a post stands, its stone, until the
    /// walls are up and hide them (what spans between the posts' heads is on
    /// BG2, `mill_frame_overhead`).
    fn framed(&mut self, map: &WorldMap, river: Option<&RiverField>, x: i32, y: i32, post: bool, wall: bool) -> MetatileId {
        let mut art = match river {
            Some(r) => self.grass_bank_art(map, &CellField::sample(map, x, y), r, x, y),
            None => slice(&self.tex.grass, x.rem_euclid(4), y.rem_euclid(4)),
        };
        tamp(&mut art, map, x, y);
        // The posts are sprites (`Game::draw_mill_posts`), sorted with the
        // castaway by their feet; here, the stones they stand on, the foot
        // of a wall's face along the north and south edges (the rest of it
        // rises on BG2), and the line of a side wall on the ground.
        let (x0, y0, x1, y1) = map.spots.mill.map_or((0, 0, 0, 0), crate::world::map::mill_plot);
        let side = wall && (x == x0 || x == x1) && y != y0 && y != y1;
        let face_row = (y == y0).then_some(true).or((y == y1).then_some(false));
        let top = |px: i32, py: i32| {
            let face = face_row.and_then(|north| wall_face_px(map, x * 16 + px, y * 16 + py).filter(|&(_, n)| n == north).map(|(c, _)| c));
            let stone = if post && !map.has_mill_walls() { foundation_stone(px, py) } else { None };
            let line = (side && (6..10).contains(&px)).then(|| plaster(x * 16 + px, y * 16 + py));
            stone.or(face).or(line)
        };
        paint_outlined(&mut art, top, grass::SOIL);
        self.unique_art(PAL_GRASS, &art)
    }

    /// What of the mill's frame spans over the cell (grass palette, BG2):
    /// the plates (the beams are sprites), or once it's on, the roof over
    /// them; `None` if nothing (or the frame isn't up).
    fn mill_frame_overhead(&mut self, map: &WorldMap, x: i32, y: i32) -> Option<MetatileId> {
        let (x0, y0, x1, y1) = crate::world::map::mill_plot(map.spots.mill?);
        if !(x0..=x1).contains(&x) || !(y0 - 1..y1).contains(&y) || !map.has_mill_frame() {
            return None;
        }
        // The posts are sprites, sorted with the castaway by their feet;
        // what spans between their heads, higher than anyone's head, is in
        // front of everyone, and so are the walls' faces above the rows
        // they stand on (the north wall's over someone behind it, outside;
        // the south wall's over someone behind it, inside).
        let top = |px: i32, py: i32| {
            let (wx, wy) = (x * 16 + px, y * 16 + py);
            let face = wall_face_px(map, wx, wy).filter(|&(_, north)| y < if north { y0 } else { y1 }).map(|(c, _)| c);
            if map.shows_mill_roof() {
                return roof_px(map, wx, wy).or(face);
            }
            span_px(map, wx, wy).or(face)
        };
        let mut art = PixelArt::filled(16, 16, 0);
        let any = paint_outlined(&mut art, top, grass::SOIL);
        any.then(|| self.unique_art(PAL_GRASS, &art))
    }

    /// The bench beside the mill's ground (grass palette): a plank laid on
    /// two stones, the plan (a pale board scratched with lines) and a hank
    /// of rope on it, outlined, its shadow on the grass.
    fn site_bench(&mut self, x: i32, y: i32) -> MetatileId {
        let mut art = slice(&self.tex.grass, x.rem_euclid(4), y.rem_euclid(4));
        for py in 0..16i32 {
            for px in 0..16i32 {
                let c = site_bench_top(px, py).or_else(|| {
                    let edge = [(-1, 0), (1, 0), (0, -1), (0, 1)].iter().any(|&(dx, dy)| site_bench_top(px + dx, py + dy).is_some());
                    let shadow = (1..=2).any(|d| site_bench_top(px - 1, py - d).is_some()) && (px + py) % 2 == 0;
                    (edge || shadow).then_some(grass::RAMP[0])
                });
                if let Some(c) = c {
                    art.set(px as usize, py as usize, c);
                }
            }
        }
        self.unique_art(PAL_GRASS, &art)
    }

    /// The mill's foundation (grass palette): the grass worn away to earth
    /// tamped hard (a few tufts left along its outer edge, and the bank's
    /// lip kept by the river), and where a post will stand, a flat stone
    /// bedded in it.
    fn foundation(&mut self, map: &WorldMap, river: Option<&RiverField>, x: i32, y: i32, stone: bool) -> MetatileId {
        let mut art = match river {
            Some(r) => self.grass_bank_art(map, &CellField::sample(map, x, y), r, x, y),
            None => slice(&self.tex.grass, x.rem_euclid(4), y.rem_euclid(4)),
        };
        tamp(&mut art, map, x, y);
        if stone {
            let top = |px: i32, py: i32| foundation_stone(px, py);
            for py in 0..16i32 {
                for px in 0..16i32 {
                    let c = top(px, py).or_else(|| {
                        let edge = [(-1, 0), (1, 0), (0, -1), (0, 1)].iter().any(|&(dx, dy)| top(px + dx, py + dy).is_some());
                        edge.then_some(grass::SOIL)
                    });
                    if let Some(c) = c {
                        art.set(px as usize, py as usize, c);
                    }
                }
            }
        }
        self.unique_art(PAL_GRASS, &art)
    }

    /// Beach along the river, inland of the surf (sand palette): a cut lip
    /// at the water, sand darkened by the wet in a ragged band, and
    /// pebbles the river has washed down.
    /// The lake's water where the stream's current runs out across it
    /// (shore palette): clear where the current is, for BG2's river.
    fn lake_current(&mut self, f: &CellField, r: &RiverField, x: i32, y: i32) -> MetatileId {
        let water = slice(&self.tex.shallow, x, y);
        let art = compose(|px, py| if r.depth[py * 16 + px] > 0.0 { 0 } else { lake_edge_pixel(f, &water, false, px, py) });
        self.unique_art(PAL_SHORE, &art)
    }

    /// Where the river runs out of the lake (shore palette): the lake's
    /// calm water, with the stream running out over the shore in the same
    /// water; the shore's soil left clear for BG2 (returned too), as along
    /// the rest of the lake.
    fn lake_mouth(&mut self, f: &CellField, r: &RiverField, x: i32, y: i32) -> (MetatileId, MetatileId) {
        let soil = self.mud_under(f, x, y);
        if f.min < 0.0 {
            // Where the lake reaches in (shore palette), its shallows
            // darken into the stream's mouth.
            let water = slice(&self.tex.shallow, x, y);
            let art = compose(|px, py| {
                let d = r.depth[py * 16 + px];
                let h = f.at(px, py);
                if d > 0.0 {
                    // The stream's current, in the blue it starts in (see
                    // `lake_blend_at`).
                    shore::WATER[0]
                } else if h < 0.0 {
                    lake_edge_pixel(f, &water, true, px, py)
                } else {
                    0
                }
            });
            return (self.unique_art(PAL_SHORE, &art), soil);
        }
        // Deep palette: it has the river's colours, so the stream runs on
        // unbroken out of the lake.
        let art = compose(|px, py| {
            let d = r.depth[py * 16 + px];
            let (wx, wy) = (x * 16 + px as i32, y * 16 + py as i32);
            if d > 0.0 {
                if lake_blend_at(r.along[py * 16 + px], bayer(wx, wy)) {
                    deep::SHELF[0]
                } else if d < 2.0 {
                    deep::RIVER_SHADE
                } else if noise_px(wx / 2, (wy - r.along[py * 16 + px] as i32) / 4, 0x5E1A) < 0.2 {
                    deep::RIVER_FLOW[1]
                } else {
                    deep::RIVER_BODY
                }
            } else {
                // The shore's soil (BG2).
                0
            }
        });
        (self.unique_art(PAL_DEEP, &art), soil)
    }

    /// The river's bank where it runs out of the lake through the shore's
    /// wet soil (rock palette): the soil darkening to the water.
    fn muddy_bank(&mut self, r: &RiverField, x: i32, y: i32) -> MetatileId {
        let tex = slice(&self.tex.mud, x, y);
        let art = compose(|px, py| {
            let d = r.depth[py * 16 + px];
            if d > 0.0 {
                return 0;
            }
            let (wx, wy) = (x * 16 + px as i32, y * 16 + py as i32);
            let b = bayer(wx, wy);
            if let Some(shade) = pebble(d, wx, wy) {
                return rock::STONE[2 + shade];
            }
            let wet = river_wet(d, wx, wy);
            if d > -1.5 || wet > -3.0 || (wet > -6.0 && b < 0.5) { rock::WOOD[0] } else { tex.get(px, py) }
        });
        self.unique_art(PAL_ROCK, &art)
    }

    fn sandy_bank(&mut self, r: &RiverField, x: i32, y: i32) -> MetatileId {
        let tex = slice(&self.tex.sand, x, y);
        let art = compose(|px, py| {
            let d = r.depth[py * 16 + px];
            if d > 0.0 {
                return 0;
            }
            let (wx, wy) = (x * 16 + px as i32, y * 16 + py as i32);
            let b = bayer(wx, wy);
            if let Some(shade) = pebble(d, wx, wy) {
                return sand::PEBBLE[shade];
            }
            let wet = river_wet(d, wx, wy);
            if d > -1.5 {
                if b < 0.5 { sand::DEBRIS } else { sand::RAMP[0] }
            } else if wet > -3.0 {
                if b < 0.5 { sand::DEBRIS } else { sand::WET }
            } else if wet > -5.0 {
                if b < 0.25 { sand::DEBRIS } else { sand::WET }
            } else if wet > -WET_BANK + 1.5 {
                sand::WET
            } else if wet > -WET_BANK {
                if b < 0.5 { sand::WET } else { tex.get(px, py) }
            } else {
                tex.get(px, py)
            }
        });
        self.unique_art(PAL_SAND, &art)
    }

    /// Ground at an inner corner of the foot of a slanting cliff (see
    /// `WorldMap::foot_fill`): the rock of the face beside it carried down to a
    /// slanting foot. The rock is drawn on BG2 (rock palette) under a hole
    /// cut in the ground on BG1, which also takes a thin shadow along the
    /// slant. `None` where nothing is filled.
    fn cliff_foot(&mut self, map: &WorldMap, x: i32, y: i32) -> Option<(MetatileId, MetatileId)> {
        let [fill_left, fill_right] = map.foot_fill(x, y);
        if !fill_left && !fill_right {
            return None;
        }
        let t = map.terrain(x, y);
        let f = CellField::sample(map, x, y);
        let (palette, ground) = if f.crosses(SAND_BELOW, 0.35, 0.3) {
            let (g, s) = self.grass_and_sand(x, y);
            (PAL_GRASS, compose(|px, py| grass_or_sand(&f, &g, &s, x, y, px, py)))
        } else if t == Terrain::Sand {
            (PAL_SAND, slice(&self.tex.sand, x, y))
        } else {
            (PAL_GRASS, slice(&self.tex.grass, x, y))
        };
        // How far into the rock a pixel is, across the slant (px; negative
        // on the ground), and the face whose rock it carries on.
        // The slant wavers a pixel either way along its length, so it
        // isn't ruled straight.
        let (x0, y0) = (x * 16, y * 16);
        let waver = |along: i32| (noise_px(along.div_euclid(3), 0, 0x51A7) * 2.99) as i32 - 1;
        let into = |px: i32, py: i32| {
            let r = if fill_right { px - py + waver(x0 + px + y0 + py) } else { i32::MIN };
            let l = if fill_left { (15 - px) - py + waver(y0 + py - x0 - px) } else { i32::MIN };
            r.max(l)
        };
        let beside = |px: i32, py: i32| if fill_right && (!fill_left || px - py >= (15 - px) - py) { 1 } else { -1 };
        let vx = x.rem_euclid(4);
        let soil = compose(|px, py| {
            let (ix, iy) = (px as i32, py as i32);
            let d = into(ix, iy);
            if d >= 0 {
                return 0;
            }
            let v = ground.get(px, py);
            if d == -1 {
                // A thin shadow along the foot.
                match palette {
                    PAL_SAND => sand::SHADOW[0],
                    _ if v == grass::SAND[0] || v == grass::SAND[1] || v == grass::SAND[2] => grass::SAND[0],
                    _ => grass::RAMP[0],
                }
            } else {
                v
            }
        });
        let rock = compose(|px, py| {
            let (ix, iy) = (px as i32, py as i32);
            let d = into(ix, iy);
            if d < 0 {
                return 0;
            }
            let (row, rows) = face_rows(map, x + beside(ix, iy), y);
            // The rock runs down to the ground with no rubble, darkened
            // where it meets it.
            let v = cliff_pixel(row, rows, false, false, false, vx * 16 + ix, iy);
            if d < 1 { SANDSTONE[1] } else { v }
        });
        Some((self.unique_art(palette, &soil), self.unique_art(PAL_ROCK, &rock)))
    }

    /// River water (BG2, under BG1's transparent water pixels): dark under
    /// the banks, deeper in the middle, with streaks of current whose
    /// colours cycle so they race downstream.
    fn river_water(&mut self, map: &WorldMap, r: &RiverField, x: i32, y: i32) -> MetatileId {
        if map.terrain(x, y) == Terrain::Cliff {
            return self.falls(r, x, y);
        }
        // How far below the foot of the falls each row is (px), if close.
        let below_falls = |py: usize| (1..=2).find(|&k| map.terrain(x, y - k) == Terrain::Cliff).map(|k| (k - 1) * 16 + py as i32);
        // Out past the beach the current is sea water: as dark as the deep
        // sea, so it reads as too deep to wade.
        let f = CellField::sample(map, x, y);
        let sea = (f.min < SEA_CURRENT_FROM && !map.is_high(x, y)).then(|| slice(&self.tex.deep, x, y));
        let art = compose(|px, py| {
            let i = py * 16 + px;
            let d = r.depth[i];
            let (wx, wy) = (x * 16 + px as i32, y * 16 + py as i32);
            let b = bayer(wx, wy);
            if d <= 0.0 {
                return 0;
            }
            // Out of the lake the stream starts in the lake's own blue,
            // turning to the river's colours over its mouth.
            if lake_blend(map, x, y, r.along[i], b) {
                return deep::SHELF[0];
            }
            if let Some(tex) = &sea {
                // Fades in from the river over a stretch of the shallows.
                let into = (SEA_CURRENT_FROM - f.at(px, py)) / (SEA_CURRENT_FROM - RIVER_CHURN_BELOW);
                if b < into {
                    return sea_current_px(r, i, tex.get(px, py), f.at(px, py), wx, wy);
                }
            }
            // White water where the falls crash into the pool.
            if let Some(fall) = below_falls(py).filter(|_| r.channel[i] > -6.0) {
                let reach = 10.0 + noise_px(wx / 2, wy, 0xFA11) * 8.0 - (r.channel[i].min(0.0)).abs();
                if (fall as f32) < reach {
                    return if b < 0.6 { deep::CREST } else { deep::RIVER_FLOW[2] };
                } else if (fall as f32) < reach + 4.0 && b < 0.3 {
                    return deep::CREST;
                }
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

    /// The waterfall (BG2, behind the cliff face's open middle): streaks
    /// of falling water whose colours cycle down the face, bright crests
    /// here and there, dark at the edges.
    fn falls(&mut self, r: &RiverField, x: i32, y: i32) -> MetatileId {
        let art = compose(|px, py| {
            let i = py * 16 + px;
            let d = r.channel[i];
            if d <= 0.0 {
                return 0;
            }
            let (wx, wy) = (x * 16 + px as i32, y * 16 + py as i32);
            if d < 1.5 {
                return deep::RIVER_SHADE;
            }
            let lane = (r.across[i] / 1.5).floor() as i32;
            let kind = hash_unit(lane, 0, 0xFA1D);
            if kind < 0.12 && hash_unit(lane, wy.div_euclid(12), 0xFA1E) < 0.7 {
                return deep::CREST;
            }
            if kind < 0.75 {
                // Each lane starts its run of colours at its own place.
                let at = r.along[i] / 2.0 + hash_unit(lane, 1, 0xFA1F) * 3.0;
                deep::RIVER_FLOW[at.floor().rem_euclid(3.0) as usize]
            } else if bayer(wx, wy) < 0.5 {
                deep::RIVER_BODY
            } else {
                deep::RIVER_FLOW[0]
            }
        });
        self.unique_art(PAL_DEEP, &art)
    }

    /// The sea where the river's current runs out from the mouth: the
    /// water around it as anywhere else, cut away where the current runs
    /// so its deep, dark water (BG2) shows through.
    fn current(&mut self, map: &WorldMap, f: &CellField, r: &RiverField, x: i32, y: i32) -> MetatileId {
        let deep = map.terrain(x, y) == Terrain::DeepWater || (f.min < DEEP_BELOW - 0.4 && f.max < DEEP_BELOW + 1.7);
        let tex = if deep { slice(&self.tex.deep, x, y) } else { slice(&self.tex.shallow, x, y) };
        let art = compose(|px, py| {
            let h = f.at(px, py);
            if r.depth[py * 16 + px] > 0.0 {
                0
            } else if deep {
                reef_px(tex.get(px, py), h, px as i32, py as i32)
            } else {
                shallow_depth(tex.get(px, py), h, x * 16 + px as i32, y * 16 + py as i32)
            }
        });
        self.unique_art(if deep { PAL_DEEP } else { PAL_SHORE }, &art)
    }

    /// A cliff's rock face: blocks of stone in courses, lit from the
    /// top-left, in shadow under the edge above, with rubble (or surf) at
    /// the foot. Where the falls come down the face is left open for the
    /// water behind it on BG2.
    fn cliff_face(&mut self, map: &WorldMap, x: i32, y: i32, river: Option<&RiverField>) -> MetatileId {
        self.cliff_face_on(map, x, y, river, false)
    }

    /// The face's foot meets the grass below it: the grass grows up over the
    /// foot (BG1 left clear there, the grass drawn on BG2 under it).
    fn grassy_foot(map: &WorldMap, x: i32, y: i32) -> bool {
        map.terrain(x, y + 1) != Terrain::Cliff
            && matches!(map.terrain(x, y + 1), Terrain::Grass | Terrain::Highland)
            && map.foot_fill(x, y + 1) == [false, false]
            && dip(map, x, y).is_none()
    }

    /// A cliff face; with `under` (BG2 free beneath it), a foot standing on
    /// grass lets the grass grow up over it.
    fn cliff_face_on(&mut self, map: &WorldMap, x: i32, y: i32, river: Option<&RiverField>, under: bool) -> MetatileId {
        let (row, rows) = face_rows_drawn(map, x, y);
        // A side opens onto lower ground, unless the rock is carried on
        // into the corner there (see `WorldMap::foot_fill`).
        // A crag's face stands on the ground, so it opens onto the ground
        // beside it too.
        let under_crag = (1..=CLIFF_ROWS).any(|k| map.terrain(x, y - k) == Terrain::Crag);
        let open = |dx: i32| {
            let beside = map.terrain(x + dx, y);
            if under_crag {
                return !matches!(beside, Terrain::Cliff | Terrain::Crag);
            }
            !matches!(beside, Terrain::Cliff | Terrain::CliffTop | Terrain::Highland) && !map.foot_fill(x + dx, y)[if dx < 0 { 1 } else { 0 }]
        };
        let wet = map.terrain(x, y + 1).holds_water() && row == rows;
        let (left, right) = (open(-1), open(1));
        let grassy = under && river.is_none() && row == rows && Self::grassy_foot(map, x, y);

        let vx = x.rem_euclid(4);
        let draw = move |px: usize, py: usize| sandstone_pixel(row, rows, left, right, wet, grassy, vx * 16 + px as i32, py as i32);
        if let Some((cut, oy)) = dip(map, x, y) {
            let art = compose(|px, py| crumbled_pixel(cut, x * 16 + px as i32, py as i32, oy, draw(px, py)));
            return self.unique_art(PAL_ROCK, &art);
        }
        match river.filter(|r| r.channel.iter().any(|&d| d > 0.0)) {
            Some(r) => {
                let art = compose(|px, py| if r.channel[py * 16 + px] > 0.0 { 0 } else { draw(px, py) });
                self.unique_art(PAL_ROCK, &art)
            }
            None => self.get(Key::Cliff { row, rows, left, right, wet, grassy, vx }, |_, bank| bank.add_art(PAL_ROCK, &compose(draw))),
        }
    }

    /// The edge of the high ground: grass running to a dark, overhanging
    /// lip where it drops to a cliff face, and to a bare earthen bank on the
    /// other sides that drop away.
    fn cliff_top(&mut self, map: &WorldMap, x: i32, y: i32) -> MetatileId {
        // The slope climbs up to the edge: it doesn't drop away there.
        let drop = |dx: i32, dy: i32| !matches!(map.terrain(x + dx, y + dy), Terrain::Highland | Terrain::CliffTop | Terrain::River | Terrain::Slope);
        let mut open = [drop(0, -1), drop(1, 0), drop(0, 1), drop(-1, 0)];
        let (vx, vy) = (x.rem_euclid(4), y.rem_euclid(4));
        if let Some((cut, 0)) = dip(map, x, y) {
            return self.unique_art(PAL_ROCK, &lowered_edge_art(cut, x));
        }
        let above_dip = dip(map, x, y + 1).filter(|&(_, oy)| oy == 0).map(|(cut, _)| cut);
        if above_dip.is_some() {
            // The edge drops to the dip below as it would to a face.
            open[2] = true;
        }
        if let Some((left, right)) = cliff_side(map, x, y) {
            return self.get(Key::CliffSide { left, right, vx, vy }, |_, bank| {
                bank.add_art(PAL_ROCK, &compose(|px, py| cliff_side_pixel(left, right, vx * 16 + px as i32, vy * 16 + py as i32)))
            });
        }
        if let Some(cut) = above_dip {
            let mut art = cliff_top_art(self.tex, open, vx, vy);
            crumble_edge(&mut art, &slice(&self.tex.grass, vx, vy), cut, x * 16);
            // A ladder stood in the dip sticks up over its edge to here.
            if map.is_ladder(x, y + 1) {
                for py in LADDER_TOP..16 {
                    for (px, v) in ladder_pixel((py - LADDER_TOP) as i32, LADDER_PALETTE_GRASS) {
                        art.set(px, py, v);
                    }
                }
            }
            return self.unique_art(PAL_GRASS, &art);
        }
        self.get(Key::CliffTop { open, vx, vy }, |tex, bank| bank.add_art(PAL_GRASS, &cliff_top_art(tex, open, vx, vy)))
    }

    /// A ladder stood against a cliff: on the rock face (rock palette, its
    /// wood colours) down to the foot, and its tops sticking up over the
    /// cliff's edge (grass palette, in its earth and sand browns).
    fn ladder(&mut self, map: &WorldMap, x: i32, y: i32, part: u8) -> MetatileId {
        let (vx, vy) = (x.rem_euclid(4), y.rem_euclid(4));
        let dipped = dip(map, x, y);
        // Stood in a dip, the ladder's tops stick up from the cell above
        // its top (see `cliff_top`).
        let lowered = dip(map, x, y - part as i32).is_some_and(|(_, oy)| oy == 0);
        if part == 0 && dipped.is_none() {
            let drop = |dx: i32, dy: i32| !matches!(map.terrain(x + dx, y + dy), Terrain::Highland | Terrain::CliffTop | Terrain::River);
            let open = [drop(0, -1), drop(1, 0), drop(0, 1), drop(-1, 0)];
            let mut art = cliff_top_art(self.tex, open, vx, vy);
            for py in LADDER_TOP..16 {
                for (px, v) in ladder_pixel((py - LADDER_TOP) as i32, LADDER_PALETTE_GRASS) {
                    art.set(px, py, v);
                }
            }
            return self.unique_art(PAL_GRASS, &art);
        }
        // The top of a crumbled cliff is the lowered edge's cell: rock
        // around the cut, like the top of the full-height face beside it.
        let (row, rows) = if part == 0 { (1, CLIFF_ROWS) } else { face_rows_drawn(map, x, y) };
        let wet = part > 0 && map.terrain(x, y + 1).holds_water() && row == rows;
        let art = compose(|px, py| {
            let wx = vx * 16 + px as i32;
            let rock = cliff_pixel(row, rows, false, false, wet, wx, py as i32);
            let rock = dipped.map_or(rock, |(cut, oy)| crumbled_pixel(cut, x * 16 + px as i32, py as i32, oy, rock));
            // The poles and rungs (with a shadow on the rock to their
            // right and below), from the tops down.
            let at = (part as i32 + lowered as i32) * 16 + py as i32 - LADDER_TOP as i32;
            if at < 0 {
                return rock;
            }
            ladder_pixel(at, LADDER_PALETTE_ROCK).find(|&(lx, _)| lx == px).map_or_else(
                || {
                    let shade =
                        ladder_pixel(at, LADDER_PALETTE_ROCK).any(|(lx, _)| lx + 1 == px) || ladder_pixel(at - 1, LADDER_PALETTE_ROCK).any(|(lx, _)| lx == px);
                    if shade && rock != 0 { SANDSTONE[0] } else { rock }
                },
                |(_, v)| v,
            )
        });
        self.unique_art(PAL_ROCK, &art)
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
        self.ground(map, x, y, false).0
    }

    /// Ground tile for a cell, and (where `under` allows it) one for BG2
    /// beneath it: the lake's shore is wet soil in the rock palette's
    /// browns, which the water's and the grass's palettes don't have, so
    /// where it meets either the soil is drawn on BG2 and shows through
    /// BG1's transparent pixels. Without BG2 the soil there is drawn in
    /// the nearest colour BG1's palette has.
    fn ground(&mut self, map: &WorldMap, x: i32, y: i32, under: bool) -> (MetatileId, Option<MetatileId>) {
        let t = map.terrain(x, y);
        if t == Terrain::Crag {
            return self.crag(map, x, y, under);
        }
        if t == Terrain::CliffTop
            && under
            && let Some((left, right)) = cliff_side(map, x, y)
        {
            // Seen from above, the side drops away in a narrow band of rock;
            // the high ground's grass runs on (on BG2) to its lip.
            let (vx, vy) = (x.rem_euclid(4), y.rem_euclid(4));
            let band = self.get(Key::CliffSideBand { left, right, vx, vy }, |_, bank| {
                bank.add_art(PAL_ROCK, &compose(|px, py| cliff_side_band_pixel(left, right, vx * 16 + px as i32, vy * 16 + py as i32)))
            });
            return (band, Some(self.interior(Terrain::Highland, x, y)));
        }
        if t == Terrain::Cliff {
            let face = self.cliff_face_on(map, x, y, None, under);
            let (row, rows) = face_rows_drawn(map, x, y);
            let grass = (under && row == rows && Self::grassy_foot(map, x, y)).then(|| self.interior(Terrain::Grass, x, y));
            return (face, grass);
        }
        if map.is_high(x, y) && !matches!(t, Terrain::Cliff | Terrain::CliffTop) && near_mud(map, x, y) {
            let t = if t == Terrain::Highland { Terrain::Grass } else { t };
            let f = CellField::sample(map, x, y);
            match t {
                Terrain::Sand | Terrain::ShallowWater if f.crosses(0.0, 0.75, 0.45) => {
                    let water = slice(&self.tex.shallow, x, y);
                    let art = compose(|px, py| lake_edge_pixel(&f, &water, under, px, py));
                    let soil = under.then(|| self.mud_under(&f, x, y));
                    return (self.unique_art(PAL_SHORE, &art), soil);
                }
                Terrain::Sand | Terrain::Grass if f.crosses(SAND_BELOW, 0.35, 0.3) => {
                    let (g, _) = self.grass_and_sand(x, y);
                    let art = compose(|px, py| {
                        let v = grass_or_sand(&f, &g, &PixelArt::filled(16, 16, 0), x, y, px, py);
                        if v == 0 && !under { grass::SOIL } else { v }
                    });
                    let soil = under.then(|| self.mud_under(&f, x, y));
                    return (self.unique_art(PAL_GRASS, &art), soil);
                }
                Terrain::Sand => {
                    let (vx, vy) = (x.rem_euclid(4), y.rem_euclid(4));
                    return (self.get(Key::Mud(vx, vy), |tex, bank| bank.add_art(PAL_ROCK, &slice(&tex.mud, vx, vy))), None);
                }
                _ => {}
            }
        }
        (self.plain_ground(map, x, y), None)
    }

    /// Wet soil for BG2 under a lake-shore cell: darker and wetter toward
    /// the water.
    fn mud_under(&mut self, f: &CellField, x: i32, y: i32) -> MetatileId {
        let mud = slice(&self.tex.mud, x, y);
        let art = compose(|px, py| {
            let h = f.at(px, py);
            let (wx, wy) = (x * 16 + px as i32, y * 16 + py as i32);
            if h < 0.35 || (h < 0.9 && bayer(wx, wy) < 0.5) { rock::WOOD[0] } else { mud.get(px, py) }
        });
        self.unique_art(PAL_ROCK, &art)
    }

    /// A cell of a crag: its top, or, just below a higher tier, the face
    /// that tier rises in, its foot (with `under`) sunk in the scrub of the
    /// tier below (drawn on BG2).
    fn crag(&mut self, map: &WorldMap, x: i32, y: i32, under: bool) -> (MetatileId, Option<MetatileId>) {
        let Some(row) = tier_face_row(map, x, y) else {
            return (self.crag_top(map, x, y), None);
        };
        let open = |dx: i32| map.terrain(x + dx, y) != Terrain::Crag || tier_face_row(map, x + dx, y).is_none();
        let (left, right) = (open(-1), open(1));
        let grassy = under && row == TIER_FACE;
        let vx = x.rem_euclid(4);
        let key = Key::Cliff { row, rows: TIER_FACE, left, right, wet: false, grassy, vx };
        let face = self.get(key, |_, bank| {
            bank.add_art(PAL_ROCK, &compose(|px, py| sandstone_pixel(row, TIER_FACE, left, right, false, grassy, vx * 16 + px as i32, py as i32)))
        });
        (face, grassy.then(|| self.crag_top(map, x, y)))
    }

    /// The top of a crag (foliage palette): a mound grown over with trees
    /// and scrub, seen from above as heaped clumps of leaves.
    fn crag_top(&mut self, map: &WorldMap, x: i32, y: i32) -> MetatileId {
        let drop = |dx: i32, dy: i32| map.terrain(x + dx, y + dy) != Terrain::Crag || map.rise(x + dx, y + dy) < map.rise(x, y);
        let open = [drop(0, -1), drop(1, 0), drop(0, 1), drop(-1, 0)];
        let tier = map.rise(x, y);
        let higher = |dx: i32, dy: i32| map.terrain(x + dx, y + dy) == Terrain::Crag && map.rise(x + dx, y + dy) > tier;
        let shade = [higher(0, -1) && tier_face_row(map, x, y).is_none(), higher(-1, 0)];
        // This tier's own slopes down to a lower one, west (lit) and east
        // (in shade).
        let lower = |dx: i32| map.terrain(x + dx, y) == Terrain::Crag && map.rise(x + dx, y) < tier;
        let slope = [lower(-1), lower(1)];
        let (vx, vy) = (x.rem_euclid(4), y.rem_euclid(4));
        self.get(Key::Crag { open, tier, shade, slope, vx, vy }, |_, bank| {
            bank.add_art(PAL_FOLIAGE, &compose(|px, py| crag_pixel(open, tier, shade, slope, vx * 16 + px as i32, vy * 16 + py as i32)))
        })
    }

    /// A cell of the slope (grass palette): a grassy hillside rising to the
    /// north-east, lighter as it rises (it faces the light), with bands of
    /// shade along its contours, so it reads as ground going up rather than
    /// a drop.
    fn slope(&mut self, map: &WorldMap, x: i32, y: i32) -> MetatileId {
        let g = slice(&self.tex.grass, x, y);
        let (low, high) = map.spots.slope_span.unwrap_or((x * 16 - y * 16 - 16, x * 16 - y * 16 + 16));
        let r = grass::RAMP;
        // How many cells of slope run on from this one each way (up to
        // two): its shading fades out toward the slope's edges, so it melts
        // into the grass around it.
        let run = |dx: i32, dy: i32| (1..=2).take_while(|&k| map.terrain(x + dx * k, y + dy * k) == Terrain::Slope).count() as i32;
        let (up, down) = (run(0, -1), run(0, 1));
        let art = compose(|px, py| {
            let (ix, iy) = (px as i32, py as i32);
            let (wx, wy) = (x * 16 + ix, y * 16 + iy);
            let b = bayer(wx, wy);
            // How high up the hillside (0 at its foot, 1 at its top),
            // measured across the whole of it on the slant, so its shading
            // runs in smooth lines rather than steps.
            let t = ((wx - wy - low) as f32 / (high - low).max(1) as f32).clamp(0.0, 1.0);
            // Distance (px) to the slope's edge, and how much of its shading
            // shows there.
            // Its ends (north and south) by the cells; its foot and its top
            // on the slant, where it meets the ground below and the high
            // ground.
            let edge = (up * 16 + iy).min(down * 16 + 15 - iy);
            let fade = (edge as f32 / 28.0).min((1.0 - t) / 0.12).min(1.0);
            if b >= fade {
                return g.get(px, py);
            }
            let t = (t + noise_px(wx / 3, wy / 3, 0x51E4) * 0.2 - 0.1).clamp(0.0, 1.0);
            // Contours run along the hillside (down to the south-east).
            let contour = ((wx - wy) as f32 / 10.0 + noise_px(wx / 4, wy / 4, 0x51E5) * 0.6).rem_euclid(1.0);
            let base = g.get(px, py);
            // The rises show more as the hillside steepens toward the top;
            // its foot fades into the ground below.
            if contour < 0.1 && t > 0.25 {
                if b < 0.3 + t * 0.4 { r[1] } else { r[2] }
            } else if contour < 0.2 && t > 0.35 && b < 0.5 {
                r[2]
            } else if t < 0.3 {
                if b < t * 1.2 { r[2] } else { base }
            } else if t < 0.5 {
                if b < 0.4 { r[2] } else { base }
            } else if t > 0.8 {
                // Up top, in the sun.
                if b < 0.5 { r[5] } else { base }
            } else if t > 0.65 && b < 0.25 {
                r[5]
            } else {
                base
            }
        });
        self.unique_art(PAL_GRASS, &art)
    }

    fn plain_ground(&mut self, map: &WorldMap, x: i32, y: i32) -> MetatileId {
        let t = map.terrain(x, y);
        if t == Terrain::Slope {
            return self.slope(map, x, y);
        }
        if t == Terrain::Crag {
            return self.crag_top(map, x, y);
        }
        match t {
            Terrain::Cliff => return self.cliff_face(map, x, y, None),
            Terrain::CliffTop => return self.cliff_top(map, x, y),
            _ => {}
        }
        // The high ground is grass, meeting the lake's shore as the grass
        // below meets the beach. Its height runs low by the sea, where it
        // ends at the cliffs' edges: without sand beside it, it has no beach
        // to meet.
        if t == Terrain::Highland && !(-1..=1).any(|dy| (-1..=1).any(|dx| map.terrain(x + dx, y + dy) == Terrain::Sand)) {
            return self.interior(Terrain::Grass, x, y);
        }
        let t = if t == Terrain::Highland { Terrain::Grass } else { t };
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

/// How many cells of rock face each tier of a crag rises in.
const TIER_FACE: i32 = 2;

/// Which row (from 1 at the top) of a higher tier's face a crag cell is,
/// if one rises just north of it.
/// Only where the higher tier's edge runs on along (it and the cells
/// either side of it are higher) does a face rise: where the edge turns or
/// slants, the tier shows only by its shaded rim.
fn tier_face_row(map: &WorldMap, x: i32, y: i32) -> Option<i32> {
    let tier = map.rise(x, y);
    let higher = |x: i32, y: i32| map.terrain(x, y) == Terrain::Crag && map.rise(x, y) > tier;
    (1..=TIER_FACE).find(|&k| higher(x, y - k)).filter(|&k| (-1..=1).all(|dx| higher(x + dx, y - k) && (1..k).all(|j| !higher(x + dx, y - j))))
}

/// A pixel of a crag's top (foliage palette) at world px within the 64 px
/// texture: round clumps of leaves heaped over one another, lit from the
/// top-left, dark in the gaps between them and along the sides that drop
/// away.
fn crag_pixel(open: [bool; 4], tier: u8, shade: [bool; 2], slope: [bool; 2], wx: i32, wy: i32) -> u8 {
    let l = foliage::LEAF;
    let (x, y) = (wx.rem_euclid(64), wy.rem_euclid(64));
    let (px, py) = (x.rem_euclid(16), y.rem_euclid(16));
    const G: i32 = 16;
    // The clump nearest the viewer (the lowest centre) that covers the
    // pixel, on a jittered grid that repeats every 64 px.
    let mut best: Option<(f32, f32, f32, f32)> = None;
    for oy in -1..=1 {
        for ox in -1..=1 {
            let (gx, gy) = (x.div_euclid(G) + ox, y.div_euclid(G) + oy);
            let (hx, hy) = (gx.rem_euclid(64 / G), gy.rem_euclid(64 / G));
            let cx = (gx * G) as f32 + 3.0 + hash_unit(hx, hy, 0xC1A1) * 10.0;
            let cy = (gy * G) as f32 + 3.0 + hash_unit(hx, hy, 0xC1A2) * 10.0;
            let r = 8.0 + hash_unit(hx, hy, 0xC1A3) * 4.0;
            let (dx, dy) = (x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);
            if dx * dx + dy * dy < r * r && best.is_none_or(|b| cy > b.1) {
                best = Some((cx, cy, dx / r, dy / r));
            }
        }
    }
    let b = bayer(wx, wy);
    let mut v = match best {
        None => l[0],
        Some((_, _, dx, dy)) => {
            // Higher tiers stand nearer the sun.
            let light = -(dx + dy) + tier as f32 * 0.35 - 0.3;
            let rim = (dx * dx + dy * dy).sqrt();
            // Leaves breaking up each clump's shading.
            let leaf = noise_px(wx.div_euclid(2), wy.div_euclid(2), 0xC1A4) - 0.5;
            let light = light + leaf * 0.7;
            if rim > 0.88 && light < 0.2 {
                l[1]
            } else if light > 0.9 {
                if b < 0.5 { l[5] } else { l[4] }
            } else if light > 0.3 {
                l[4]
            } else if light > -0.3 {
                if b < 0.5 { l[3] } else { l[2] }
            } else if light > -0.8 {
                l[2]
            } else {
                l[1]
            }
        }
    };
    let rag = |k: i32| (noise_px(k, 0, 0xC7A8) * 2.5) as i32;
    // Slopes down to a lower tier: lit facing west, shaded facing east.
    if slope[0] && px < 4 + rag(wy + 17) {
        v = if px < 2 || b < 0.5 { l[5] } else { l[4] };
    } else if slope[1] && px > 11 - rag(wy + 19) {
        v = if px > 13 || b < 0.5 { l[0] } else { l[1] };
    }
    // The shadow a higher tier casts down and to the right over this one.
    if (shade[1] && px < 4 + rag(wy + 11)) || (shade[0] && py < 4 + rag(wx + 13)) {
        v = match v {
            x if x == l[5] || x == l[4] => l[2],
            x if x == l[3] || x == l[2] => l[1],
            _ => l[0],
        };
    }
    if (open[3] && px < 1 + rag(wy)) || (open[0] && py < 1 + rag(wx + 7)) || (open[1] && px > 14 - rag(wy + 3)) {
        v = if b < 0.5 { l[0] } else { l[1] };
    }
    v
}

/// A pixel of a crag's face (rock palette): sandstone in soft layers,
/// streaked down by the rain, with dark hollows near the foot, scrub
/// hanging over the top and rubble (or the lake's lapping) at the foot.
#[allow(clippy::too_many_arguments)]
fn sandstone_pixel(row: i32, rows: i32, left: bool, right: bool, wet: bool, grassy: bool, wx: i32, py: i32) -> u8 {
    let s = rock::STONE;
    // Everything repeats every 64 px along, so faces can share blocks.
    let wx = wx.rem_euclid(64);
    let sand = rock::SAND;
    let fy = (row - 1) * 16 + py;
    let depth = rows * 16;
    let b = bayer(wx, fy);
    // Layers, wavering a little along the face.
    let band = (fy as f32 + noise_px(wx.div_euclid(8).rem_euclid(8), 0, 0x5A01) * 5.0) / 6.0;
    let layer = band.floor() as i32;
    let tone = hash_unit(layer, 0, 0x5A02);
    let mut v = if band.fract() < 0.15 {
        rock::SAND_SHADOW
    } else if tone < 0.35 {
        sand[0]
    } else if tone < 0.8 {
        if b < 0.3 { sand[0] } else { sand[1] }
    } else {
        if b < 0.5 { sand[1] } else { sand[2] }
    };
    // Rain streaks running down from the top, each its own length.
    let col = wx.div_euclid(2).rem_euclid(32);
    let streak = noise_px(col, 0, 0x5A03);
    let reach = (noise_px(col, 1, 0x5A04) * depth as f32) as i32;
    if streak < 0.22 && fy < reach {
        v = if streak < 0.08 { rock::WOOD[1] } else { rock::WOOD[2] };
    }
    // Hollows weathered out low on the face: soft-edged, darkest inside.
    let a = wx as f32 / 64.0 * std::f32::consts::TAU;
    let hollow = value_noise(a.cos() * 1.2 + fy as f32 / 5.0, a.sin() * 1.2 + fy as f32 / 9.0, 0x5A05) - (depth - fy) as f32 / depth as f32 * 0.5;
    if hollow > 0.62 {
        v = rock::WOOD[0];
    } else if hollow > 0.55 {
        v = if b < 0.5 { rock::WOOD[0] } else { rock::SAND_SHADOW };
    }
    // Scrub spilling over the top in rounded clumps, shading the rock
    // under it.
    let clump = wx.div_euclid(8);
    let cx = clump * 8 + 4;
    let reach = 3.0 + hash_unit(clump.rem_euclid(8), 0, 0x5A06) * 9.0;
    let dx = (wx - cx) as f32 / 4.0;
    let hang = (reach * (1.0 - dx * dx).max(0.0).sqrt()) as i32 + 2;
    if fy < hang - 1 {
        v = if fy >= hang - 3 && b < 0.5 { rock::WOOD[0] } else { rock::MOSS };
    } else if fy < hang + 2 {
        v = rock::WOOD[0];
    } else if fy < hang + 4 && b < 0.5 {
        v = rock::SAND_SHADOW;
    }
    // Grass growing up over the foot (left clear for the grass on BG2),
    // the rock in its shade just above.
    if grassy && row == rows {
        let blade = noise_px(wx.div_euclid(2).rem_euclid(32), 5, 0x5A10);
        let tall = 2 + (blade * 5.0) as i32;
        if py >= 16 - tall {
            return 0;
        }
        if py >= 14 - tall {
            return if py == 15 - tall || bayer(wx, py) < 0.5 { rock::WOOD[0] } else { rock::SAND_SHADOW };
        }
    }
    // Along the foot: the lake (or the sea) lapping where it stands in
    // water; on dry ground no rubble, only the rock darkening into it.
    if row == rows && wet && py >= 12 {
        let n = noise_px(wx, py, 0xF008);
        if n < 0.5 {
            v = s[5];
        } else if n < 0.75 {
            v = s[4];
        }
    } else if row == rows && !grassy && py >= 14 {
        v = if py == 15 || bayer(wx, py) < 0.5 { rock::SAND_SHADOW } else { v };
    }
    let px = wx.rem_euclid(16);
    if (left && px == 0) || (right && px == 15) {
        v = rock::WOOD[0];
    } else if (left && px == 1) || (right && px == 14) {
        v = rock::SAND_SHADOW;
    }
    v
}

/// How far (px) down the stream from the lake its water turns from the
/// lake's blue to the river's colours.
const LAKE_BLEND_PX: f32 = 56.0;

/// Whether a pixel of the stream this far (`along`, px) out of the lake is
/// still drawn in the lake's blue (dithered by `b` as it turns).
fn lake_blend_at(along: f32, b: f32) -> bool {
    along < LAKE_BLEND_PX && b >= along / LAKE_BLEND_PX
}

/// `lake_blend_at`, only up on the high ground (the stream out of the lake).
fn lake_blend(map: &WorldMap, x: i32, y: i32, along: f32, b: f32) -> bool {
    map.is_high(x, y) && lake_blend_at(along, b)
}

/// Whether a cell is on or by the lake's wet soil.
fn near_mud(map: &WorldMap, x: i32, y: i32) -> bool {
    (-1..=1).any(|dy| (-1..=1).any(|dx| map.soil(x + dx, y + dy) == Soil::Mud))
}

/// A pixel of the lake's water meeting its shore (shore palette): calm,
/// with no surf, only a pale line where it laps the soil. The soil is left
/// clear for BG2 (or, without it, drawn wet).
fn lake_edge_pixel(f: &CellField, water: &PixelArt, under: bool, px: usize, py: usize) -> u8 {
    let h = f.at(px, py);
    let d = -h * PX_PER_UNIT;
    let (ix, iy) = (px as i32, py as i32);
    if h >= 0.0 {
        if under { 0 } else { shore::WET_DARK }
    } else if d < 1.2 {
        shore::WATER[3]
    } else if d < 2.4 {
        if bayer(ix, iy) < 0.5 { shore::WATER[3] } else { shore::WATER[2] }
    } else {
        shallow_depth(water.get(px, py), h, ix, iy)
    }
}

/// The edge of the high ground (grass palette): see `Builder::cliff_top`.
fn cliff_top_art(tex: &TerrainTextures, open: [bool; 4], vx: i32, vy: i32) -> PixelArt {
    let g = slice(&tex.grass, vx, vy);
    let r = grass::RAMP;
    compose(|px, py| {
        let (wx, wy) = (vx * 16 + px as i32, vy * 16 + py as i32);
        let b = bayer(wx, wy);
        let (fx, fy) = (px as f32, py as f32);
        // Ragged depth of each side's edge.
        let rag = |k: i32| noise_px(k, 0, 0xED6E) * 2.5;
        let south = 15.0 - fy - rag(wx);
        let sides = [
            if open[0] { fy - rag(wx + 99) } else { f32::MAX },
            if open[1] { 15.0 - fx - rag(wy + 7) } else { f32::MAX },
            if open[3] { fx - rag(wy + 51) } else { f32::MAX },
        ];
        let side = sides.iter().copied().fold(f32::MAX, f32::min);
        if open[2] && south < 1.0 {
            r[0]
        } else if open[2] && south < 2.0 {
            if b < 0.5 { r[0] } else { grass::SOIL }
        } else if open[2] && south < 3.0 {
            if b < 0.5 { r[1] } else { r[2] }
        } else if side < 1.0 {
            grass::SOIL
        } else if side < 2.0 {
            if b < 0.5 { grass::SOIL } else { r[0] }
        } else if side < 3.0 {
            if b < 0.5 { r[1] } else { g.get(px, py) }
        } else {
            g.get(px, py)
        }
    })
}

/// How deep (px) the grass sinks where the top of a low cliff has
/// crumbled, in the middle of the cut, from the top of the lowered edge's
/// cell (the edge is a row lower there, drawn as the top of the rock
/// beside it, so the cut looks only this deep).
const CRUMBLE_DEPTH: f32 = 13.0;

/// Where the top of a low cliff has crumbled, as the world px its flat
/// bottom runs from and to; the cut slants up from there over
/// `CRUMBLE_SLANT` px either side.
type Cut = (i32, i32);

/// How wide (px) each slanting end of a crumbled cut is.
const CRUMBLE_SLANT: f32 = 16.0;

/// Whether the cell is the lowered edge of a low cliff face: the face
/// below it is short enough for a ladder (the recess in the north cliff),
/// and its edge sits a row below the full-height rock's beside it.
fn lowered(map: &WorldMap, x: i32, y: i32) -> bool {
    map.terrain(x, y) == Terrain::CliffTop && map.terrain(x, y + 1) == Terrain::Cliff && face_rows(map, x, y + 1).1 <= LADDER_ROWS
}

/// The cut a cell in the row of a lowered edge lies in, if any: across the
/// lowered cells, its flat bottom inset half a cell from their ends so its
/// slants reach half a cell into the full-height rock either side.
fn cut_at(map: &WorldMap, x: i32, y: i32) -> Option<Cut> {
    let at = [x, x - 1, x + 1].into_iter().find(|&c| lowered(map, c, y))?;
    let (mut l, mut r) = (at, at);
    while lowered(map, l - 1, y) {
        l -= 1;
    }
    while lowered(map, r + 1, y) {
        r += 1;
    }
    Some((l * 16 + 8, (r + 1) * 16 - 8))
}

/// The cells of a crumbled cliff top: those in the lowered edge's row (0),
/// or in the row below it (16), with the cut and the cell's offset (px)
/// from the top of the cut.
fn dip(map: &WorldMap, x: i32, y: i32) -> Option<(Cut, i32)> {
    if let Some(cut) = cut_at(map, x, y) {
        return Some((cut, 0));
    }
    if map.terrain(x, y) == Terrain::Cliff {
        return cut_at(map, x, y - 1).map(|cut| (cut, 16));
    }
    None
}

/// Rows of a full-height cliff face.
const CLIFF_ROWS: i32 = 3;

/// The lowered edge's cell of a crumbled cliff top (rock palette): the top
/// of the full-height face beside it, cut away where the grass has sunk.
fn lowered_edge_art(cut: Cut, x: i32) -> PixelArt {
    let vx = x.rem_euclid(4);
    compose(|px, py| {
        let rock = cliff_pixel(1, CLIFF_ROWS, false, false, false, vx * 16 + px as i32, py as i32);
        crumbled_pixel(cut, x * 16 + px as i32, py as i32, 0, rock)
    })
}

/// The rows a cliff face is drawn as: a low face below a lowered edge
/// carries on the courses of the full-height rock beside it.
fn face_rows_drawn(map: &WorldMap, x: i32, y: i32) -> (i32, i32) {
    let (row, rows) = face_rows(map, x, y);
    if lowered(map, x, y - row) { (row + 1, rows + 1) } else { (row, rows) }
}

/// How much of the cut there is at world px `wx` (0 outside it, 1 across
/// its flat bottom).
fn crumble_amount((a, b): Cut, wx: i32) -> f32 {
    let x = wx as f32 + 0.5;
    ((x - (a as f32 - CRUMBLE_SLANT)).min(b as f32 + CRUMBLE_SLANT - x) / CRUMBLE_SLANT).clamp(0.0, 1.0)
}

/// How far down from the top of the lowered edge's cell the cut reaches,
/// at world px `wx`.
fn crumble_depth(cut: Cut, wx: i32) -> f32 {
    crumble_depth_beside(cut, wx, 0)
}

/// `crumble_depth` `dx` px along from `wx`.
fn crumble_depth_beside(cut: Cut, wx: i32, dx: i32) -> f32 {
    crumble_amount(cut, wx + dx) * (CRUMBLE_DEPTH + (noise_px((wx + dx).div_euclid(2), 0, 0xC2B1) - 0.5) * 3.0)
}

/// How wide (px) the side of the high ground shows along each slanting
/// end of the cut.
const CRUMBLE_WALL: i32 = 3;
/// How far (px) the shadow of the cut's left side falls across the grass.
const CRUMBLE_SHADOW: i32 = 12;

/// One pixel of the top cell of a crumbled cliff face (rock palette):
/// transparent where the grass has sunk into the cut (drawn on BG2, see
/// `crumbled_grass`), then roots hanging out from under the turf over the
/// rock (`rock`) below.
fn crumbled_pixel(cut: Cut, wx: i32, py: i32, oy: i32, rock: u8) -> u8 {
    let depth = crumble_depth(cut, wx);
    if depth <= 0.0 {
        return rock;
    }
    let py = py + oy;
    let y = py as f32 + 0.5;
    let b = bayer(wx, py);
    if y < depth {
        // Along a slanting end, the side of the higher ground beside the
        // cut shows as a strip of rock: in shadow on the left (facing away
        // from the light), lit on the right.
        let outside = |dx: i32| crumble_depth_beside(cut, wx, dx) <= y;
        // Right under the edge above, the strips are in its shadow like the
        // rock beside them, so they don't stick up past its line.
        let wall = (1..=CRUMBLE_WALL).any(|k| outside(-k) || outside(k));
        if wall && (y < 3.0 || (y < 5.0 && b < 0.5)) {
            return rock;
        }
        if (1..=CRUMBLE_WALL).any(|k| outside(-k)) {
            return SANDSTONE[0];
        }
        if let Some(k) = (1..=CRUMBLE_WALL).find(|&k| outside(k)) {
            return if k == 1 {
                SANDSTONE[4]
            } else if k == 2 || b < 0.5 {
                SANDSTONE[3]
            } else {
                SANDSTONE[2]
            };
        }
        return 0;
    }
    // Roots hanging out of the bank, some to a fair length.
    let a = crumble_amount(cut, wx);
    let col = wx.div_euclid(3);
    let length = (2.0 + hash_unit(col, 1, 0xC2B4) * 7.0) * a;
    if hash_unit(col, 0, 0xC2B3) < 0.35 && wx.rem_euclid(3) == 1 && y < depth + length {
        return if y > depth + length - 1.5 { rock::WOOD[1] } else { rock::WOOD[0] };
    }
    // Shadow cast down on the rock just under the turf (fainter along the
    // slanting ends).
    let shade = if crumble_amount(cut, wx) < 0.95 { 0.2 } else { 0.5 };
    if y < depth + 2.0 && b < shade { SANDSTONE[0] } else { rock }
}

/// The high ground's edge cell above a crumbled face (grass palette):
/// where the cut is, its dark overhanging lip gives way to grass running
/// on down into the cut (fading back in toward the cut's slanting ends).
fn crumble_edge(art: &mut PixelArt, g: &PixelArt, cut: Cut, x0: i32) {
    for px in 0..16 {
        let a = crumble_amount(cut, x0 + px as i32);
        for py in 0..16 {
            // The lip's dark rows reach up to about 6 px, ragged. Where the
            // grass hasn't replaced them, toward the cut's ends, they're
            // softened so the lip fades into the cut.
            let up = 16.0 - (py as f32 + 0.5);
            // At the cut's left end the left side's shadow falls here (its
            // edge slanting on up from the shadow on the grass below): the
            // lip stays dark instead of giving way to grass.
            let edge = 15.0 - 1.25 * up + bayer(x0 + px as i32, py as i32) * 3.0;
            let along = (x0 + px as i32 - cut.0) as f32 + CRUMBLE_SLANT;
            let in_shadow = (0.0..edge).contains(&along);
            if in_shadow && up < 6.0 {
                let v = art.get(px, py);
                if v == grass::RAMP[0] || v == grass::RAMP[1] || v == grass::SOIL {
                    art.set(px, py, if bayer(x0 + px as i32, py as i32) < 0.75 { grass::RAMP[0] } else { grass::RAMP[1] });
                }
            } else if up < 6.0 * a {
                art.set(px, py, g.get(px, py));
            } else if up < 6.0 && a > 0.15 {
                let v = art.get(px, py);
                let r = grass::RAMP;
                if v == r[0] || v == grass::SOIL {
                    art.set(px, py, r[1]);
                } else if v == r[1] {
                    art.set(px, py, r[2]);
                }
            }
        }
    }
}

/// The grass sunk into the cut at the top of a crumbled face (grass
/// palette, on BG2 under the face's transparent pixels): the high ground's
/// grass running on down, darkening as it slopes away toward its front,
/// where the turf's dark edge overhangs the rock.
fn crumbled_grass(tex: &TerrainTextures, cut: Cut, oy: i32, x: i32, y: i32) -> PixelArt {
    let g = slice(&tex.grass, x, y);
    let r = grass::RAMP;
    compose(|px, py| {
        let wx = x * 16 + px as i32;
        let fy = py as i32 + oy;
        let depth = crumble_depth(cut, wx);
        let up = depth - (fy as f32 + 0.5);
        let b = bayer(wx, fy);
        // The shadow the left side of the cut casts on the grass beside it.
        let y = fy as f32 + 0.5;
        let shadowed = |k: i32| crumble_depth_beside(cut, wx, -k) <= y;
        if up > 0.0 {
            // How far from the side (px): darkest against it, fading out.
            if let Some(k) = (1..=CRUMBLE_WALL + CRUMBLE_SHADOW).find(|&k| shadowed(k)) {
                let k = k - CRUMBLE_WALL;
                let v = if k <= 3 {
                    r[0]
                } else if k <= 6 {
                    if b < 0.6 { r[0] } else { r[1] }
                } else if k <= 9 {
                    if b < 0.5 { r[1] } else { r[2] }
                } else if b < 0.3 {
                    r[1]
                } else {
                    g.get(px, py)
                };
                return v;
            }
        }
        // Along the cut's slanting ends the edge is lighter, so it eases
        // into the lip either side instead of drawing a dark line to it.
        let slant = crumble_amount(cut, wx) < 0.95;
        if up <= 0.0 {
            0
        } else if up < 1.2 {
            if slant { r[1] } else { r[0] }
        } else if up < 2.5 {
            if slant {
                if b < 0.5 { r[1] } else { r[2] }
            } else if b < 0.5 {
                r[0]
            } else {
                r[1]
            }
        } else if up < 4.5 {
            if b < 0.5 { r[1] } else { r[2] }
        } else if up < 6.5 && b < 0.3 {
            r[2]
        } else {
            g.get(px, py)
        }
    })
}

/// Top and bottom rows of a laid log bridge in its cells.
const BRIDGE_ROWS: (i32, i32) = (1, 14);

/// The colour of a laid log bridge's pixel in its cell (grass palette), if
/// it's on the bridge: two logs side by side running east to west, lit
/// from above, cut ends showing on the banks, lashed with rope near them.
fn bridge_pixel(px: i32, py: i32, part: u8) -> Option<u8> {
    let (top, bottom) = BRIDGE_ROWS;
    let mid = (top + bottom) / 2;
    if !(top..=bottom).contains(&py) {
        return None;
    }
    // The logs reach halfway onto the bank cells.
    let (from, to) = match part {
        0 => (6, 15),
        2 => (0, 9),
        _ => (0, 15),
    };
    if !(from..=to).contains(&px) {
        return None;
    }
    // Row within its log, from the lit top.
    let row = if py <= mid { py - top } else { py - mid - 1 };
    let rows = mid - top + 1;
    let last = row == rows - 1;
    let end = (part == 0 && px == from) || (part == 2 && px == to);
    let grain = (part == 0 && px == from + 1) || (part == 2 && px == to - 1);
    let lashed = (part == 0 && (11..=12).contains(&px)) || (part == 2 && (3..=4).contains(&px));
    Some(if end || last {
        grass::RAMP[0]
    } else if grain {
        if row == 0 || row == rows - 2 {
            grass::SAND[0]
        } else if row == rows / 2 - 1 {
            grass::SAND[1]
        } else {
            grass::SAND[2]
        }
    } else if lashed {
        if (px + py) % 2 == 0 { grass::SAND[2] } else { grass::SOIL }
    } else if row == 0 {
        grass::SAND[1]
    } else if row >= rows - 2 || (px * 7 + py * 3 + part as i32 * 5) % 11 == 0 {
        // The shaded underside, and knots in the grain.
        grass::SOIL
    } else {
        grass::SAND[0]
    })
}

/// Frames of the water wheel turning (from one paddle to the next), and
/// video frames per frame.
const WHEEL_FRAMES: u32 = 4;
const WHEEL_PERIOD: u32 = 8;
/// The water wheel seen from above, in its two-by-two block of cells (px):
/// the rims' columns (west and east), how far it runs up and down the
/// river, and how many paddles go round it.
const WHEEL_RIMS: (i32, i32) = (6, 16);
const WHEEL_ENDS: (i32, i32) = (2, 29);
const WHEEL_PADDLES: u32 = 12;
/// The axle's rows, reaching from the west edge across the wheel.
const WHEEL_AXLE: (i32, i32) = (15, 16);
/// Where the posts holding the axle's ends stand (their bearing stones'
/// west column): on the bank's edge, and in the river past the far rim.
const WHEEL_POSTS: [i32; 2] = [0, 21];

/// The colour of the water wheel seen from above at `(x, y)` in its block
/// (px) on frame `f`, if any of it is there. It is a drum lying across the
/// stream: its top rolls toward the viewer's eye from the far end, so the
/// paddles crowd together toward both ends and darken there, the gaps
/// between them in shadow; the axle runs out of both rims; and white water
/// churns where the paddles come down into the river at the lower end.
fn water_wheel_top(x: i32, y: i32, f: u32) -> Option<u8> {
    use std::f32::consts::TAU;
    let (west, east) = WHEEL_RIMS;
    let (top, bottom) = WHEEL_ENDS;
    if !(0..32).contains(&x) {
        return None;
    }
    // The axle, from the bearing on the bank's edge across to the one on
    // the post in the river beyond the far rim.
    if (WHEEL_AXLE.0..=WHEEL_AXLE.1).contains(&y) && ((WHEEL_POSTS[0] + 4..west).contains(&x) || (east + 2..WHEEL_POSTS[1]).contains(&x)) {
        return Some(if y == WHEEL_AXLE.0 { grass::SAND[0] } else { grass::SOIL });
    }
    // Each end rests on a stone bearing atop a lauan post (its foot going
    // down toward the viewer into the bank or the water).
    for post in WHEEL_POSTS {
        let (dx, dy) = (x - post, y - WHEEL_AXLE.0);
        if (0..4).contains(&dx) && (-3..3).contains(&dy) {
            return Some(if dx == 0 || dy == -3 {
                grass::SAND[0]
            } else if dy == 2 {
                grass::SAND[2]
            } else {
                grass::SAND[1]
            });
        }
        if (1..3).contains(&dx) && (3..9).contains(&dy) {
            return Some(if dx == 1 { grass::SAND[2] } else { grass::SOIL });
        }
    }
    // Foam churned up below the lower end, flickering.
    if (bottom + 1..=bottom + 2).contains(&y) && (west..=east + 1).contains(&x) {
        return ((x * 3 + y + f as i32) % 4 == 0).then_some(grass::WHITE);
    }
    if !(top..=bottom).contains(&y) || !(west..=east + 1).contains(&x) {
        return None;
    }
    // Where on the drum (up the screen is away over the top): -1 at the
    // upper end, 1 at the lower.
    let half = (bottom - top) as f32 / 2.0;
    let s = ((y as f32 + 0.5 - top as f32 - half) / half).clamp(-1.0, 1.0);
    let phi = s.asin();
    let facing = phi.cos();
    // The hubs on the rims, by the axle.
    if (WHEEL_AXLE.0 - 1..=WHEEL_AXLE.1 + 1).contains(&y) && (x == west || x == east + 1) {
        return Some(grass::SOIL);
    }
    if x == west || x == east + 1 {
        return Some(if facing > 0.6 { grass::SAND[1] } else { grass::SAND[2] });
    }
    // A paddle where the turning drum brings one round (they come over the
    // top toward the eye: down the screen).
    // Along the drum's surface (px), the paddles evenly spaced round it;
    // seen from above, the gaps narrow as it curves away.
    let spacing = TAU * half / WHEEL_PADDLES as f32;
    let along = (phi * half - f as f32 / WHEEL_FRAMES as f32 * spacing).rem_euclid(spacing);
    if along.min(spacing - along) * facing < 1.0 {
        return Some(if facing > 0.8 {
            grass::SAND[0]
        } else if facing > 0.5 {
            grass::SAND[1]
        } else {
            grass::SAND[2]
        });
    }
    Some(if facing > 0.85 { grass::SOIL } else { grass::RAMP[0] })
}

/// The wood lathe in its two-by-two block of cells (px): the rows its bed
/// rails run along, the columns of its tailstock and headstock, and the
/// pulleys of the rope drive, on the wheel's axle (by the block's east
/// edge, on the axle's rows) and on the headstock's spindle.
const LATHE_RAILS: [i32; 2] = [19, 27];
const LATHE_BED: (i32, i32) = (3, 24);
const LATHE_TAIL: (i32, i32) = (3, 8);
const LATHE_HEAD: (i32, i32) = (16, 23);
const LATHE_DRIVE: (i32, i32, i32) = (28, 9, 22);
const LATHE_SPINDLE: (i32, i32, i32) = (24, 18, 27);

/// The colour of the wood lathe seen from above at `(x, y)` in its block
/// (px), if any of it is there: lauan (sand browns, lit on the top and the
/// left, the soil brown in shade), stone bearings (white), the billet
/// turned pale with grooves round it, and the rope.
/// The grass of a cell of the mill's ground worn away to earth tamped
/// hard (grass palette), a few tufts left along the ground's outer edge;
/// what isn't grass (the bank's lip by the river) is kept.
fn tamp(art: &mut PixelArt, map: &WorldMap, x: i32, y: i32) {
    let plot = map.spots.mill.map(crate::world::map::mill_plot);
    for py in 0..16 {
        for px in 0..16 {
            if !grass::RAMP.contains(&art.get(px, py)) {
                continue;
            }
            let (wx, wy) = (x * 16 + px as i32, y * 16 + py as i32);
            // How far in from the ground's outer edge (px).
            let inside = plot.map_or(16, |(x0, y0, x1, y1)| (wx - x0 * 16).min((x1 + 1) * 16 - 1 - wx).min(wy - y0 * 16).min((y1 + 1) * 16 - 1 - wy));
            if inside < 2 && hash_unit(wx, wy, 0xF0D1) < 0.5 {
                continue;
            }
            let n = hash_unit(wx, wy, 0xF0D2);
            art.set(
                px,
                py,
                if n < 0.18 {
                    grass::SOIL
                } else if n > 0.93 {
                    grass::SAND[1]
                } else {
                    grass::SAND[0]
                },
            );
        }
    }
}

/// The bench beside the mill's ground: its pixel at `(x, y)` in its cell,
/// if anything is there. A trestle table: a broad plank on two legs, the
/// plan (a pale board scratched with lines) and a hank of rope on it.
fn site_bench_top(x: i32, y: i32) -> Option<u8> {
    // The plan on the plank: a pale board, lines scratched on it.
    if (2..=7).contains(&x) && (3..=6).contains(&y) {
        return Some(if (y == 4 || y == 5) && (3..=6).contains(&x) && (x + y) % 3 != 0 { grass::SAND[0] } else { grass::WHITE });
    }
    // A hank of rope: a ring, its middle dark.
    let (rx, ry) = (x as f32 + 0.5 - 11.0, (y as f32 + 0.5 - 5.0) * 1.5);
    let r = (rx * rx + ry * ry).sqrt();
    if r <= 2.6 {
        return Some(if r < 1.0 {
            grass::SOIL
        } else if (x + y) % 2 == 0 {
            grass::SAND[2]
        } else {
            grass::SAND[1]
        });
    }
    // The plank: lit along its top, its front edge in shadow.
    if (0..=15).contains(&x) && (5..=9).contains(&y) {
        return Some(match y {
            5 => grass::SAND[2],
            6..=7 => grass::SAND[1],
            8 => grass::SAND[0],
            _ => grass::SOIL,
        });
    }
    // The two legs it stands on.
    if ((2..=3).contains(&x) || (12..=13).contains(&x)) && (10..=13).contains(&y) {
        return Some(if x == 2 || x == 12 { grass::SAND[0] } else { grass::SOIL });
    }
    None
}

/// Paints `top` (a cell's pixels, 0..16 each way) over `art`, outlined in
/// `outline` where it meets what's under it. Whether anything was painted.
fn paint_outlined(art: &mut PixelArt, top: impl Fn(i32, i32) -> Option<u8>, outline: u8) -> bool {
    let mut any = false;
    for py in 0..16i32 {
        for px in 0..16i32 {
            let c = top(px, py).or_else(|| [(-1, 0), (1, 0), (0, -1), (0, 1)].iter().any(|&(dx, dy)| top(px + dx, py + dy).is_some()).then_some(outline));
            if let Some(c) = c {
                art.set(px as usize, py as usize, c);
                any = true;
            }
        }
    }
    any
}

/// The face of one of the mill's walls at world pixel `(wx, wy)`, once
/// they're up, and whether it's the north wall's (its inner face, seen
/// over the ground) or the south wall's (its outer face): plaster from the
/// plate down to the ground, not across the doorway (where, once the roof
/// is on, the door hangs). The north posts show in the north face; the
/// south face runs over its posts and hides them.
fn wall_face_px(map: &WorldMap, wx: i32, wy: i32) -> Option<(u8, bool)> {
    let site = map.spots.mill.filter(|_| map.has_mill_walls())?;
    let (x0, y0, x1, y1) = crate::world::map::mill_plot(site);
    let mid = |c: i32| c * 16 + 8;
    let xm = (x0 + x1) / 2;
    if !(mid(x0) - 2..mid(x1) + 2).contains(&wx) {
        return None;
    }
    let on_post = [x0, xm, x1].iter().any(|&c| (mid(c) - 2..mid(c) + 2).contains(&wx));
    let base = |cy: i32| cy * 16 + 8 + crate::game::mill::POST_FOOT;
    if !on_post && (mid(x0)..mid(x1)).contains(&wx) && (post_head(y0, y1) + 4..base(y0)).contains(&wy) {
        return Some((plaster(wx, wy), true));
    }
    let (door0, door1) = crate::world::map::mill_doorway(site).map(|(x, _)| x).into();
    let in_door = (mid(door0) - 8..mid(door1) + 8).contains(&wx);
    if in_door && map.has_mill_roof() && (post_head(y1, y1) + 4..base(y1)).contains(&wy) {
        let open = map.mill_door() as i32 * crate::world::map::MILL_DOOR_STEP_PX;
        return Some((mill_door_px(wx - (mid(door0) - 8), wy - base(y1), open), false));
    }
    ((post_head(y1, y1) + 4..base(y1)).contains(&wy) && !in_door).then(|| (plaster(wx, wy), false))
}

/// The mill's door in its doorway (grass palette), at `(x, y)` from the
/// doorway's west edge and its foot (`y` < 0 up it), slid `open` px east
/// into the wall: one leaf of upright lauan planks, dark between them, a
/// batten across near the top and the bottom, a pull hollowed by its west
/// edge, running on a pale sill. Where it has slid away, the dark inside
/// under the roof.
fn mill_door_px(x: i32, y: i32, open: i32) -> u8 {
    let dark = grass::RAMP[0];
    if y >= -2 {
        return if y == -2 { grass::SAND[2] } else { grass::SOIL };
    }
    let lx = x - open;
    if lx < 0 {
        return if y < -8 || (x + y).rem_euclid(2) == 0 { dark } else { grass::SOIL };
    }
    let w = MILL_DOOR_W;
    let pull = (2..4).contains(&lx) && (-14..-10).contains(&y);
    if lx == 0 || lx == w - 1 || pull {
        return dark;
    }
    if (-6..-3).contains(&y) || (-21..=-18).contains(&y) {
        return match y {
            -6 | -21 => grass::SAND[0],
            -4 | -18 => dark,
            _ => grass::SOIL,
        };
    }
    match lx.rem_euclid(4) {
        0 => dark,
        1 if hash_unit(x, y, 0xD00E) < 0.6 => grass::SAND[0],
        _ => grass::SOIL,
    }
}

/// How wide the mill's door is (px): the doorway's two cells.
const MILL_DOOR_W: i32 = 32;

/// Clay and soil plastered over bamboo lath (grass palette): a pale earth,
/// flecked darker and lighter, and here and there, where it's thin, the
/// lath showing through it: a grid of brown strips.
fn plaster(wx: i32, wy: i32) -> u8 {
    let thin = hash_unit(wx.div_euclid(8), wy.div_euclid(6), 0x1A7B) < 0.45;
    let lath = wx.rem_euclid(4) == 0 || wy.rem_euclid(3) == 0;
    if thin && lath && hash_unit(wx, wy, 0x1A7C) < 0.8 {
        return grass::SOIL;
    }
    let n = hash_unit(wx, wy, 0x5A11);
    if n < 0.12 {
        grass::SAND[0]
    } else if n > 0.94 {
        grass::SAND[2]
    } else {
        grass::SAND[1]
    }
}

/// The mill's thatched gable roof at world pixel `(wx, wy)`, once it's on
/// (grass palette), seen from the south: its ridge along the north plate,
/// capped with a dark bundle held by crossed sticks astride it, the slope
/// running down in courses of straw to the eaves over the south wall, its
/// cut ends thick there; a little past the end posts each way, the verges
/// a little darker. The ridge rises no higher than the north plate, clear
/// of the river's water drawn on BG2 north-east of the ground.
fn roof_px(map: &WorldMap, wx: i32, wy: i32) -> Option<u8> {
    let (x0, y0, x1, y1) = crate::world::map::mill_plot(map.spots.mill?);
    let mid = |c: i32| c * 16 + 8;
    let (l, r) = (mid(x0) - 7, mid(x1) + 7);
    let ridge = post_head(y0, y1);
    let eave = post_head(y1, y1) + 10;
    // The crossed sticks astride the ridge, standing above it.
    if (l + 6..r - 6).contains(&wx) && (ridge - 4..ridge + 5).contains(&wy) {
        let k = (wx - l - 6).rem_euclid(20);
        let dy = wy - (ridge - 4);
        if k == dy / 2 + 4 || k == 8 - dy / 2 {
            return Some(grass::SOIL);
        }
    }
    if !(l..r).contains(&wx) || !(ridge..eave).contains(&wy) {
        return None;
    }
    let n = hash_unit(wx, wy, 0x7A7C);
    let dy = wy - ridge;
    let to_eave = eave - 1 - wy;
    // The ridge: a thick dark bundle laid along the top.
    const CAP: i32 = 6;
    if dy < CAP {
        return Some(match dy {
            0 => grass::SAND[0],
            d if d == CAP - 1 => grass::SOIL,
            _ if (wx + dy).rem_euclid(2) == 0 => grass::SOIL,
            _ => grass::SAND[0],
        });
    }
    // The eaves: the thick cut ends of the thatch, lit along their top.
    if to_eave == 0 {
        return Some(grass::SOIL);
    }
    if to_eave < 4 {
        return Some(if (wx + to_eave).rem_euclid(2) == 0 && n < 0.7 { grass::SOIL } else { grass::SAND[0] });
    }
    if to_eave == 4 {
        return Some(grass::SAND[2]);
    }
    // The verges: the ends of the slope, a little darker.
    if wx - l < 2 || r - 1 - wx < 2 {
        return Some(if n < 0.4 { grass::SOIL } else { grass::SAND[0] });
    }
    // The slope: courses of thatch, each a row of straw strokes, shaded
    // under the course above.
    const COURSE: i32 = 5;
    let (c, in_c) = ((dy - CAP) / COURSE, (dy - CAP) % COURSE);
    let stroke = (wx + c * 2 + (hash_unit(wx, c, 0x51) * 2.0) as i32).rem_euclid(3) == 0;
    Some(match in_c {
        0 if n < 0.7 => grass::SOIL,
        0 => grass::SAND[0],
        1 if stroke => grass::SOIL,
        1 => grass::SAND[0],
        _ if stroke => grass::SAND[0],
        _ if n > 0.85 && in_c == COURSE - 1 => grass::SAND[2],
        _ => grass::SAND[1],
    })
}

/// What spans between the posts' heads at world pixel `(wx, wy)`: the
/// lauan plates along the north and south walls (the pine beams across
/// between them are sprites, `Game::draw_mill_posts`).
fn span_px(map: &WorldMap, wx: i32, wy: i32) -> Option<u8> {
    let (x0, y0, x1, y1) = crate::world::map::mill_plot(map.spots.mill?);
    let mid = |c: i32| c * 16 + 8;
    for cy in [y0, y1] {
        let dy = wy - post_head(cy, y1);
        if (0..4).contains(&dy) && (mid(x0) - 2..mid(x1) + 2).contains(&wx) {
            return Some(match dy {
                0 => grass::SAND[1],
                3 => grass::SOIL,
                _ => grass::SAND[0],
            });
        }
    }
    None
}

/// A flat stone bedded in the mill's foundation, in the middle of its
/// cell: a pale top, shaded toward its lower right.
fn foundation_stone(x: i32, y: i32) -> Option<u8> {
    let (dx, dy) = ((x as f32 + 0.5 - 8.0) / 5.5, (y as f32 + 0.5 - 8.5) / 4.0);
    let d = dx * dx + dy * dy;
    (d <= 1.0).then(|| {
        if dx + dy > 0.7 {
            grass::SAND[1]
        } else if d < 0.25 && dx + dy < -0.2 {
            grass::WHITE
        } else {
            grass::SAND[2]
        }
    })
}

/// The marking's pixel at `(x, y)` in its cell, if anything is there: the
/// stake (at a corner) in front of the rope.
fn marking_top(m: Marking, x: i32, y: i32) -> Option<u8> {
    if m.stake
        && let Some(c) = marking_stake(x, y)
    {
        return Some(c);
    }
    marking_rope(m, x, y)
}

/// A stake split from bamboo, driven into the middle of a cell: a pale
/// cut top, a lit face and a shaded one.
fn marking_stake(x: i32, y: i32) -> Option<u8> {
    if !(6..=9).contains(&x) || !(2..=10).contains(&y) {
        return None;
    }
    Some(if y == 2 {
        grass::SAND[2]
    } else if x <= 7 {
        grass::SAND[1]
    } else {
        grass::SAND[0]
    })
}

/// Rope lying through the middle of a cell, out each way it runs: two
/// pixels thick, twisted (light and dark by turns).
fn marking_rope(m: Marking, x: i32, y: i32) -> Option<u8> {
    let (mx, my) = (7..=8, 7..=8);
    let twist = |a: i32, b: i32| if (a + b) % 3 == 0 { grass::SAND[0] } else { grass::SAND[2] };
    let across = (my.contains(&y)) && ((m.ropes & Marking::WEST != 0 && x <= 8) || (m.ropes & Marking::EAST != 0 && x >= 7) || (m.stake && mx.contains(&x)));
    let along = (mx.contains(&x)) && ((m.ropes & Marking::NORTH != 0 && y <= 8) || (m.ropes & Marking::SOUTH != 0 && y >= 7));
    (across || along).then(|| twist(x, y))
}

fn lathe_top(x: i32, y: i32) -> Option<u8> {
    if !(0..32).contains(&x) || !(0..32).contains(&y) {
        return None;
    }
    let wood = |lit: bool, shade: bool| {
        if lit {
            grass::SAND[1]
        } else if shade {
            grass::SOIL
        } else {
            grass::SAND[0]
        }
    };
    // The wheel's axle comes in from the east to the drive pulley.
    if (WHEEL_AXLE.0..=WHEEL_AXLE.1).contains(&y) && x > LATHE_DRIVE.0 {
        return Some(if y == WHEEL_AXLE.0 { grass::SAND[0] } else { grass::SOIL });
    }
    // The pulleys, seen edge on: a rim of lauan, a groove of rope.
    for (px, top, bottom) in [LATHE_DRIVE, LATHE_SPINDLE] {
        if (px..=px + 1).contains(&x) && (top..=bottom).contains(&y) {
            return Some(if x == px { grass::SAND[2] } else { wood(false, y == bottom) });
        }
    }
    // The rope drive, running down from the axle's pulley to the
    // spindle's, crossed.
    for (from, to) in
        [((LATHE_DRIVE.0, LATHE_DRIVE.1), (LATHE_SPINDLE.0 + 1, LATHE_SPINDLE.2)), ((LATHE_DRIVE.0, LATHE_DRIVE.2), (LATHE_SPINDLE.0 + 1, LATHE_SPINDLE.1))]
    {
        let (dx, dy) = (to.0 - from.0, to.1 - from.1);
        if (from.1.min(to.1)..=from.1.max(to.1)).contains(&y) {
            let along = (y - from.1) as f32 / dy as f32;
            if (x as f32 - (from.0 as f32 + along * dx as f32)).abs() < 0.6 {
                return Some(if y % 2 == 0 { grass::SAND[2] } else { grass::SAND[1] });
            }
        }
    }
    // The headstock: a tall block, its top lit, with a stone bearing for
    // the spindle.
    let (h0, h1) = LATHE_HEAD;
    if (h0..=h1).contains(&x) && (13..=29).contains(&y) {
        if (h0 + 2..=h0 + 4).contains(&x) && (15..=16).contains(&y) {
            return Some(if y == 15 { grass::WHITE } else { grass::SAND[2] });
        }
        return Some(wood(y <= 16 || x == h0, x == h1 || y >= 28));
    }
    // The tailstock, lower, with its own bearing.
    let (t0, t1) = LATHE_TAIL;
    if (t0..=t1).contains(&x) && (17..=29).contains(&y) {
        if (t0 + 2..=t0 + 3).contains(&x) && (19..=20).contains(&y) {
            return Some(if y == 19 { grass::WHITE } else { grass::SAND[2] });
        }
        return Some(wood(y <= 19 || x == t0, x == t1 || y >= 28));
    }
    // The billet held between them, turned round: pale, ringed by grooves.
    if (t1 + 1..h0).contains(&x) && (21..=24).contains(&y) {
        return Some(if x % 3 == 0 {
            grass::SAND[0]
        } else if y == 21 {
            grass::WHITE
        } else if y == 24 {
            grass::SAND[1]
        } else {
            grass::SAND[2]
        });
    }
    // The bed: two rails along the bank, legs under their ends.
    let (b0, b1) = LATHE_BED;
    for rail in LATHE_RAILS {
        if (b0..=b1).contains(&x) && (rail..=rail + 1).contains(&y) {
            return Some(wood(y == rail, false));
        }
    }
    if [b0 + 1, b1 - 1].contains(&x) && (LATHE_RAILS[1] + 2..=LATHE_RAILS[1] + 3).contains(&y) {
        return Some(grass::SOIL);
    }
    None
}

/// Row of the cliff's edge cell from which a stood ladder's tops stick up.
const LADDER_TOP: usize = 9;

/// A ladder's wood in one palette: dark, mid, light, and the rope.
type LadderColours = [u8; 4];
const LADDER_PALETTE_ROCK: LadderColours = [rock::WOOD[0], rock::WOOD[1], rock::WOOD[2], rock::WOOD[3]];
const LADDER_PALETTE_GRASS: LadderColours = [grass::SOIL, grass::SAND[0], grass::SAND[1], grass::SAND[2]];

/// The pixels of a stood ladder on one of its rows (`y` counted from the
/// tops of its poles down), as (x in the cell, colour): two poles lit on
/// the left, and every 7 rows a rung lashed across them.
fn ladder_pixel(y: i32, c: LadderColours) -> impl Iterator<Item = (usize, u8)> {
    let [dark, mid, light, rope] = c;
    let rung = y > 2 && y % 7 == 3;
    let under = y > 2 && y % 7 == 4;
    let poles = [(3, light), (4, mid), (5, dark), (11, light), (12, mid), (13, dark)];
    let pole = poles.into_iter().map(move |(x, v)| (x, if rung && x != 5 && x != 13 { rope } else { v }));
    let bar = (6..11).filter(move |_| rung || under).map(move |x| (x, if rung { light } else { dark }));
    pole.chain(bar)
}

/// Where a cliff face cell sits in its face: its row from 1 at the top,
/// and how many rows the face has.
fn face_rows(map: &WorldMap, x: i32, y: i32) -> (i32, i32) {
    let row = (1..).find(|&k| map.terrain(x, y - k) != Terrain::Cliff).unwrap_or(1);
    (row, row + (1..).find(|&k| map.terrain(x, y + k) != Terrain::Cliff).unwrap_or(1) - 1)
}

/// How far from the river (px) the sand along it is soaked.
const WET_BANK: f32 = 9.0;

/// Depth into the river (px; negative on the bank) with the wet edge's
/// raggedness added.
fn river_wet(d: f32, wx: i32, wy: i32) -> f32 {
    d + (noise_px(wx / 3, wy / 3, 0x3E7D) - 0.5) * 3.0
}

/// A pebble washed down by the river on its sandy bank, if one is at this
/// pixel: its shade (0 dark rim, 1 body, 2 lit from the top-left).
fn pebble(d: f32, wx: i32, wy: i32) -> Option<usize> {
    const GRID: i32 = 6;
    if !(-12.0..-1.5).contains(&d) {
        return None;
    }
    let (gx, gy) = (wx.div_euclid(GRID), wy.div_euclid(GRID));
    // Fewer the further from the water.
    if hash_unit(gx, gy, 0x9EB1) > 0.5 * (1.0 + d / 14.0) {
        return None;
    }
    let cx = (gx * GRID) as f32 + 3.0 + (hash_unit(gx, gy, 0x9EB2) - 0.5) * 1.5;
    let cy = (gy * GRID) as f32 + 3.0 + (hash_unit(gx, gy, 0x9EB3) - 0.5) * 1.5;
    let radius = 1.4 + hash_unit(gx, gy, 0x9EB4) * 1.0;
    let (dx, dy) = (wx as f32 + 0.5 - cx, (wy as f32 + 0.5 - cy) * 1.25);
    let sd = (dx * dx + dy * dy).sqrt();
    if sd > radius {
        return None;
    }
    Some(if sd > radius - 0.7 || dx + dy > 0.8 {
        0
    } else if dx + dy < -0.6 {
        2
    } else {
        1
    })
}

/// One pixel of deep water rising toward the shallows (deep palette).
fn reef_px(base: u8, h: f32, x: i32, y: i32) -> u8 {
    if h > DEEP_BELOW + 0.4 {
        deep::SHELF[1]
    } else if h > DEEP_BELOW - 0.4 || (h > DEEP_BELOW - 1.0 && bayer(x, y) < 0.5) {
        deep::SHELF[0]
    } else {
        base
    }
}

/// One pixel of the current out at sea (deep palette): deep water at the
/// shelf's colour along its edges, where it meets the shallows, with
/// streaks of the river's colours racing out along the flow.
fn sea_current_px(r: &RiverField, i: usize, base: u8, h: f32, wx: i32, wy: i32) -> u8 {
    // Out in the deep its edge melts into the sea.
    let d = if h < DEEP_BELOW { f32::MAX } else { r.depth[i] };
    if d < 1.5 || (d < 3.0 && bayer(wx, wy) < 0.5) {
        return deep::SHELF[0];
    }
    let lane = (r.across[i] / 2.0).floor() as i32;
    let along = r.along[i] + hash_unit(lane, 0, 0xC0E1) * 40.0;
    let dash = (along / 16.0).floor() as i32;
    // The streaks thin out over the deep, where the current loses itself.
    let fade = ((h - DEEP_BELOW + 1.5) / 1.5).clamp(0.0, 1.0);
    if d > 3.5 && hash_unit(lane, dash, 0xC0E2) < 0.2 * fade && along.rem_euclid(16.0) < 7.0 {
        return deep::RIVER_FLOW[(r.along[i] / 2.0).floor().rem_euclid(3.0) as usize];
    }
    base
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

/// Size (px) of the grid the rocks of a cliff face are scattered on; the
/// face repeats every 64 px along, so the grid's columns wrap at
/// `64 / ROCK_W`.
const ROCK_W: i32 = 8;
const ROCK_H: i32 = 7;

/// The rock a point of a cliff face belongs to: the nearest of the stones
/// scattered one to a grid square (a Voronoi pattern), by its grid square.
fn rock_at(x: i32, y: i32) -> (i32, i32) {
    let (gx, gy) = (x.div_euclid(ROCK_W), y.div_euclid(ROCK_H));
    let mut best = (i32::MAX, (0, 0));
    for oy in -1..=1 {
        for ox in -1..=1 {
            let (cx, cy) = (gx + ox, gy + oy);
            let col = cx.rem_euclid(64 / ROCK_W);
            let sx = cx * ROCK_W + (hash_unit(col, cy, 0xC11F) * ROCK_W as f32) as i32;
            let sy = cy * ROCK_H + (hash_unit(col, cy, 0xC120) * ROCK_H as f32) as i32;
            // Stretched across, so the rocks lie in rough ledges.
            let d = (x - sx).pow(2) + (y - sy).pow(2) * 2;
            if d < best.0 {
                best = (d, (col, cy));
            }
        }
    }
    best.1
}

/// The cliffs' sandstone (rock palette) as a ramp, dark to light, in place
/// of the stones' greys.
const SANDSTONE: [u8; 6] = [rock::WOOD[0], rock::SAND_SHADOW, rock::SAND[0], rock::SAND[1], rock::SAND[2], rock::SAND[2]];

/// Whether the edge of the high ground drops away only to the side
/// (left, right), not to a face below it.
fn cliff_side(map: &WorldMap, x: i32, y: i32) -> Option<(bool, bool)> {
    if map.terrain(x, y) != Terrain::CliffTop || dip(map, x, y).is_some() || dip(map, x, y + 1).is_some_and(|(_, oy)| oy == 0) {
        return None;
    }
    let drop = |dx: i32, dy: i32| !matches!(map.terrain(x + dx, y + dy), Terrain::Highland | Terrain::CliffTop | Terrain::River | Terrain::Slope);
    let (left, right) = (drop(-1, 0), drop(1, 0));
    (!drop(0, 1) && (left || right)).then_some((left, right))
}

/// How wide (px) the rock of a side seen from above is, before its ragged
/// lip.
const CLIFF_SIDE_BAND: i32 = 4;

/// One pixel of the side of the high ground as a narrow band of rock along
/// the drop (rock palette), with grass hanging over its lip; 0 (clear)
/// past it.
fn cliff_side_band_pixel(left: bool, right: bool, wx: i32, wy: i32) -> u8 {
    let px = wx.rem_euclid(16);
    let width = |k: i32| CLIFF_SIDE_BAND + (noise_px(wy / 2, k, 0x51DE) * 3.0) as i32;
    let reach = |open: bool, from: i32, k: i32| if open { from - width(k) } else { i32::MAX };
    let past = reach(left, px, 0).min(reach(right, 15 - px, 1));
    if past >= 1 {
        0
    } else if past == 0 {
        if bayer(wx, wy) < 0.5 { rock::MOSS } else { SANDSTONE[1] }
    } else {
        cliff_side_pixel(left, right, wx, wy)
    }
}

/// One pixel of the side of the high ground seen from above (rock
/// palette): the sandstone of its face, with grass hanging over the top
/// edge on the high side and a dark drop on the open side.
fn cliff_side_pixel(left: bool, right: bool, wx: i32, wy: i32) -> u8 {
    let s = SANDSTONE;
    let (x, y) = (wx.rem_euclid(64), wy.rem_euclid(64));
    // The rocks lie the other way round: in columns down the side.
    let here = rock_at(y, x);
    let b = bayer(wx, wy);
    let px = x.rem_euclid(16);
    let crack = rock_at(y, x + 1) != here || rock_at(y + 1, x) != here;
    let lit = rock_at(y, x - 1) != here || rock_at(y - 1, x) != here;
    let tone = hash_unit(here.0, here.1, 0xC123);
    // Distance (px) in from the open side, and from the high side.
    let from_open = if left { px } else { 15 - px };
    let from_high = if left && right { 16 } else { 15 - from_open };
    if left && right && (px == 0 || px == 15) || (!(left && right) && from_open == 0) {
        s[0]
    } else if from_high < 2 + (noise_px(wx, wy / 2, 0x3056) * 2.0) as i32 {
        // Grass hanging over the edge.
        if b < 0.5 { rock::MOSS } else { s[1] }
    } else if crack {
        s[0]
    } else if lit {
        if tone < 0.5 { s[4] } else { s[3] }
    } else if from_open < 3 {
        s[1]
    } else if b < 0.2 + tone * 0.6 {
        s[3]
    } else {
        s[2]
    }
}

/// One pixel of a cliff face (rock palette). `row` counts from 1 at the
/// top of a face `rows` cells tall; `wx` runs along the face (px, repeating
/// every 64), `py` down the cell.
fn cliff_pixel(row: i32, rows: i32, left: bool, right: bool, wet: bool, wx: i32, py: i32) -> u8 {
    sandstone_pixel(row, rows, left, right, wet, false, wx, py)
}

/// How far around the screen (px) ground must be ready before it is drawn,
/// how far ahead of the screen chunks are built a little each frame, and
/// past what distance built chunks are put away (their blocks freed).
const NEEDED_MARGIN: i32 = 16;
const PREFETCH_MARGIN: i32 = CHUNK_PX / 2;
const KEEP_MARGIN: i32 = CHUNK_PX;
/// How much building ahead is done per frame, in plain cells: about 4 rows
/// of a chunk. Cells by the river cost `RIVER_CELL_COST` each (their water
/// and banks are worked out pixel by pixel), so fewer rows go in a frame
/// there; at least one row always does.
const PREFETCH_BUDGET: i32 = 4 * CHUNK_CELLS;
const RIVER_CELL_COST: i32 = 8;

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
        let visual = VisualMap { ground: TileGrid::new(map.width(), map.height(), border), overhead: TileGrid::new(map.width(), map.height(), BgCell::EMPTY) };
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
        let ahead = chunks_touching(grow(PREFETCH_MARGIN), self.chunks).find(|&id| self.slot(id).is_some_and(|i| self.built[i] != Built::Yes));
        if let Some(id) = ahead {
            let rows = self.rows_in_budget(map, id);
            self.build_rows(map, assets, visual, id, rows);
        }
    }

    /// How many of a chunk's rows still to build fit in a frame's building
    /// ahead (`PREFETCH_BUDGET`).
    fn rows_in_budget(&self, map: &WorldMap, id: ChunkId) -> i32 {
        let done = match self.slot(id).map(|i| self.built[i]) {
            Some(Built::Rows(r)) => r,
            _ => 0,
        };
        let (x0, y0) = id.origin();
        let cost = |y: i32| (x0..x0 + CHUNK_CELLS).map(|x| if map.is_riverside(x, y) { RIVER_CELL_COST } else { 1 }).sum::<i32>();
        let mut spent = 0;
        let mut rows = 0;
        for y in y0 + done..y0 + CHUNK_CELLS {
            spent += cost(y);
            if rows > 0 && spent > PREFETCH_BUDGET {
                break;
            }
            rows += 1;
        }
        rows.max(1)
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
        Feature::Pine(p) => {
            // Laid out like a broadleaf: the tiers on BG2, their shadow and
            // the trunk's foot on BG1.
            let col = if p.flip { PinePart::W as u8 - 1 - p.dx } else { p.dx } as usize;
            match p.dy {
                3 => b.prop(29 + col as u8, &props.pine_base[col], p.flip, x, y),
                dy => {
                    overhead = BgCell { tile: props.pine_upper[dy as usize * 3 + col], hflip: p.flip, high: true };
                    if dy == 2 { b.prop(32 + col as u8, &props.pine_shade[col], p.flip, x, y) } else { b.terrain(map, x, y) }
                }
            }
        }
        Feature::PineCone => b.prop(35, &props.pine_cone, x % 2 == 0, x, y),
        Feature::PineStump(s) => b.prop(36, &props.pine_stump, s.flip, x, y),
        Feature::BambooStump(s) => b.prop(43, &props.bamboo_stump, s.flip, x, y),
        Feature::Bamboo(p) => {
            // The stalk and leaves on BG2, their shadow and the stalk's
            // foot on BG1.
            let v = p.v as usize % 3;
            match p.dy {
                3 => b.prop(37 + v as u8, &props.bamboo_base[v], p.flip, x, y),
                dy => {
                    overhead = BgCell { tile: props.bamboo_upper[v][dy as usize], hflip: p.flip, high: true };
                    if dy == 2 { b.prop(40 + v as u8, &props.bamboo_shade[v], p.flip, x, y) } else { b.terrain(map, x, y) }
                }
            }
        }
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
        Feature::Ladder(part) => b.ladder(map, x, y, part),
        Feature::Bridge(part) => b.bridge(map, river.as_ref(), x, y, part),
        Feature::WaterWheel(part) => b.water_wheel(map, river.as_ref(), x, y, part),
        Feature::Lathe(part) => b.lathe(map, river.as_ref(), x, y, part),
        Feature::Marking(m) => b.marking(map, river.as_ref(), x, y, m),
        Feature::Foundation(stone) => b.foundation(map, river.as_ref(), x, y, stone),
        Feature::Framed(post) => b.framed(map, river.as_ref(), x, y, post, false),
        Feature::MillWall => b.framed(map, river.as_ref(), x, y, false, true),
        Feature::MillDoor => b.framed(map, river.as_ref(), x, y, false, false),
        Feature::SiteBench => b.site_bench(x, y),
        Feature::Flower => b.prop(5, &props.flowers, x % 2 == 1, x, y),
        Feature::ClayBed(c) => b.clay_bed(x, y, c),
        Feature::BareEarth(part) => {
            let (grass, earth) = b.bare_earth(x, y, part);
            overhead = BgCell { tile: earth, hflip: false, high: false };
            grass
        }
        // A signal fire is a sprite standing on the ground as it was.
        Feature::None | Feature::SignalFire(_) => match (&river, map.terrain(x, y)) {
            (Some(r), Terrain::Cliff) => b.cliff_face(map, x, y, Some(r)),
            (Some(r), _) => {
                let f = CellField::sample(map, x, y);
                let lake_shore = f.min < 0.6 && f.max > 0.0 && map.is_high(x, y) && map.terrain(x, y) != Terrain::DeepWater && near_mud(map, x, y);
                if map.is_high(x, y) && f.max <= 0.0 {
                    // The stream's current across the lake's shallows: the
                    // lake (shore palette) clear where the current runs, for
                    // it to show on BG2.
                    b.lake_current(&f, r, x, y)
                } else if lake_shore && (r.max > 0.0 || map.terrain(x, y) == Terrain::River) {
                    let (mouth, soil) = b.lake_mouth(&f, r, x, y);
                    overhead = BgCell { tile: soil, hflip: false, high: false };
                    mouth
                } else if lake_shore {
                    // Beside the mouth, the shore as anywhere along the lake.
                    let (ground, soil) = b.ground(map, x, y, true);
                    if let Some(soil) = soil {
                        overhead = BgCell { tile: soil, hflip: false, high: false };
                    }
                    ground
                } else {
                    b.river_bank(map, &f, r, x, y)
                }
            }
            (None, _) => match b.cliff_foot(map, x, y) {
                Some((ground, scree)) => {
                    overhead = BgCell::new(scree);
                    ground
                }
                None => {
                    let (ground, under) = b.ground(map, x, y, true);
                    if let Some(u) = under {
                        overhead = BgCell { tile: u, hflip: false, high: false };
                    }
                    ground
                }
            },
        },
    };
    if let Some((cut, oy)) = dip(map, x, y).filter(|&(_, oy)| oy == 0 && overhead.is_empty()) {
        overhead = BgCell::new(b.unique_art(PAL_GRASS, &crumbled_grass(b.tex, cut, oy, x, y)));
    }
    // The mill's frame: the posts' heads and the plates and beams between
    // them, in front of the castaway (who walks under them).
    if overhead.is_empty()
        && let Some(tile) = b.mill_frame_overhead(map, x, y)
    {
        overhead = BgCell { tile, hflip: false, high: true };
    }
    if let Some(r) = river.filter(|r| r.max > 0.0 && overhead.is_empty()) {
        overhead = BgCell { tile: b.river_water(map, &r, x, y), hflip: false, high: false };
    }
    (BgCell::new(tile), overhead)
}

/// How far out from the middle of a two-by-two patch (clay or bare earth)
/// the pixel at `(u, v)` (0..32 from its top-left cell `anchor`) lies: under
/// 1 is inside. Its rim wobbles, differently for each patch, so no two look
/// alike and none looks drawn with compasses.
fn patch_reach(anchor: (i32, i32), u: i32, v: i32) -> f32 {
    let (dx, dy) = ((u as f32 + 0.5 - 16.0) / 14.5, (v as f32 + 0.5 - 16.5) / 12.5);
    let a = dy.atan2(dx);
    let phase = |k: u32| hash_unit(anchor.0, anchor.1, k) * std::f32::consts::TAU;
    let wobble = 1.0 + 0.11 * (2.0 * a + phase(1)).sin() + 0.08 * (3.0 * a + phase(2)).sin() + 0.05 * (5.0 * a + phase(3)).sin();
    dx.hypot(dy) / wobble
}

/// A ragged rim for a patch: a pixel's own jitter on its reach.
fn patch_rim(anchor: (i32, i32), u: i32, v: i32, seed: u32) -> f32 {
    hash_unit(anchor.0 * 32 + u, anchor.1 * 32 + v, seed) * 0.14
}

/// One pixel of a clay bed (rock palette) over the shore's `mud`: a flat,
/// slick slab of blue-grey clay lying in the mud, darker where it's
/// wettest, with streaks of sheen and a dark seam where it meets the mud.
/// Dug, a hole is cut in its middle, its far side in shadow and water
/// seeping into its bottom.
fn clay_bed_pixel(anchor: (i32, i32), u: i32, v: i32, dug: bool, mud: u8) -> u8 {
    let reach = patch_reach(anchor, u, v);
    let rim = patch_rim(anchor, u, v, 0xC1A7);
    if reach > 1.0 - rim {
        return mud;
    }
    let (wx, wy) = (anchor.0 * 32 + u, anchor.1 * 32 + v);
    let speck = hash_unit(wx, wy, 0x5EE7);
    if reach > 0.9 - rim {
        return if speck < 0.6 { rock::WOOD[0] } else { rock::STONE[1] };
    }
    if dug {
        let hole = ((u as f32 + 0.5 - 16.0) / 7.0).powi(2) + ((v as f32 + 0.5 - 17.0) / 5.0).powi(2);
        if hole < 1.0 {
            // Water seeping in shines at the bottom of the hole.
            let pool = v >= 19 && hole < 0.55;
            return if pool && speck < 0.45 {
                rock::STONE[3]
            } else if pool {
                rock::STONE[2]
            } else {
                rock::STONE[0]
            };
        }
        if hole < 1.45 {
            // The cut sides: the far one in shadow, the near one lit.
            return if v < 17 { rock::STONE[1] } else { rock::STONE[4] };
        }
    }
    // Flat: wet patches (slow noise) rather than a lit dome; sheen in
    // short streaks along the slab.
    let wet = value_noise(wx as f32 / 9.0, wy as f32 / 6.0, anchor.0 as u32 * 31 + anchor.1 as u32);
    let streak = hash_unit(wx / 4, wy, 0x51EE) > 0.9 && reach < 0.75;
    let d = bayer(wx, wy);
    if streak {
        rock::STONE[4]
    } else if wet + d * 0.2 > 0.62 {
        rock::STONE[2]
    } else if speck > 0.97 {
        rock::WOOD[1]
    } else {
        rock::STONE[3]
    }
}

/// Whether the earth shows through the grass at a pixel of a bare patch
/// (with a few tufts of grass left standing near its rim).
fn earth_shows(anchor: (i32, i32), u: i32, v: i32) -> bool {
    let reach = patch_reach(anchor, u, v);
    let rim = patch_rim(anchor, u, v, 0xEA27);
    reach < 1.0 - rim && !(reach > 0.86 && hash_unit(anchor.0 * 32 + u, anchor.1 * 32 + v, 0x7F7F) < 0.25)
}

/// One pixel of bare earth (rock palette): dry, flat brown ground,
/// mottled darker and lighter, with grit and the odd pebble, and the
/// grass's shadow along its top rim.
fn bare_earth_pixel(anchor: (i32, i32), u: i32, v: i32) -> u8 {
    let (wx, wy) = (anchor.0 * 32 + u, anchor.1 * 32 + v);
    let speck = hash_unit(wx, wy, 0xD127);
    let reach = patch_reach(anchor, u, v);
    // The rim of grass above throws a little shadow onto the earth.
    if reach > 0.86 && v < 16 {
        return rock::WOOD[0];
    }
    if speck > 0.985 {
        return rock::STONE[4];
    }
    if speck > 0.97 {
        return rock::STONE[2];
    }
    let mottle = value_noise(wx as f32 / 6.0, wy as f32 / 5.0, anchor.0 as u32 * 17 + anchor.1 as u32) + (speck - 0.5) * 0.5;
    if mottle > 0.6 {
        rock::WOOD[2]
    } else if mottle > 0.15 {
        rock::WOOD[1]
    } else {
        rock::WOOD[0]
    }
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
    let tile = if p.is_crown() { tiles.crowns[(map.fruit(p.anchor(x, y)) as usize).min(tiles.crowns.len() - 1)] } else { tiles.upper[dy * 3 + col] };
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

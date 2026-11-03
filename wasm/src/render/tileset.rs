//! Builds the background metatile map from world terrain, generating
//! transition ("autotile") metatiles for coastlines and grass edges.
//!
//! Transition tiles are composed once at load time and stored in VRAM like
//! any other CHR — the runtime cost is just a table lookup.

use std::collections::HashMap;

use crate::assets::palettes::{BG_DEEP, BG_SHORE, BG_VEG};
use crate::assets::{Assets, SURF_PERIOD, TerrainArt, WATER_PERIOD};
use crate::common::rng::hash2;
use crate::ppu::BgSource;
use crate::ppu::banks::{MetatileBank, MetatileId};
use crate::ppu::pattern::PixelArt;
use crate::world::terrain::PalmPart;
use crate::world::{Feature, Terrain, WorldMap};

/// Background map ready to hand to the PPU.
pub struct VisualMap {
    w: i32,
    h: i32,
    ids: Vec<MetatileId>,
    /// Drawn outside the map (open sea).
    border: MetatileId,
}

impl BgSource for VisualMap {
    #[inline]
    fn metatile_at(&self, mx: i32, my: i32) -> MetatileId {
        if mx < 0 || my < 0 || mx >= self.w || my >= self.h {
            self.border
        } else {
            self.ids[(my * self.w + mx) as usize]
        }
    }
}

// Neighbour mask bits.
const N: u8 = 1;
const E: u8 = 2;
const S: u8 = 4;
const W: u8 = 8;
const NE: u8 = 16;
const SE: u8 = 32;
const SW: u8 = 64;
const NW: u8 = 128;

/// Edge wobble so coastlines don't look ruled. Ends match so tiles join.
const WOBBLE: [f32; 16] = [0.0, 0.5, 1.0, 1.5, 1.5, 1.0, 0.5, 0.0, 0.0, 0.5, 1.0, 1.0, 0.5, 0.0, 0.0, 0.0];

/// Does neighbour terrain `n` "intrude" into a cell of terrain `t`?
fn intrudes(t: Terrain, n: Terrain) -> bool {
    match t {
        Terrain::DeepWater => n != Terrain::DeepWater,
        Terrain::ShallowWater => n.is_land(),
        Terrain::Sand => false,
        Terrain::Grass => n != Terrain::Grass,
    }
}

fn edge_mask(map: &WorldMap, x: i32, y: i32) -> u8 {
    let t = map.terrain(x, y);
    // Off-map counts as the same terrain so edges don't appear at the border.
    let hit = |dx: i32, dy: i32| {
        let (nx, ny) = (x + dx, y + dy);
        map.in_bounds(nx, ny) && intrudes(t, map.terrain(nx, ny))
    };
    let mut m = 0;
    for (bit, dx, dy) in [(N, 0, -1), (E, 1, 0), (S, 0, 1), (W, -1, 0)] {
        if hit(dx, dy) {
            m |= bit;
        }
    }
    // Corners only matter when neither adjacent side is already an edge.
    for (bit, dx, dy, a, b) in [(NE, 1, -1, N, E), (SE, 1, 1, S, E), (SW, -1, 1, S, W), (NW, -1, -1, N, W)] {
        if m & (a | b) == 0 && hit(dx, dy) {
            m |= bit;
        }
    }
    m
}

/// Distance (in px) from pixel (x, y) to the nearest intruding edge.
fn edge_distance(x: usize, y: usize, mask: u8) -> f32 {
    let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
    let mut d = f32::MAX;
    if mask & N != 0 {
        d = d.min(fy - WOBBLE[x]);
    }
    if mask & S != 0 {
        d = d.min(16.0 - fy - WOBBLE[(x + 5) % 16]);
    }
    if mask & W != 0 {
        d = d.min(fx - WOBBLE[y]);
    }
    if mask & E != 0 {
        d = d.min(16.0 - fx - WOBBLE[(y + 5) % 16]);
    }
    let corner = |cx: f32, cy: f32, w: f32| ((fx - cx).powi(2) + (fy - cy).powi(2)).sqrt() - w;
    if mask & NW != 0 {
        d = d.min(corner(0.0, 0.0, WOBBLE[(x + y) % 16]));
    }
    if mask & NE != 0 {
        d = d.min(corner(16.0, 0.0, WOBBLE[(x + y) % 16]));
    }
    if mask & SW != 0 {
        d = d.min(corner(0.0, 16.0, WOBBLE[(x + y) % 16]));
    }
    if mask & SE != 0 {
        d = d.min(corner(16.0, 16.0, WOBBLE[(x + y) % 16]));
    }
    d
}

fn compose(base: &PixelArt, f: impl Fn(usize, usize, u8) -> u8) -> PixelArt {
    let mut out = base.clone();
    for y in 0..16 {
        for x in 0..16 {
            out.set(x, y, f(x, y, base.get(x, y)));
        }
    }
    out
}

/// Generates and caches transition metatiles.
struct Autotiler<'a> {
    art: &'a TerrainArt,
    cache: HashMap<(Terrain, u8), MetatileId>,
}

/// How far the surf reaches up the beach on each animation frame.
const SURF_REACH: [f32; 4] = [1.0, 2.0, 3.0, 2.0];

impl<'a> Autotiler<'a> {
    fn tile(&mut self, bank: &mut MetatileBank, t: Terrain, mask: u8) -> MetatileId {
        if let Some(&id) = self.cache.get(&(t, mask)) {
            return id;
        }
        let art = self.art;
        let id = match t {
            Terrain::ShallowWater => {
                // Sand bank, then foam that laps back and forth.
                let frames = (0..4)
                    .map(|f| {
                        let base = &art.shallow[(f / 2) % art.shallow.len()];
                        compose(base, |x, y, p| {
                            let d = edge_distance(x, y, mask);
                            if d < 2.5 {
                                3
                            } else if d < 2.5 + SURF_REACH[f] {
                                2
                            } else {
                                p
                            }
                        })
                        .to_tile16()
                    })
                    .collect();
                bank.add(BG_SHORE, frames, SURF_PERIOD)
            }
            Terrain::DeepWater => {
                // Lighter fringe where the sea floor rises.
                let frames = art
                    .deep
                    .iter()
                    .map(|base| {
                        compose(base, |x, y, p| {
                            let d = edge_distance(x, y, mask);
                            if d < 1.5 || (d < 3.0 && (x + y) % 2 == 0) { 3 } else { p }
                        })
                        .to_tile16()
                    })
                    .collect();
                bank.add(BG_DEEP, frames, WATER_PERIOD)
            }
            Terrain::Grass => {
                let a = compose(&art.grass, |x, y, p| {
                    let d = edge_distance(x, y, mask);
                    if d < 1.5 {
                        3
                    } else if d < 2.5 {
                        1
                    } else {
                        p
                    }
                });
                bank.add_art(BG_VEG, &a)
            }
            Terrain::Sand => unreachable!("sand has no transitions"),
        };
        self.cache.insert((t, mask), id);
        id
    }
}

pub fn build_visual_map(map: &WorldMap, assets: &mut Assets, seed: u32) -> VisualMap {
    let bg = assets.bg;
    let mut autotiler = Autotiler { art: &assets.terrain_art, cache: HashMap::new() };
    let bank = &mut assets.vram.metatiles;
    let mut ids = Vec::with_capacity((map.width() * map.height()) as usize);

    for y in 0..map.height() {
        for x in 0..map.width() {
            let variant = (hash2(x, y, seed ^ 0xBEEF) % 100) as usize;
            let id = match map.feature(x, y) {
                Feature::Palm(part) => match part {
                    PalmPart::TopLeft => bg.palm[0],
                    PalmPart::TopRight => bg.palm[1],
                    PalmPart::BottomLeft => bg.palm[2],
                    PalmPart::BottomRight => bg.palm[3],
                },
                Feature::Rock => bg.rock,
                Feature::Bush => bg.bush,
                Feature::Driftwood => bg.driftwood,
                Feature::Shell => bg.shell,
                Feature::Flower => bg.flowers,
                Feature::None => {
                    let t = map.terrain(x, y);
                    let mask = edge_mask(map, x, y);
                    match t {
                        _ if mask != 0 && t != Terrain::Sand => autotiler.tile(bank, t, mask),
                        Terrain::DeepWater => bg.deep,
                        Terrain::ShallowWater => bg.shallow,
                        Terrain::Sand => bg.sand[usize::from(variant < 25)],
                        Terrain::Grass => bg.grass[usize::from(variant < 30)],
                    }
                }
            };
            ids.push(id);
        }
    }
    VisualMap { w: map.width(), h: map.height(), ids, border: bg.deep }
}

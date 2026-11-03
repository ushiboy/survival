//! Procedural generation of the southern beach area of the island.
//!
//! Layout (north → south): dense jungle, grassland, a wide beach with palms,
//! the surf line, shallows and then open sea. A cove cuts into the beach and a
//! small sandbar sits offshore.

use super::harvest::PALM_COCONUTS;
use super::map::{TILE_SIZE, WorldMap};
use super::noise::fbm;
use super::river::River;
use super::terrain::{Feature, PalmPart, Terrain};
use crate::common::rng::{Rng, hash_unit};
use std::f32::consts::TAU;

pub const MAP_W: i32 = 64;
pub const MAP_H: i32 = 48;

/// Elevation contours (in tiles above the waterline) separating terrains.
/// The renderer draws transitions along these contours per pixel.
pub const DEEP_BELOW: f32 = -5.0;
pub const SAND_BELOW: f32 = 7.0;
/// Props keep this far (in elevation) from contours so they never sit on a
/// transition.
const PROP_MARGIN: f32 = 1.2;
const JUNGLE_ABOVE: f32 = 17.0;
const CRAB_COUNT: usize = 7;

/// Height of the land relative to the shoreline, in tiles (negative = sea).
struct HeightField {
    seed: u32,
}

impl HeightField {
    fn coast_y(&self, x: f32) -> f32 {
        let t = x / MAP_W as f32 * TAU;
        MAP_H as f32 * 0.52 + 3.5 * (t + 0.6).sin() + 2.0 * (t * 2.7 + 1.9).sin()
    }

    fn at(&self, x: i32, y: i32) -> f32 {
        let (fx, fy) = (x as f32, y as f32);
        let n = fbm(fx * 0.11, fy * 0.11, self.seed, 3);
        let mut h = (self.coast_y(fx) - fy) + n * 3.0;

        // Cove cutting into the beach.
        let cove_x = MAP_W as f32 * 0.28;
        h -= bump(fx, fy, cove_x, self.coast_y(cove_x) - 2.0, 6.0) * 7.0;
        // Offshore sandbar.
        let bar_x = MAP_W as f32 * 0.72;
        h += bump(fx, fy, bar_x, self.coast_y(bar_x) + 9.0, 5.0) * 13.0;
        h
    }
}

impl HeightField {
    /// The river flows in from beyond the north edge and into the cove.
    fn river(&self) -> River {
        let cove_x = MAP_W as f32 * 0.28;
        let source_x = MAP_W as f32 * 0.38 + hash_unit(3, 4, self.seed ^ 0x5B1C) * 4.0;
        River::new((source_x, -4.0), (cove_x, self.coast_y(cove_x) + 1.0), self.seed)
    }
}

/// Marks the river's cells (impassable water) and the cells along it.
fn lay_river(map: &mut WorldMap, river: &River) {
    let ts = TILE_SIZE as f32;
    for y in 0..MAP_H {
        for x in 0..MAP_W {
            let depth = river.depth_px((x as f32 + 0.5) * ts, (y as f32 + 0.5) * ts);
            // Anything within half a cell's diagonal (+ the bank lip) of the water.
            map.set_riverside(x, y, depth > -24.0);
            if depth > -2.0 && map.terrain(x, y).is_land() {
                map.set_terrain(x, y, Terrain::River);
            }
        }
    }
}

/// Picks a bank of the upper river, in the open grassland (the jungle is too
/// thick to reach the water), where water can be drawn.
fn water_spot(map: &WorldMap, river: &River, heights: &[f32]) -> Option<(i32, i32)> {
    let ts = TILE_SIZE as f32;
    for i in 0..60 {
        let t = 0.1 + i as f32 * 0.01;
        let ((cx, cy), (nx, ny)) = river.point(t);
        for side in [1.0, -1.0] {
            let off = River::half_width(t) + 0.45;
            let (bx, by) = (cx + nx * side * off, cy + ny * side * off);
            let (gx, gy) = (bx.round() as i32, by.round() as i32);
            if !map.in_bounds(gx, gy) || !map.terrain(gx, gy).is_land() {
                continue;
            }
            let h = heights[(gy * MAP_W + gx) as usize];
            if (SAND_BELOW + 1.0..JUNGLE_ABOVE - 1.0).contains(&h) {
                return Some((((bx + 0.5) * ts) as i32, ((by + 0.5) * ts) as i32));
            }
        }
    }
    None
}

fn bump(x: f32, y: f32, cx: f32, cy: f32, r: f32) -> f32 {
    let d = ((x - cx).powi(2) + (y - cy).powi(2)).sqrt();
    (1.0 - d / r).max(0.0)
}

fn classify(h: f32) -> Terrain {
    if h < DEEP_BELOW {
        Terrain::DeepWater
    } else if h < 0.0 {
        Terrain::ShallowWater
    } else if h < SAND_BELOW {
        Terrain::Sand
    } else {
        Terrain::Grass
    }
}

pub fn generate(seed: u32) -> WorldMap {
    let field = HeightField { seed };
    let river = field.river();
    let mut map = WorldMap::new(MAP_W, MAP_H, Terrain::DeepWater);
    let mut heights = vec![0.0f32; (MAP_W * MAP_H) as usize];
    for y in 0..MAP_H {
        for x in 0..MAP_W {
            let h = field.at(x, y);
            heights[(y * MAP_W + x) as usize] = h;
            map.set_terrain(x, y, classify(h));
        }
    }
    enforce_transitions(&mut map);
    // Cells turned to sand above keep sand-level elevation, so the grass
    // fringe is drawn around them instead of a square edge.
    for y in 0..MAP_H {
        for x in 0..MAP_W {
            let h = &mut heights[(y * MAP_W + x) as usize];
            if map.terrain(x, y) == Terrain::Sand && *h >= SAND_BELOW {
                *h = SAND_BELOW - 0.8;
            }
        }
    }
    lay_river(&mut map, &river);
    map.water_spot = water_spot(&map, &river, &heights);
    map.river = Some(river);

    let spawn_cell = find_spawn(&map, &heights);
    map.spawn = (spawn_cell.0 * TILE_SIZE, spawn_cell.1 * TILE_SIZE);
    let near_spawn = |x: i32, y: i32| (x - spawn_cell.0).abs() <= 3 && (y - spawn_cell.1).abs() <= 3;

    place_palms(&mut map, &heights, seed, &near_spawn);
    place_scatter(&mut map, &heights, seed, &near_spawn);
    map.set_elevation(heights);
    map.crab_spawns = pick_crab_spawns(&map, seed, spawn_cell);
    map
}

/// Guarantees the sea → shallows → sand → grass ordering so every edge has a
/// matching transition tile.
fn enforce_transitions(map: &mut WorldMap) {
    let snapshot = map.clone();
    let neighbours = |x: i32, y: i32, pred: &dyn Fn(Terrain) -> bool| {
        (-1..=1).any(|dy| (-1..=1).any(|dx| (dx, dy) != (0, 0) && snapshot.in_bounds(x + dx, y + dy) && pred(snapshot.terrain(x + dx, y + dy))))
    };
    for y in 0..map.height() {
        for x in 0..map.width() {
            match snapshot.terrain(x, y) {
                Terrain::DeepWater if neighbours(x, y, &|t| t.is_land()) => {
                    map.set_terrain(x, y, Terrain::ShallowWater)
                }
                Terrain::Grass if neighbours(x, y, &|t| t.is_water()) => map.set_terrain(x, y, Terrain::Sand),
                _ => {}
            }
        }
    }
}

/// Lower beach near the middle of the map, with open space around it.
fn find_spawn(map: &WorldMap, heights: &[f32]) -> (i32, i32) {
    let cx = MAP_W / 2;
    for dx in 0..MAP_W / 2 {
        for x in [cx + dx, cx - dx] {
            for y in 0..MAP_H {
                let h = heights[(y * MAP_W + x) as usize];
                let clear = (-1..=1).all(|oy| (-1..=1).all(|ox| map.terrain(x + ox, y + oy) == Terrain::Sand));
                if (1.5..3.5).contains(&h) && clear {
                    return (x, y);
                }
            }
        }
    }
    (cx, MAP_H / 2)
}

fn place_palms(map: &mut WorldMap, heights: &[f32], seed: u32, near_spawn: &dyn Fn(i32, i32) -> bool) {
    let (pw, ph) = (PalmPart::W, PalmPart::H);
    for y in 0..MAP_H - ph + 1 {
        for x in 0..MAP_W - pw + 1 {
            // Anchor on the trunk base (bottom row), which sits on the upper beach.
            let h = heights[((y + ph - 1) * MAP_W + x) as usize];
            if !(2.0..SAND_BELOW).contains(&h) || hash_unit(x, y, seed ^ 0xFA1A) > 0.16 {
                continue;
            }
            // The trunk base and canopy-shadow rows sit on clean sand; the
            // canopy rows above only need dry, empty ground to overhang.
            let fits = (0..ph).all(|dy| {
                (0..pw).all(|dx| {
                    let (cx, cy) = (x + dx, y + dy);
                    let h = heights[(cy * MAP_W + cx) as usize];
                    let ground_ok = if dy >= ph - 2 {
                        (PROP_MARGIN..SAND_BELOW - PROP_MARGIN).contains(&h) && map.terrain(cx, cy) == Terrain::Sand
                    } else {
                        map.terrain(cx, cy).is_land()
                    };
                    ground_ok && map.feature(cx, cy) == Feature::None && !near_spawn(cx, cy) && !map.is_riverside(cx, cy)
                })
            });
            // Keep a gap between trees.
            let crowded = (-1..=ph).any(|dy| (-1..=pw).any(|dx| matches!(map.feature(x + dx, y + dy), Feature::Palm(_))));
            if fits && !crowded {
                let flip = hash_unit(x, y, seed ^ 0xF11B) < 0.5;
                // Most palms carry a full bunch; a few have little or none.
                let r = hash_unit(x, y, seed ^ 0xC0C0);
                let coconuts = if r < 0.08 { 0 } else if r < 0.2 { 1 } else if r < 0.5 { 2 } else { PALM_COCONUTS };
                map.set_fruit((x, y), coconuts);
                for dy in 0..ph {
                    for dx in 0..pw {
                        map.set_feature(x + dx, y + dy, Feature::Palm(PalmPart { dx: dx as u8, dy: dy as u8, flip }));
                    }
                }
            }
        }
    }
}

fn place_scatter(map: &mut WorldMap, heights: &[f32], seed: u32, near_spawn: &dyn Fn(i32, i32) -> bool) {
    for y in 0..MAP_H {
        for x in 0..MAP_W {
            if map.feature(x, y) != Feature::None || near_spawn(x, y) || map.is_riverside(x, y) {
                continue;
            }
            let h = heights[(y * MAP_W + x) as usize];
            let r = hash_unit(x, y, seed ^ 0xF00D);
            let clear_of_edges = h > SAND_BELOW + PROP_MARGIN || (PROP_MARGIN..SAND_BELOW - PROP_MARGIN).contains(&h);
            if !clear_of_edges {
                continue;
            }
            let feature = match map.terrain(x, y) {
                Terrain::Grass if h > JUNGLE_ABOVE => {
                    if r < 0.6 { Feature::Bush } else if r < 0.66 { Feature::Flower } else { Feature::None }
                }
                Terrain::Grass => {
                    if r < 0.05 { Feature::Bush } else if r < 0.12 { Feature::Flower } else { Feature::None }
                }
                Terrain::Sand => {
                    if r < 0.016 {
                        Feature::Rock
                    } else if r < 0.028 {
                        Feature::Pebbles
                    } else if r < 0.038 {
                        Feature::Driftwood
                    } else if r < 0.062 {
                        Feature::Shell
                    } else {
                        Feature::None
                    }
                }
                _ => Feature::None,
            };
            map.set_feature(x, y, feature);
        }
    }
}

fn pick_crab_spawns(map: &WorldMap, seed: u32, spawn: (i32, i32)) -> Vec<(i32, i32)> {
    let mut rng = Rng::new(seed ^ 0xC4AB);
    let mut out = Vec::new();
    for _ in 0..2000 {
        if out.len() >= CRAB_COUNT {
            break;
        }
        let (x, y) = (rng.between(0, MAP_W - 1), rng.between(0, MAP_H - 1));
        let ok = map.terrain(x, y) == Terrain::Sand
            && !map.props(x, y).solid
            && ((x - spawn.0).abs() > 4 || (y - spawn.1).abs() > 4);
        if ok {
            out.push((x * TILE_SIZE, y * TILE_SIZE));
        }
    }
    out
}

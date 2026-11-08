//! Dev tool: draws the whole island as a map, a few pixels per cell, to see
//! the plan's shape at a glance. Writes a PPM.
//!
//! Usage: cargo run --release --example island_map -- out.ppm [px-per-cell]
//!
//! Colours: deep sea, shallows, beach, grassland and jungle (darker green),
//! the river (bright blue), the high ground (olive), its edge (dark olive)
//! and cliff faces (grey); trees dark green, bushes and vines mid green,
//! boulders and driftwood grey and brown, shells pink; the landing white,
//! the water-drawing spot cyan, the narrow of the river red and the foot of
//! the low cliff orange. Ground the castaway can't walk to from the landing
//! is drawn darker.
//!
//! It also walks the island from the landing and reports what can be
//! reached: the edges of the walkable ground, and whether the water-drawing
//! spot, the foot of the low cliff and the near bank at the narrow are in
//! reach (and the far bank is not).

use std::collections::VecDeque;

use survival_core::world::{Feature, TILE_SIZE, Terrain, WorldMap, island};

/// Cells the castaway can walk to from `start` (moving cell to cell through
/// ground that isn't solid; diagonal steps need both sides open).
fn reachable(map: &WorldMap, start: (i32, i32)) -> Vec<bool> {
    let (w, h) = (map.width(), map.height());
    let open = |x: i32, y: i32| map.in_bounds(x, y) && !map.props(x, y).solid;
    let mut seen = vec![false; (w * h) as usize];
    let mut queue = VecDeque::from([start]);
    seen[(start.1 * w + start.0) as usize] = true;
    while let Some((x, y)) = queue.pop_front() {
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (1, -1), (-1, 1), (-1, -1)] {
            let (nx, ny) = (x + dx, y + dy);
            let diagonal_ok = dx == 0 || dy == 0 || (open(x + dx, y) && open(x, y + dy));
            if open(nx, ny) && diagonal_ok && !seen[(ny * w + nx) as usize] {
                seen[(ny * w + nx) as usize] = true;
                queue.push_back((nx, ny));
            }
        }
    }
    seen
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let out = args.get(1).cloned().unwrap_or_else(|| "island.ppm".into());
    let scale: i32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(4);
    let map = island::generate(1);
    let (w, h) = (map.width(), map.height());
    let spawn = (map.spawn.0 / TILE_SIZE, map.spawn.1 / TILE_SIZE + 1);
    let water = map.water_spot.map(|(x, y)| (x / TILE_SIZE, y / TILE_SIZE));
    let spots = map.spots;
    let reach = reachable(&map, spawn);
    let can = |c: (i32, i32)| map.in_bounds(c.0, c.1) && reach[(c.1 * w + c.0) as usize];
    let mut data = format!("P6\n{} {}\n255\n", w * scale, h * scale).into_bytes();
    for py in 0..h * scale {
        for px in 0..w * scale {
            let (x, y) = (px / scale, py / scale);
            let e = map.elevation(x, y);
            let mut c: [u8; 3] = match map.terrain(x, y) {
                Terrain::DeepWater => [20, 50, 120],
                Terrain::ShallowWater => [60, 140, 200],
                Terrain::River => [40, 110, 230],
                Terrain::Sand => [230, 210, 150],
                Terrain::Grass if e > 17.0 => [40, 110, 50],
                Terrain::Grass => [100, 170, 70],
                Terrain::Highland => [120, 130, 60],
                Terrain::CliffTop => [80, 84, 40],
                Terrain::Cliff => [130, 126, 130],
            };
            c = match map.feature(x, y) {
                Feature::Palm(_) | Feature::Broadleaf(_) => [20, 70, 30],
                Feature::Bush | Feature::Vine => [60, 130, 50],
                Feature::Rock => [120, 120, 120],
                Feature::Driftwood(_) => [140, 100, 60],
                Feature::Shell => [240, 160, 170],
                _ => c,
            };
            let walkable = !map.props(x, y).solid || matches!(map.feature(x, y), Feature::Palm(_) | Feature::Broadleaf(_));
            if walkable && map.terrain(x, y).is_land() && !reach[(y * w + x) as usize] {
                c = c.map(|v| (v as u32 * 3 / 5) as u8);
            }
            if (x, y) == spawn {
                c = [255, 255, 255];
            }
            if Some((x, y)) == water {
                c = [0, 255, 255];
            }
            if Some((x, y)) == spots.bridge {
                c = [255, 40, 40];
            }
            if Some((x, y)) == spots.ladder {
                c = [255, 150, 0];
            }
            data.extend_from_slice(&c);
        }
    }
    std::fs::write(&out, data).expect("write map");
    println!("{w}x{h} cells, landing {spawn:?}, water {water:?}, bridge {:?}, ladder {:?}", spots.bridge, spots.ladder);

    let cells: Vec<(i32, i32)> = (0..h).flat_map(|y| (0..w).map(move |x| (x, y))).filter(|&(x, y)| reach[(y * w + x) as usize]).collect();
    let (x0, x1) = (cells.iter().map(|c| c.0).min().unwrap(), cells.iter().map(|c| c.0).max().unwrap());
    let (y0, y1) = (cells.iter().map(|c| c.1).min().unwrap(), cells.iter().map(|c| c.1).max().unwrap());
    println!("walkable from the landing: {} cells, x {x0}..={x1}, y {y0}..={y1}", cells.len());
    let high = cells.iter().any(|&(x, y)| map.terrain(x, y) == Terrain::Highland);
    println!("  high ground reached: {high}");
    // The water is drawn from beside the spot (within reach of it).
    let near = |c: (i32, i32)| (-1..=1).any(|dy| (-1..=1).any(|dx| can((c.0 + dx, c.1 + dy))));
    println!("  water spot in reach: {}", water.is_some_and(near));
    println!("  foot of the low cliff in reach: {}", spots.ladder.is_some_and(can));
    if let (Some((bx, by)), Some(river)) = (spots.bridge, map.river.as_ref()) {
        // Look across the river at the narrow, both ways.
        let p = river.nearest(bx as f32, by as f32);
        let banks: Vec<bool> = (-1..=1)
            .step_by(2)
            .map(|side| {
                (1..8).map(|k| (bx + side * k, by)).find(|&(x, y)| map.terrain(x, y) != Terrain::River).is_some_and(near)
            })
            .collect();
        println!("  banks at the narrow in reach (west, east): {:?} (channel half-width {:.2})", banks, p.half);
    }
}

//! Dev tool: draws the whole island as a map, a few pixels per cell, to see
//! the plan's shape at a glance. Writes a PPM.
//!
//! Usage: cargo run --release --example island_map -- out.ppm [px-per-cell]
//!
//! Colours: deep sea, shallows, beach, grassland and jungle (darker green),
//! the river (bright blue); trees dark green, bushes and vines mid green,
//! boulders and driftwood grey and brown, shells pink; the landing white and
//! the water-drawing spot cyan.

use survival_core::world::{Feature, TILE_SIZE, Terrain, island};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let out = args.get(1).cloned().unwrap_or_else(|| "island.ppm".into());
    let scale: i32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(4);
    let map = island::generate(1);
    let (w, h) = (map.width(), map.height());
    let spawn = (map.spawn.0 / TILE_SIZE, map.spawn.1 / TILE_SIZE + 1);
    let water = map.water_spot.map(|(x, y)| (x / TILE_SIZE, y / TILE_SIZE));
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
            };
            c = match map.feature(x, y) {
                Feature::Palm(_) | Feature::Broadleaf(_) => [20, 70, 30],
                Feature::Bush | Feature::Vine => [60, 130, 50],
                Feature::Rock => [120, 120, 120],
                Feature::Driftwood(_) => [140, 100, 60],
                Feature::Shell => [240, 160, 170],
                _ => c,
            };
            if (x, y) == spawn {
                c = [255, 255, 255];
            }
            if Some((x, y)) == water {
                c = [0, 255, 255];
            }
            data.extend_from_slice(&c);
        }
    }
    std::fs::write(&out, data).expect("write map");
    println!("{w}x{h} cells, landing {spawn:?}, water {water:?}");
}

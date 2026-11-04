//! Dev tool: lists the water-drawing spot, and the palms (with their
//! coconuts) and bushes nearest the spawn point.
//! Usage: cargo run --release --example palms -- [seed]
use survival_core::world::terrain::PalmPart;
use survival_core::world::{Feature, TILE_SIZE, island};

fn main() {
    let seed = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(1);
    let map = island::generate(seed);
    let spawn = (map.spawn.0 / TILE_SIZE, map.spawn.1 / TILE_SIZE);
    println!("spawn cell {spawn:?}");
    if let Some((sx, sy)) = map.water_spot {
        println!("water-drawing spot cell {:?}", (sx / TILE_SIZE, sy / TILE_SIZE));
    }
    let mut palms = Vec::new();
    for y in 0..map.height() {
        for x in 0..map.width() {
            if let Feature::Palm(p @ PalmPart { dx: 0, dy: 0, .. }) = map.feature(x, y) {
                let base = (x + PalmPart::W / 2, y + PalmPart::H - 1);
                let d = (base.0 - spawn.0).abs() + (base.1 - spawn.1).abs();
                palms.push((d, base, map.fruit((x, y)), p.flip));
            }
        }
    }
    palms.sort();
    for (d, base, fruit, flip) in palms.iter().take(8) {
        println!("trunk base {base:?} dist {d} coconuts {fruit} flip {flip}");
    }
    let mut bushes: Vec<_> = (0..map.height())
        .flat_map(|y| (0..map.width()).map(move |x| (x, y)))
        .filter(|&(x, y)| map.feature(x, y) == Feature::Bush)
        .map(|(x, y)| ((x - spawn.0).abs() + (y - spawn.1).abs(), (x, y)))
        .collect();
    bushes.sort();
    for (d, cell) in bushes.iter().take(4) {
        println!("bush {cell:?} dist {d}");
    }
}

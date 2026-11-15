//! Dev check: builds a map many chunks wide (the island tiled over and
//! over), sweeps the screen across it while the ground streams in and out,
//! and checks the tiles on screen against ground built fresh for the same
//! view, pixel for pixel. Also reports the blocks held as it goes.
//!
//! Usage: cargo run --release --example stream_check

use survival_core::assets::Assets;
use survival_core::common::math::Rect;
use survival_core::ppu::{BgCell, BgLayer, SCREEN_H, SCREEN_W};
use survival_core::render::tileset::Tileset;
use survival_core::world::{TILE_SIZE, WorldMap, island};

const REPEAT: (i32, i32) = (5, 5);

fn big_map() -> WorldMap {
    let island = island::generate(1);
    let (w, h) = (island.width(), island.height());
    let mut map = WorldMap::new(w * REPEAT.0, h * REPEAT.1, survival_core::world::Terrain::DeepWater);
    let mut elevation = Vec::with_capacity((map.width() * map.height()) as usize);
    for y in 0..map.height() {
        for x in 0..map.width() {
            let (sx, sy) = (x % w, y % h);
            map.set_terrain(x, y, island.terrain(sx, sy));
            elevation.push(island.elevation(sx, sy));
            // Trees span several cells: keep only those wholly inside a copy.
            let f = island.feature(sx, sy);
            let near_edge = sx < 4 || sy < 4 || sx >= w - 4 || sy >= h - 4;
            if !near_edge {
                map.set_feature(x, y, f);
            }
        }
    }
    map.set_elevation(elevation);
    map
}

fn same(a: &Assets, ca: BgCell, b: &Assets, cb: BgCell) -> bool {
    if ca.is_empty() || cb.is_empty() {
        return ca.is_empty() == cb.is_empty();
    }
    let (ta, pa) = a.vram.metatiles.frame(ca.tile, 0);
    let (tb, pb) = b.vram.metatiles.frame(cb.tile, 0);
    ta == tb && pa == pb && ca.hflip == cb.hflip && ca.high == cb.high
}

fn main() {
    let map = big_map();
    let mut assets = Assets::load();
    let (mut tileset, mut visual) = Tileset::new(&map, &mut assets);
    let (pw, ph) = map.pixel_size();
    let (sw, sh) = (SCREEN_W as i32, SCREEN_H as i32);
    // A zig-zag across the whole map, 6 px a frame.
    let mut path = Vec::new();
    let mut y = 0;
    while y <= ph - sh {
        let xs: Vec<i32> = (0..=(pw - sw)).step_by(6).collect();
        if (y / 200) % 2 == 0 {
            path.extend(xs.iter().map(|&x| (x, y)))
        } else {
            path.extend(xs.iter().rev().map(|&x| (x, y)))
        }
        y += 200;
    }
    let mut worst = 0;
    let mut checked = 0;
    for (n, &(x, y)) in path.iter().enumerate() {
        let view = Rect::new(x, y, sw, sh);
        tileset.stream(&map, &mut assets, &mut visual, view);
        worst = worst.max(tileset.resident());
        if n % 97 != 0 {
            continue;
        }
        let mut fresh_assets = Assets::load();
        let (mut fresh, mut fresh_visual) = Tileset::new(&map, &mut fresh_assets);
        fresh.stream(&map, &mut fresh_assets, &mut fresh_visual, view);
        for cy in y.div_euclid(TILE_SIZE)..=(y + sh).div_euclid(TILE_SIZE) {
            for cx in x.div_euclid(TILE_SIZE)..=(x + sw).div_euclid(TILE_SIZE) {
                for (l, (a, b)) in
                    [(visual.ground.cell(cx, cy), fresh_visual.ground.cell(cx, cy)), (visual.overhead.cell(cx, cy), fresh_visual.overhead.cell(cx, cy))]
                        .into_iter()
                        .enumerate()
                {
                    assert!(same(&assets, a, &fresh_assets, b), "layer {l} cell ({cx},{cy}) differs at view ({x},{y})");
                    assert!(l == 1 || !a.is_empty(), "ground cell ({cx},{cy}) not built at view ({x},{y})");
                }
            }
        }
        checked += 1;
    }
    println!(
        "map {}x{} cells, {} views swept, {} checked; ground blocks held: at most {}, at the end {}",
        map.width(),
        map.height(),
        path.len(),
        checked,
        worst,
        tileset.resident()
    );
}

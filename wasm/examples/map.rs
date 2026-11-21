//! Dev tool: maps of the island, for working on its plan
//! (`wasm/src/world/island_plan.txt`) and the places in it.
//!
//! Usage (from the repository): `pnpm map -- [options]`, or
//! `cargo run --profile tools --example map -- [options]` here. Options:
//!
//! - `--quick`: the plan map only (skips the views drawn by the game).
//! - `--area X0,Y0,X1,Y1`: the view covers only these cells (inclusive).
//! - `--shrink N`: the view is shrunk, N×N pixels averaged into one.
//! - `--scale N`: the plan map's pixels per cell (default 6).
//! - `--bridge`: a log bridge is laid across the river's narrow first.
//! - `--wheel`: a water wheel is set in the river at the mill site first.
//! - `--lathe`: a water wheel and the wood lathe beside it are set at the
//!   mill site first.
//! - `--stakes`: the water wheel, the wood lathe and the marking stakes
//!   round the mill's ground are set first.
//! - `--foundation`: all that, and the mill's foundation laid.
//! - `--frame`: and its frame raised on it too.
//! - `--walls`: and its walls put up round the frame.
//! - `--roof`: and its roof put on, its door hung (the bench cleared).
//! - `--set-base`: keeps this run as the base later runs are compared
//!   with; `--clear-base` goes back to comparing with the run before.
//! - `--out DIR`: where to write (default `wasm/target/map`).
//!
//! It writes:
//!
//! - `plan.png`: the island a few pixels a cell, coloured by ground and what
//!   stands on it, over the plan's blocks and cell coordinates, with the
//!   special places marked. Ground the castaway can't walk to from the
//!   landing is drawn darker.
//! - `view.png`: the island as the game draws it (no HUD, castaway or
//!   creatures; at midday), 16 px a cell, with cell coordinates.
//! - `spots.png`: a screen of each special place, as the game draws it.
//! - `plan-diff.png` / `view-diff.png`: what changed since the base (or the
//!   last run), in colour over the rest greyed out, the cells boxed. If the
//!   island changed size, the earlier one is lined up where it matches
//!   best; cells it didn't have (and cells the earlier view didn't cover)
//!   are boxed in green.
//!
//! It also prints what can be reached from the landing, how many branches,
//! stones, crabs, fish and shells there are (they grow with the island),
//! and what changed.

#[path = "map_parts/image.rs"]
mod image;

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::time::Instant;

use image::{Image, Rgb, text_width};
use survival_core::game::Game;
use survival_core::ppu::{SCREEN_H, SCREEN_W};
use survival_core::world::island::PLAN_BLOCK;
use survival_core::world::terrain::Soil;
use survival_core::world::{CHUNK_CELLS, Feature, TILE_SIZE, Terrain, WorldMap};

const CHANGED: Rgb = [255, 0, 255];
/// Cells the earlier run didn't have (the island grew, or the view
/// covers more).
const ADDED: Rgb = [0, 230, 120];
const AXIS_BG: Rgb = [24, 24, 32];
const LABEL: Rgb = [235, 235, 235];
/// Font pixel size of labels.
const FONT: i32 = 2;
/// Room for the coordinates along the top and the left of a map.
const MARGIN_TOP: i32 = 14;
const MARGIN_LEFT: i32 = 30;

struct Options {
    out: PathBuf,
    quick: bool,
    area: Option<(i32, i32, i32, i32)>,
    shrink: i32,
    scale: i32,
    set_base: bool,
    clear_base: bool,
    bridge: bool,
    wheel: bool,
    lathe: bool,
    stakes: bool,
    /// How many of the mill's stages are built first.
    stages: usize,
}

fn options() -> Options {
    let mut o = Options {
        out: Path::new(env!("CARGO_MANIFEST_DIR")).join("target/map"),
        quick: false,
        area: None,
        shrink: 1,
        scale: 6,
        set_base: false,
        clear_base: false,
        bridge: false,
        wheel: false,
        lathe: false,
        stakes: false,
        stages: 0,
    };
    let mut args = std::env::args().skip(1);
    let number = |v: Option<String>, what: &str| -> i32 { v.and_then(|v| v.parse().ok()).unwrap_or_else(|| panic!("{what} needs a number")) };
    while let Some(a) = args.next() {
        match a.as_str() {
            "--quick" => o.quick = true,
            "--bridge" => o.bridge = true,
            "--wheel" => o.wheel = true,
            "--lathe" => {
                o.wheel = true;
                o.lathe = true;
            }
            "--stakes" => {
                o.wheel = true;
                o.lathe = true;
                o.stakes = true;
            }
            "--foundation" => {
                o.wheel = true;
                o.lathe = true;
                o.stakes = true;
                o.stages = 1;
            }
            "--frame" => {
                o.wheel = true;
                o.lathe = true;
                o.stakes = true;
                o.stages = 2;
            }
            "--walls" => {
                o.wheel = true;
                o.lathe = true;
                o.stakes = true;
                o.stages = 3;
            }
            "--roof" => {
                o.wheel = true;
                o.lathe = true;
                o.stakes = true;
                o.stages = 4;
            }
            "--set-base" => o.set_base = true,
            "--clear-base" => o.clear_base = true,
            "--out" => o.out = PathBuf::from(args.next().expect("--out needs a directory")),
            "--shrink" => o.shrink = number(args.next(), "--shrink").max(1),
            "--scale" => o.scale = number(args.next(), "--scale").max(2),
            "--area" => {
                let v: Vec<i32> = args.next().expect("--area needs X0,Y0,X1,Y1").split(',').map(|n| n.trim().parse().expect("--area takes cells")).collect();
                assert!(v.len() == 4, "--area needs X0,Y0,X1,Y1");
                o.area = Some((v[0].min(v[2]), v[1].min(v[3]), v[0].max(v[2]), v[1].max(v[3])));
            }
            "--" => {}
            other => panic!("unknown option {other} (see the top of wasm/examples/map.rs)"),
        }
    }
    o
}

/// The special places, with a letter each for the plan map.
fn spots(map: &WorldMap) -> Vec<(&'static str, char, Rgb, (i32, i32))> {
    let cell = |(x, y): (i32, i32)| (x / TILE_SIZE, y / TILE_SIZE);
    let mut v = vec![("LANDING", 'L', [255, 255, 255], (map.spawn.0 / TILE_SIZE, map.spawn.1 / TILE_SIZE + 1))];
    for (i, &w) in map.water_spots.iter().enumerate() {
        // The first is on the pool's east bank, then one up on the high
        // ground, and one on the pool's west bank, beyond the river.
        let name = match i {
            0 => "WATER SPOT",
            _ if map.is_high(w.0 / TILE_SIZE, w.1 / TILE_SIZE) => "UPPER WATER SPOT",
            _ => "WEST WATER SPOT",
        };
        v.push((name, 'W', [0, 255, 255], cell(w)));
    }
    if let Some(f) = map.spots.falls {
        v.push(("FALLS POOL", 'F', [120, 200, 255], f));
    }
    if let Some(m) = map.spots.mill {
        v.push(("MILL SITE", 'M', [200, 120, 255], m));
    }
    if let Some(b) = map.spots.bridge {
        v.push(("RIVER NARROW", 'N', [255, 60, 60], b));
    }
    if let Some(s) = map.spots.slope {
        v.push(("WEST SLOPE", 'S', [255, 220, 120], s));
    }
    if let Some(l) = map.spots.ladder {
        v.push(("LOW CLIFF", 'C', [255, 160, 0], l));
    }
    v
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

    let t = Instant::now();
    let mut game = Game::new(1);
    game.skip_title();
    if o.wheel && !game.debug_set_wheel() {
        println!("no water wheel goes in at the mill site");
    }
    if o.lathe && !game.debug_set_lathe() {
        println!("no wood lathe goes in beside the water wheel");
    }
    if o.stakes && !game.debug_set_stakes() {
        println!("the mill's ground isn't marked out");
    }
    if o.stages > 0 && !game.debug_build_mill(o.stages) {
        println!("the mill's stages aren't built");
    }
    if o.bridge && !game.debug_lay_bridge() {
        println!("no bridge goes across the narrow");
    }
    let map = game.map().clone();
    let spots = spots(&map);

    // The plan map.
    let reach = reachable(&map, spots[0].3);
    report_reach(&map, &reach, &spots);
    report_counts(&game);
    let cells = cell_signature(&map);
    let old_cells = std::fs::read_to_string(against.join("plan.cells")).ok();
    let old_plan = Image::load_png(&against.join("plan.png"));
    let plan = plan_map(&map, &reach, &spots, o.scale, o.area);
    plan.save_png(&o.out.join("plan.png"));
    std::fs::write(o.out.join("plan.cells"), &cells).expect("write plan.cells");
    let plan_diff = o.out.join("plan-diff.png");
    let diff = old_cells.as_deref().map(|old| diff_cells(old, &cells));
    if let Some(d) = &diff {
        if d.old_size != (map.width(), map.island_size().1) {
            println!(
                "plan: the island was {}x{} cells in {against_name}; its cells are compared {},{} cells over ({} new, {} gone)",
                d.old_size.0,
                d.old_size.1,
                d.shift.0,
                d.shift.1,
                d.added.len(),
                d.removed
            );
        }
        if d.changed.is_empty() && d.added.is_empty() {
            println!("plan: no cell changed since {against_name}");
            let _ = std::fs::remove_file(&plan_diff);
        } else {
            describe("plan", &d.changed, against_name);
            let s = o.scale;
            let size = (map.width() * s, map.island_size().1 * s);
            // The earlier plan, moved to where its cells are now.
            match old_plan.filter(|p| p.w == d.old_size.0 * s + MARGIN_LEFT) {
                Some(old) => {
                    let old = align(&old, &plan, (d.old_size.0 * s, d.old_size.1 * s), (d.shift.0 * s, d.shift.1 * s), size);
                    before_after(&old, &plan, &d.changed, &d.added, s as f32, (0, 0), size)
                }
                None => highlight(&plan, &d.changed, &d.added, s as f32, (0, 0), (MARGIN_LEFT, MARGIN_TOP), size),
            }
            .save_png(&plan_diff);
        }
    }
    println!("wrote plan.png ({} ms)", t.elapsed().as_millis());

    if !o.quick {
        // The view drawn by the game, and a screen of each special place.
        let t = Instant::now();
        let area = o.area.unwrap_or((0, 0, map.width() - 1, map.island_size().1 - 1));
        let area = (area.0.max(0), area.1.max(0), area.2.min(map.width() - 1), area.3.min(map.island_size().1 - 1));
        let meta = format!("area {},{},{},{} shrink {}\n", area.0, area.1, area.2, area.3, o.shrink);
        let old_meta = std::fs::read_to_string(against.join("view.txt")).ok();
        let old_view = old_meta
            .as_deref()
            .and_then(parse_view_meta)
            .filter(|&(_, shrink)| shrink == o.shrink)
            .and_then(|(a, _)| Image::load_png(&against.join("view.png")).map(|img| (a, img)));
        let raw = draw_view(&mut game, area).shrink(o.shrink);
        let cell_px = TILE_SIZE as f32 / o.shrink as f32;
        let view = with_axes(&raw, cell_px, (area.0, area.1), label_step(cell_px));
        view.save_png(&o.out.join("view.png"));
        std::fs::write(o.out.join("view.txt"), &meta).expect("write view.txt");
        let view_diff = o.out.join("view-diff.png");
        // The earlier view's cells, where they are now (the island may
        // have grown since), and how far its pixels move to get there.
        let shift = diff.as_ref().map_or((0, 0), |d| d.shift);
        let lined_up = old_view.and_then(|(a, old)| {
            let a = (a.0 + shift.0, a.1 + shift.1, a.2 + shift.0, a.3 + shift.1);
            let (px, py) = ((a.0 - area.0) * TILE_SIZE, (a.1 - area.1) * TILE_SIZE);
            let overlaps = a.0 <= area.2 && area.0 <= a.2 && a.1 <= area.3 && area.1 <= a.3;
            (overlaps && px % o.shrink == 0 && py % o.shrink == 0).then(|| {
                let old = align(&old, &view, (old.w - MARGIN_LEFT, old.h - MARGIN_TOP), (px / o.shrink, py / o.shrink), (raw.w, raw.h));
                (a, old)
            })
        });
        match lined_up {
            Some((a, old)) => {
                let covered = |(x, y): (i32, i32)| (a.0..=a.2).contains(&x) && (a.1..=a.3).contains(&y);
                let added: Vec<_> = (area.1..=area.3).flat_map(|y| (area.0..=area.2).map(move |x| (x, y))).filter(|&c| !covered(c)).collect();
                let changed = changed_view_cells(&old, &view, cell_px, (area.0, area.1), &covered);
                if !added.is_empty() {
                    println!("view: {} cells weren't in {against_name}'s view", added.len());
                }
                if changed.is_empty() && added.is_empty() {
                    println!("view: nothing changed since {against_name}");
                    let _ = std::fs::remove_file(&view_diff);
                } else {
                    describe("view", &changed, against_name);
                    before_after(&old, &view, &changed, &added, cell_px, (area.0, area.1), (raw.w, raw.h)).save_png(&view_diff);
                }
            }
            None => {
                if old_meta.is_some() {
                    println!("view: not compared ({against_name} drew another part of the island, or at another size)");
                }
                let _ = std::fs::remove_file(&view_diff);
            }
        }
        spot_sheet(&mut game, &spots).save_png(&o.out.join("spots.png"));
        println!("wrote view.png and spots.png ({} ms)", t.elapsed().as_millis());
    }

    if o.set_base {
        std::fs::create_dir_all(&base).expect("make base/");
        for f in ["plan.cells", "plan.png", "view.png", "view.txt"] {
            let _ = std::fs::copy(o.out.join(f), base.join(f));
        }
        println!("kept this run as the base");
    }
    println!("in {}", o.out.display());
}

// --- How much there is ---------------------------------------------------

/// Prints how many of the things whose number grows with the island (see
/// `island::for_area`) there are on it, for keeping docs/README.md's
/// counts right.
fn report_counts(game: &Game) {
    let save: serde_json::Value = serde_json::from_str(&game.debug_save_json()).expect("the save is JSON");
    let island = &save["island"];
    // Lying below the cliffs (`false`) or up on the high ground (`true`).
    let lying = |kind: &str, high: bool| {
        island["items"].as_array().map_or(0, |v| {
            v.iter()
                .filter(|i| i["kind"] == kind)
                .filter(|i| {
                    let (x, y) = (i["at"][0].as_i64().unwrap_or(0) as i32, i["at"][1].as_i64().unwrap_or(0) as i32);
                    game.map().is_high(x / TILE_SIZE, y / TILE_SIZE) == high
                })
                .count()
        })
    };
    let len = |key: &str| island[key].as_array().map_or(0, |v| v.len());
    let vines = |high: bool| {
        (0..game.map().island_size().1)
            .flat_map(|y| (0..game.map().width()).map(move |x| (x, y)))
            .filter(|&(x, y)| game.map().is_high(x, y) == high && game.map().feature(x, y) == Feature::Vine)
            .count()
    };
    // Fish in the lake (on the high ground), and elsewhere.
    let fish = |high: bool| {
        island["fish"].as_array().map_or(0, |v| {
            v.iter()
                .filter(|p| {
                    let (x, y) = (p[0].as_i64().unwrap_or(0) as i32, p[1].as_i64().unwrap_or(0) as i32);
                    game.map().is_high(x / TILE_SIZE, y / TILE_SIZE) == high
                })
                .count()
        })
    };
    println!("on the high ground: {} branches, {} stones, {} vines, {} fish in the lake", lying("Branch", true), lying("Stone", true), vines(true), fish(true));
    println!(
        "on the island ({} cells of dry land): {} branches, {} stones, {} vines, {} crabs, {} fish, {} shells on the beach (up to {} more each morning, to {})",
        (0..game.map().island_size().1)
            .flat_map(|y| (0..game.map().width()).map(move |x| (x, y)))
            .filter(|&(x, y)| !game.map().is_high(x, y) && matches!(game.map().terrain(x, y), Terrain::Sand | Terrain::Grass))
            .count(),
        lying("Branch", false),
        lying("Stone", false),
        vines(false),
        len("crabs"),
        fish(false),
        count_shells(game.map()),
        game.debug_shells_per_morning(),
        island["shell_cap"],
    );
    // Patches, counted by their top-left cells.
    let patches = |is: &dyn Fn(Feature) -> bool, high: bool| {
        (0..game.map().island_size().1)
            .flat_map(|y| (0..game.map().width()).map(move |x| (x, y)))
            .filter(|&(x, y)| game.map().is_high(x, y) == high && is(game.map().feature(x, y)))
            .count()
    };
    let clay = |f: Feature| matches!(f, Feature::ClayBed(c) if c.part == 0);
    let earth = |f: Feature| f == Feature::BareEarth(0);
    println!(
        "digging: {} clay beds on the lake shore; bare earth below the cliffs {}, on the high ground {}",
        patches(&clay, true),
        patches(&earth, false),
        patches(&earth, true)
    );
}

fn count_shells(map: &WorldMap) -> usize {
    (0..map.island_size().1).flat_map(|y| (0..map.width()).map(move |x| (x, y))).filter(|&(x, y)| map.feature(x, y) == Feature::Shell).count()
}

// --- What can be reached --------------------------------------------------

/// Cells the castaway can walk to from `start` (moving cell to cell through
/// ground that isn't solid; diagonal steps need both sides open).
fn reachable(map: &WorldMap, start: (i32, i32)) -> Vec<bool> {
    let (w, h) = (map.width(), map.island_size().1);
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

fn report_reach(map: &WorldMap, reach: &[bool], spots: &[(&str, char, Rgb, (i32, i32))]) {
    let w = map.width();
    let can = |c: (i32, i32)| map.in_bounds(c.0, c.1) && reach[(c.1 * w + c.0) as usize];
    let near = |c: (i32, i32)| (-1..=1).any(|dy| (-1..=1).any(|dx| can((c.0 + dx, c.1 + dy))));
    let cells: Vec<(i32, i32)> = (0..map.island_size().1).flat_map(|y| (0..w).map(move |x| (x, y))).filter(|&c| can(c)).collect();
    let span = |f: fn(&(i32, i32)) -> i32| (cells.iter().map(f).min().unwrap_or(0), cells.iter().map(f).max().unwrap_or(0));
    let ((x0, x1), (y0, y1)) = (span(|c| c.0), span(|c| c.1));
    println!("island {}x{} cells; walkable from the landing: {} cells, x {x0}..={x1}, y {y0}..={y1}", w, map.island_size().1, cells.len());
    println!("  high ground reached: {}", cells.iter().any(|&(x, y)| map.terrain(x, y) == Terrain::Highland));
    // The falls pool is water: it's seen, not walked to.
    for (name, letter, _, at) in spots.iter().filter(|s| s.1 != 'F') {
        println!("  {letter} {name} {at:?}: {}", if near(*at) { "in reach" } else { "OUT OF REACH" });
    }
    if let (Some((bx, by)), Some(river)) = (map.spots.bridge, map.river.as_ref()) {
        // Look across the river at the narrow, both ways.
        let p = river.nearest(bx as f32, by as f32);
        let banks: Vec<bool> =
            [-1, 1].iter().map(|side| (1..8).map(|k| (bx + side * k, by)).find(|&(x, y)| map.terrain(x, y) != Terrain::River).is_some_and(near)).collect();
        println!("  banks at the narrow in reach (west, east): {banks:?} (channel half-width {:.2})", p.half);
    }
}

// --- The plan map ---------------------------------------------------------

fn ground_colour(map: &WorldMap, x: i32, y: i32) -> Rgb {
    let c = match map.terrain(x, y) {
        Terrain::DeepWater => [20, 50, 120],
        Terrain::ShallowWater => [60, 140, 200],
        Terrain::River => [40, 110, 230],
        Terrain::Sand => [230, 210, 150],
        Terrain::Grass if map.elevation(x, y) > 17.0 => [40, 110, 50],
        Terrain::Grass => [100, 170, 70],
        Terrain::Highland => [140, 150, 70],
        Terrain::CliffTop => [90, 94, 44],
        Terrain::Cliff => [130, 126, 130],
        Terrain::Crag => [96, 92, 104],
        Terrain::Slope => [190, 150, 100],
        Terrain::Indoors => [0, 0, 0],
    };
    // The lake's wet soil.
    let c = if map.terrain(x, y) == Terrain::Sand && map.soil(x, y) == Soil::Mud { [120, 90, 60] } else { c };
    match map.feature(x, y) {
        Feature::Palm(_) => [30, 90, 30],
        Feature::Broadleaf(_) => [15, 60, 25],
        Feature::Pine(_) => [20, 75, 60],
        Feature::Bamboo(_) => [150, 190, 60],
        Feature::Stump(_) | Feature::BroadleafStump(_) | Feature::PineStump(_) | Feature::BambooStump(_) => [110, 80, 50],
        Feature::Bush | Feature::Vine | Feature::VineStake => [70, 140, 50],
        Feature::Rock => [120, 120, 120],
        Feature::Driftwood(_) => [150, 105, 60],
        Feature::Shell => [240, 160, 170],
        Feature::Flower => [230, 220, 90],
        Feature::PineCone => [170, 100, 40],
        Feature::ClayBed(_) => [150, 160, 175],
        Feature::BareEarth(_) => [150, 110, 70],
        Feature::Ladder(_) | Feature::Bridge(_) | Feature::WaterWheel(_) | Feature::Lathe(_) | Feature::SignalFire(_) | Feature::Marking(_) => [200, 150, 90],
        Feature::Foundation(_) => [170, 130, 90],
        Feature::Framed(post) => {
            if post {
                [120, 80, 50]
            } else {
                [170, 130, 90]
            }
        }
        Feature::SiteBench => [200, 150, 90],
        Feature::MillWall => [215, 190, 140],
        Feature::MillDoor => [120, 85, 55],
        Feature::MillRoomWall => [215, 190, 140],
        Feature::None => c,
    }
}

const LEGEND: [(&str, Rgb); 20] = [
    ("DEEP SEA", [20, 50, 120]),
    ("SHALLOWS", [60, 140, 200]),
    ("RIVER", [40, 110, 230]),
    ("BEACH", [230, 210, 150]),
    ("GRASS", [100, 170, 70]),
    ("FOREST", [40, 110, 50]),
    ("HIGH GROUND", [140, 150, 70]),
    ("CLIFF EDGE", [90, 94, 44]),
    ("CLIFF", [130, 126, 130]),
    ("CRAG", [96, 92, 104]),
    ("SLOPE", [190, 150, 100]),
    ("WET SOIL", [120, 90, 60]),
    ("PALM", [30, 90, 30]),
    ("BROADLEAF", [15, 60, 25]),
    ("PINE", [20, 75, 60]),
    ("BAMBOO", [150, 190, 60]),
    ("BUSH VINE", [70, 140, 50]),
    ("ROCK", [120, 120, 120]),
    ("CLAY BED", [150, 160, 175]),
    ("BARE EARTH", [150, 110, 70]),
];

fn plan_map(map: &WorldMap, reach: &[bool], spots: &[(&str, char, Rgb, (i32, i32))], s: i32, area: Option<(i32, i32, i32, i32)>) -> Image {
    let (w, h) = (map.width(), map.island_size().1);
    let mut img = Image::new(w * s, h * s, [0; 3]);
    for y in 0..h {
        for x in 0..w {
            let mut c = ground_colour(map, x, y);
            let walkable =
                !map.props(x, y).solid || matches!(map.feature(x, y), Feature::Palm(_) | Feature::Broadleaf(_) | Feature::Pine(_) | Feature::Bamboo(_));
            if walkable && map.terrain(x, y).is_land() && !reach[(y * w + x) as usize] {
                c = c.map(|v| (v as u32 * 3 / 5) as u8);
            }
            img.fill_rect(x * s, y * s, s, s, c);
        }
    }
    // The plan's blocks faintly, chunks (and labelled coordinates) firmly.
    for x in 0..=w {
        let a = if x % CHUNK_CELLS == 0 {
            150
        } else if x % PLAN_BLOCK == 0 {
            45
        } else {
            0
        };
        (0..h * s).for_each(|py| img.blend(x * s, py, [0; 3], a));
    }
    for y in 0..=h {
        let a = if y % CHUNK_CELLS == 0 {
            150
        } else if y % PLAN_BLOCK == 0 {
            45
        } else {
            0
        };
        (0..w * s).for_each(|px| img.blend(px, y * s, [0; 3], a));
    }
    if let Some((x0, y0, x1, y1)) = area {
        img.frame(x0 * s - 2, y0 * s - 2, (x1 - x0 + 1) * s + 4, (y1 - y0 + 1) * s + 4, 2, [255, 230, 0]);
    }
    for &(_, letter, c, (x, y)) in spots {
        img.frame(x * s - 2, y * s - 2, s + 4, s + 4, 2, c);
        img.text(x * s + s + 3, y * s - 1, FONT, c, &letter.to_string());
    }
    let framed = with_axes(&img, s as f32, (0, 0), PLAN_BLOCK * 4);
    // The legend underneath.
    let names: Vec<(String, Rgb)> = LEGEND
        .iter()
        .map(|&(n, c)| (n.to_string(), c))
        .chain(spots.iter().map(|&(n, l, c, _)| (format!("{l} {n}"), c)))
        .chain([("DARK: OUT OF REACH".to_string(), [70, 70, 70])])
        .collect();
    let entry_w = names.iter().map(|(n, _)| text_width(FONT, n)).max().unwrap_or(0) + 26;
    let per_row = ((framed.w - MARGIN_LEFT) / entry_w).max(1);
    let rows = (names.len() as i32 + per_row - 1) / per_row;
    let mut out = Image::new(framed.w, framed.h + rows * 16 + 6, AXIS_BG);
    out.paste(&framed, 0, 0);
    for (i, (name, c)) in names.iter().enumerate() {
        let (col, row) = (i as i32 % per_row, i as i32 / per_row);
        let (x, y) = (MARGIN_LEFT + col * entry_w, framed.h + 6 + row * 16);
        out.fill_rect(x, y, 10, 10, *c);
        out.text(x + 14, y, FONT, LABEL, name);
    }
    out
}

/// Adds coordinates along the top and left: a tick every cell (when there's
/// room) and a label every `step` cells. `origin` is the cell at the image's
/// top-left; `cell_px` how wide a cell is drawn.
fn with_axes(img: &Image, cell_px: f32, origin: (i32, i32), step: i32) -> Image {
    let mut out = Image::new(img.w + MARGIN_LEFT, img.h + MARGIN_TOP, AXIS_BG);
    out.paste(img, MARGIN_LEFT, MARGIN_TOP);
    let ticks = cell_px >= 4.0;
    let mut x = origin.0;
    while ((x - origin.0) as f32 * cell_px) < img.w as f32 {
        let px = MARGIN_LEFT + ((x - origin.0) as f32 * cell_px) as i32;
        if x % step == 0 {
            out.fill_rect(px, MARGIN_TOP - 4, 1, 4, LABEL);
            out.text(px + 2, 1, FONT, LABEL, &x.to_string());
        } else if ticks {
            out.fill_rect(px, MARGIN_TOP - 2, 1, 2, [120, 120, 120]);
        }
        x += 1;
    }
    let mut y = origin.1;
    while ((y - origin.1) as f32 * cell_px) < img.h as f32 {
        let py = MARGIN_TOP + ((y - origin.1) as f32 * cell_px) as i32;
        if y % step == 0 {
            out.fill_rect(MARGIN_LEFT - 4, py, 4, 1, LABEL);
            let label = y.to_string();
            out.text(MARGIN_LEFT - 6 - text_width(FONT, &label), py + 2, FONT, LABEL, &label);
        } else if ticks {
            out.fill_rect(MARGIN_LEFT - 2, py, 2, 1, [120, 120, 120]);
        }
        y += 1;
    }
    out
}

/// Labels at least ~48 px apart.
fn label_step(cell_px: f32) -> i32 {
    [4, 8, 16, 32, 64].into_iter().find(|&s| s as f32 * cell_px >= 48.0).unwrap_or(128)
}

// --- What changed ---------------------------------------------------------

/// One line per row of cells, two characters a cell (ground, then what
/// stands there), so runs can be compared (and read).
fn cell_signature(map: &WorldMap) -> String {
    let mut s = String::new();
    for y in 0..map.island_size().1 {
        for x in 0..map.width() {
            s.push(match map.terrain(x, y) {
                Terrain::DeepWater => '~',
                Terrain::ShallowWater => '-',
                Terrain::River => 'r',
                Terrain::Sand => '.',
                Terrain::Grass => ',',
                Terrain::Highland => '^',
                Terrain::CliffTop => 'e',
                Terrain::Cliff => '|',
                Terrain::Crag => 'A',
                Terrain::Slope => '/',
                Terrain::Indoors => 'I',
            });
            s.push(match map.feature(x, y) {
                Feature::None => ' ',
                Feature::Palm(_) => 'p',
                Feature::Stump(_) => 's',
                Feature::Broadleaf(_) => 'b',
                Feature::BroadleafStump(_) => 'S',
                Feature::Rock => 'o',
                Feature::Bush => 'h',
                Feature::Driftwood(_) => 'd',
                Feature::Shell => 'c',
                Feature::Flower => 'f',
                Feature::Vine => 'v',
                Feature::VineStake => 'V',
                Feature::Ladder(_) => 'L',
                Feature::Bridge(_) => '=',
                Feature::WaterWheel(_) => 'w',
                Feature::Lathe(_) => 'T',
                Feature::SiteBench => 'B',
                Feature::MillWall => '#',
                Feature::MillDoor => 'D',
                Feature::MillRoomWall => 'H',
                Feature::Framed(post) => {
                    if post {
                        'P'
                    } else {
                        '_'
                    }
                }
                Feature::Foundation(stone) => {
                    if stone {
                        'Q'
                    } else {
                        '_'
                    }
                }
                Feature::Marking(m) => {
                    if m.stake {
                        'X'
                    } else {
                        '+'
                    }
                }
                Feature::SignalFire(_) => 'F',
                Feature::Pine(_) => 'n',
                Feature::PineStump(_) => 'N',
                Feature::PineCone => 'k',
                Feature::Bamboo(_) => 'y',
                Feature::BambooStump(_) => 'Y',
                Feature::ClayBed(_) => 'C',
                Feature::BareEarth(_) => 'E',
            });
        }
        s.push('\n');
    }
    s
}

/// How a run's cells differ from an earlier run's.
struct CellDiff {
    /// The earlier island's size, in cells.
    old_size: (i32, i32),
    /// Where the earlier island's cells are now: cell (x, y) then is
    /// (x + dx, y + dy) now (the island grew on the west or north).
    shift: (i32, i32),
    /// Cells (now) that differ from the cell they were.
    changed: Vec<(i32, i32)>,
    /// Cells (now) the earlier island didn't have.
    added: Vec<(i32, i32)>,
    /// How many of the earlier island's cells are gone.
    removed: usize,
}

/// Compares two cell signatures. When the island's size changed, the
/// earlier one is lined up where it matches best: moved by anything from
/// nothing to all the cells it grew or shrank by, along each side.
fn diff_cells(old: &str, new: &str) -> CellDiff {
    let grid =
        |s: &str| -> Vec<Vec<[u8; 2]>> { s.lines().map(|l| l.as_bytes().chunks(2).map(|c| [c[0], c.get(1).copied().unwrap_or(b' ')]).collect()).collect() };
    let (old, new) = (grid(old), grid(new));
    let size = |g: &Vec<Vec<[u8; 2]>>| (g.iter().map(|r| r.len()).max().unwrap_or(0) as i32, g.len() as i32);
    let (old_size, new_size) = (size(&old), size(&new));
    let at = |g: &Vec<Vec<[u8; 2]>>, x: i32, y: i32| -> Option<[u8; 2]> { g.get(usize::try_from(y).ok()?)?.get(usize::try_from(x).ok()?).copied() };
    let span = |d: i32| d.min(0)..=d.max(0);
    let same = |(dx, dy): (i32, i32)| {
        (0..old_size.1)
            .flat_map(|y| (0..old_size.0).map(move |x| (x, y)))
            .filter(|&(x, y)| at(&old, x, y).is_some() && at(&old, x, y) == at(&new, x + dx, y + dy))
            .count()
    };
    let shift = span(new_size.0 - old_size.0)
        .flat_map(|dx| span(new_size.1 - old_size.1).map(move |dy| (dx, dy)))
        .max_by_key(|&(dx, dy)| (same((dx, dy)), -(dx.abs() + dy.abs())))
        .unwrap_or((0, 0));
    let (mut changed, mut added) = (Vec::new(), Vec::new());
    for y in 0..new_size.1 {
        for x in 0..new_size.0 {
            match at(&old, x - shift.0, y - shift.1) {
                None => added.push((x, y)),
                Some(c) if Some(c) != at(&new, x, y) => changed.push((x, y)),
                Some(_) => {}
            }
        }
    }
    let removed = (0..old_size.1)
        .flat_map(|y| (0..old_size.0).map(move |x| (x, y)))
        .filter(|&(x, y)| at(&old, x, y).is_some() && at(&new, x + shift.0, y + shift.1).is_none())
        .count();
    CellDiff { old_size, shift, changed, added, removed }
}

/// The view's area and shrink, from the `view.txt` a run wrote.
fn parse_view_meta(meta: &str) -> Option<((i32, i32, i32, i32), i32)> {
    let mut words = meta.split_whitespace();
    let (_, area, _, shrink) = (words.next()?, words.next()?, words.next()?, words.next()?);
    let v: Vec<i32> = area.split(',').map(|n| n.parse().ok()).collect::<Option<_>>()?;
    (v.len() == 4).then_some(((v[0], v[1], v[2], v[3]), shrink.parse().ok()?))
}

/// `old` (a map `old_size` px from the margins) moved `shift` px, in the
/// frame of `new` (its map `size` px): where the earlier run's map isn't,
/// it's blank.
fn align(old: &Image, new: &Image, old_size: (i32, i32), shift: (i32, i32), size: (i32, i32)) -> Image {
    let mut out = new.clone();
    out.fill_rect(MARGIN_LEFT, MARGIN_TOP, size.0, size.1, AXIS_BG);
    for y in 0..old_size.1 {
        for x in 0..old_size.0 {
            let (nx, ny) = (x + shift.0, y + shift.1);
            if (0..size.0).contains(&nx) && (0..size.1).contains(&ny) {
                out.set(MARGIN_LEFT + nx, MARGIN_TOP + ny, old.get(MARGIN_LEFT + x, MARGIN_TOP + y));
            }
        }
    }
    out
}

fn describe(what: &str, changed: &[(i32, i32)], against: &str) {
    if changed.is_empty() {
        println!("{what}: no cell changed since {against} (see {what}-diff.png for the new ones)");
        return;
    }
    let (x0, x1) = (changed.iter().map(|c| c.0).min().unwrap(), changed.iter().map(|c| c.0).max().unwrap());
    let (y0, y1) = (changed.iter().map(|c| c.1).min().unwrap(), changed.iter().map(|c| c.1).max().unwrap());
    println!("{what}: {} cells changed since {against}, within x {x0}..={x1}, y {y0}..={y1} (see {what}-diff.png)", changed.len());
}

/// The map (the `size` px from `offset`) greyed out, with the changed
/// and added cells in colour and an outline around each patch of them.
fn highlight(img: &Image, changed: &[(i32, i32)], added: &[(i32, i32)], cell_px: f32, origin: (i32, i32), offset: (i32, i32), size: (i32, i32)) -> Image {
    let mut out = img.clone();
    for y in offset.1..offset.1 + size.1 {
        for x in offset.0..offset.0 + size.0 {
            let p = img.get(x, y);
            let grey = ((p[0] as u32 * 3 + p[1] as u32 * 6 + p[2] as u32) / 10 / 2 + 50) as u8;
            out.set(x, y, [grey; 3]);
        }
    }
    let edge = |x: i32| offset.0 + ((x - origin.0) as f32 * cell_px).round() as i32;
    let edge_y = |y: i32| offset.1 + ((y - origin.1) as f32 * cell_px).round() as i32;
    for &(x, y) in changed.iter().chain(added) {
        for py in edge_y(y)..edge_y(y + 1) {
            for px in edge(x)..edge(x + 1) {
                if (0..img.w).contains(&px) && (0..img.h).contains(&py) {
                    out.set(px, py, img.get(px, py));
                }
            }
        }
    }
    for (cells, colour) in [(added, ADDED), (changed, CHANGED)] {
        let set: std::collections::HashSet<(i32, i32)> = cells.iter().copied().collect();
        for &(x, y) in cells {
            let (l, r, t, b) = (edge(x), edge(x + 1), edge_y(y), edge_y(y + 1));
            if !set.contains(&(x - 1, y)) {
                out.fill_rect(l - 1, t - 1, 2, b - t + 2, colour);
            }
            if !set.contains(&(x + 1, y)) {
                out.fill_rect(r - 1, t - 1, 2, b - t + 2, colour);
            }
            if !set.contains(&(x, y - 1)) {
                out.fill_rect(l - 1, t - 1, r - l + 2, 2, colour);
            }
            if !set.contains(&(x, y + 1)) {
                out.fill_rect(l - 1, b - 1, r - l + 2, 2, colour);
            }
        }
    }
    out
}

/// Before above after, each greyed out where nothing changed (`size` px of
/// map from the margins), the changed and added cells in colour and
/// outlined. `old` is lined up with `new` (see `align`).
fn before_after(old: &Image, new: &Image, changed: &[(i32, i32)], added: &[(i32, i32)], cell_px: f32, origin: (i32, i32), size: (i32, i32)) -> Image {
    let at = (MARGIN_LEFT, MARGIN_TOP);
    let (before, after) = (highlight(old, changed, added, cell_px, origin, at, size), highlight(new, changed, added, cell_px, origin, at, size));
    let mut out = Image::new(new.w, old.h + new.h + 32, AXIS_BG);
    out.text(2, 2, FONT, LABEL, "BEFORE");
    out.paste(&before, 0, 16);
    out.text(2, old.h + 18, FONT, LABEL, "AFTER");
    out.paste(&after, 0, old.h + 32);
    out
}

/// Cells of the view with any pixel changed, among those `covered` by
/// the earlier view.
fn changed_view_cells(old: &Image, new: &Image, cell_px: f32, origin: (i32, i32), covered: &dyn Fn((i32, i32)) -> bool) -> Vec<(i32, i32)> {
    let mut changed = std::collections::BTreeSet::new();
    for y in MARGIN_TOP..new.h {
        for x in MARGIN_LEFT..new.w {
            let cx = origin.0 + ((x - MARGIN_LEFT) as f32 / cell_px) as i32;
            let cy = origin.1 + ((y - MARGIN_TOP) as f32 / cell_px) as i32;
            if covered((cx, cy)) && old.get(x, y) != new.get(x, y) {
                changed.insert((cy, cx));
            }
        }
    }
    changed.into_iter().map(|(y, x)| (x, y)).collect()
}

// --- Views drawn by the game ----------------------------------------------

/// The cells `area` (inclusive) as the game draws them, a screen at a time.
fn draw_view(game: &mut Game, (x0, y0, x1, y1): (i32, i32, i32, i32)) -> Image {
    let (left, top) = (x0 * TILE_SIZE, y0 * TILE_SIZE);
    let (w, h) = ((x1 - x0 + 1) * TILE_SIZE, (y1 - y0 + 1) * TILE_SIZE);
    let (sw, sh) = (SCREEN_W as i32, SCREEN_H as i32);
    let mut img = Image::new(w, h, [0; 3]);
    for oy in (0..h).step_by(sh as usize) {
        for ox in (0..w).step_by(sw as usize) {
            game.debug_render_view((left + ox, top + oy));
            let rgba = game.rgba();
            for y in 0..sh.min(h - oy) {
                for x in 0..sw.min(w - ox) {
                    let i = ((y * sw + x) * 4) as usize;
                    img.set(ox + x, oy + y, [rgba[i], rgba[i + 1], rgba[i + 2]]);
                }
            }
        }
    }
    img
}

/// A screen around each special place, three to a row, each named.
fn spot_sheet(game: &mut Game, spots: &[(&str, char, Rgb, (i32, i32))]) -> Image {
    let (sw, sh) = (SCREEN_W as i32, SCREEN_H as i32);
    let (cols, gap, title) = (3, 6, 14);
    let rows = (spots.len() as i32 + cols - 1) / cols;
    let mut sheet = Image::new(cols * (sw + gap) + gap, rows * (sh + title + gap) + gap, AXIS_BG);
    for (i, &(name, letter, c, (x, y))) in spots.iter().enumerate() {
        let (col, row) = (i as i32 % cols, i as i32 / cols);
        let (ox, oy) = (gap + col * (sw + gap), gap + row * (sh + title + gap));
        let at = (x * TILE_SIZE + TILE_SIZE / 2 - sw / 2, y * TILE_SIZE + TILE_SIZE / 2 - sh / 2);
        game.debug_render_view(at);
        let rgba = game.rgba();
        let mut screen = Image::new(sw, sh, [0; 3]);
        for py in 0..sh {
            for px in 0..sw {
                let j = ((py * sw + px) * 4) as usize;
                screen.set(px, py, [rgba[j], rgba[j + 1], rgba[j + 2]]);
            }
        }
        // Corner marks around the place's cell.
        let (cx, cy) = (x * TILE_SIZE - at.0, y * TILE_SIZE - at.1);
        for (dx, dy, w, h) in [(-2, -2, 5, 1), (-2, -2, 1, 5), (13, -2, 5, 1), (17, -2, 1, 5), (-2, 17, 5, 1), (-2, 13, 1, 5), (13, 17, 5, 1), (17, 13, 1, 5)] {
            screen.fill_rect(cx + dx, cy + dy, w, h, c);
        }
        sheet.paste(&screen, ox, oy + title);
        sheet.text(ox, oy + 1, FONT, c, &format!("{letter} {name} ({x},{y})"));
    }
    sheet
}

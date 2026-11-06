//! Dev tool: runs the game natively and writes the final frame as a PPM image.
//!
//! Usage: cargo run --release --example snapshot -- <out.ppm> [script] [3h-skips]
//! `script` is a comma-separated list of `buttons:frames` steps, where
//! `buttons` is a SNES joypad word in hex (e.g. `0200:90,0400:40` = left for
//! 90 frames, then down for 40). Default: `0:30`. Set `SNAP_AT=x,y` to
//! start awake in that cell instead of washed up at the spawn, and
//! `SNAP_GIVE=stone,vine,branch` to start with those items in the bag, and
//! `SNAP_CRAB=dx` to set a crab down that far right of the castaway, and
//! `SNAP_HP=n` to start with `n` hearts.
use survival_core::game::Game;
use survival_core::items::ItemKind;
use survival_core::ppu::{SCREEN_H, SCREEN_W};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let out = args.get(1).cloned().unwrap_or_else(|| "snapshot.ppm".into());
    let script = args.get(2).cloned().unwrap_or_else(|| "0:30".into());
    let skips: u32 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(0);

    let mut game = Game::new(1);
    // SNAP_AT=x,y starts the castaway awake in that cell.
    if let Ok(at) = std::env::var("SNAP_AT") {
        let (x, y) = at.split_once(',').expect("SNAP_AT must be x,y");
        game.debug_place_player((x.parse().expect("x"), y.parse().expect("y")));
    }
    if let Ok(give) = std::env::var("SNAP_GIVE") {
        for name in give.split(',') {
            let kind = match name {
                "coconut" => ItemKind::Coconut,
                "timber" => ItemKind::Timber,
                "water" => ItemKind::RawWater,
                "branch" => ItemKind::Branch,
                "stone" => ItemKind::Stone,
                "vine" => ItemKind::Vine,
                "axe" => ItemKind::StoneAxe,
                "drill" => ItemKind::FireDrill,
                "spear" => ItemKind::WoodSpear,
                "crab" => ItemKind::Crab,
                "grilled" => ItemKind::GrilledCrab,
                "leaf" => ItemKind::Leaf,
                "bark" => ItemKind::Bark,
                "lauan" => ItemKind::LauanTimber,
                "pot" => ItemKind::BarkContainer,
                "boiled" => ItemKind::BoiledWater,
                "frame" => ItemKind::PackFrame,
                "husk" => ItemKind::Husk,
                "rope" => ItemKind::Rope,
                "frond" => ItemKind::PalmFrond,
                "thatch" => ItemKind::Thatch,
                "shelter" => ItemKind::Shelter,
                _ => panic!("unknown item {name}"),
            };
            game.debug_give(kind);
        }
    }
    // SNAP_HP=n starts with that many hearts.
    if let Ok(hp) = std::env::var("SNAP_HP") {
        game.debug_set_hp(hp.parse().expect("SNAP_HP must be hearts"));
    }
    // SNAP_CRAB=dx sets a crab down that far to the right of the feet.
    if let Ok(dx) = std::env::var("SNAP_CRAB") {
        game.debug_crab_ahead(dx.parse().expect("SNAP_CRAB must be px"));
    }
    // Each SELECT press skips 3 in-game hours (debug feature).
    for _ in 0..skips {
        game.step(0x2000);
        game.step(0);
    }
    for step in script.split(',') {
        let (buttons, frames) = step.split_once(':').expect("step must be buttons:frames");
        let buttons = u16::from_str_radix(buttons, 16).expect("buttons must be hex");
        for _ in 0..frames.parse::<u32>().expect("frames must be a number") {
            game.step(buttons);
        }
    }
    eprintln!("player tile {:?}", game.player_tile());
    game.render();
    let mut data = format!("P6\n{SCREEN_W} {SCREEN_H}\n255\n").into_bytes();
    for px in game.rgba().chunks(4) {
        data.extend_from_slice(&px[..3]);
    }
    std::fs::write(&out, data).expect("write snapshot");
}

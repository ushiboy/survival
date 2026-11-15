//! Dev tool: runs the game natively and writes the final frame as a PPM image.
//!
//! Usage: cargo run --release --example snapshot -- <out.ppm> [script] [3h-skips]
//! `script` is a comma-separated list of `buttons:frames` steps, where
//! `buttons` is a SNES joypad word in hex (e.g. `0200:90,0400:40` = left for
//! 90 frames, then down for 40). Default: `0:30`. Set `SNAP_AT=x,y` to
//! start awake in that cell instead of washed up at the spawn, and
//! `SNAP_GIVE=stone,vine,branch` to start with those items in the bag, and
//! `SNAP_CRAB=dx` to set a crab down that far right of the castaway, and
//! `SNAP_HP=n` to start with `n` health (of 64), `SNAP_FOOD=n` and
//! `SNAP_WATER=n` with that much food and water, `SNAP_UPSET=1` with an
//! upset stomach, and `SNAP_HOURS=n` to start `n`
//! hours later in the day, `SNAP_RECIPES=all` to know every recipe, and `SNAP_FISH=dx,dy` to put
//! a fish in the water that far from the castaway's feet. `SNAP_EVERY_FRAME=1`
//! draws every frame as the browser does; `SNAP_RELOAD=1` saves the game at
//! the end and draws it carried on from that save instead, checking it
//! saves the same again; `SNAP_SAVE_OUT=path` writes the save there, and
//! `SNAP_SAVE_IN=path` starts with that save kept, CONTINUE offered (the
//! title screen is skipped
//! unless `SNAP_TITLE=1`), and reports the ground blocks
//! held at the end.
use survival_core::game::{Game, SaveOp};
use survival_core::items::ItemKind;
use survival_core::ppu::{SCREEN_H, SCREEN_W};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let out = args.get(1).cloned().unwrap_or_else(|| "snapshot.ppm".into());
    let script = args.get(2).cloned().unwrap_or_else(|| "0:30".into());
    let skips: u32 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(0);

    // SNAP_SAVE_IN=path starts at the title with that save kept (CONTINUE
    // offered).
    let kept = std::env::var("SNAP_SAVE_IN").ok().map(|p| std::fs::read_to_string(p).expect("read save"));
    let mut game = Game::with_save(1, kept.as_deref());
    // SNAP_TITLE=1 starts at the title screen; otherwise it's skipped.
    if std::env::var("SNAP_TITLE").is_err() {
        game.skip_title();
    }
    // SNAP_AT=x,y starts the castaway awake in that cell.
    if let Ok(at) = std::env::var("SNAP_AT") {
        let (x, y) = at.split_once(',').expect("SNAP_AT must be x,y");
        game.debug_place_player((x.parse().expect("x"), y.parse().expect("y")));
    }
    // SNAP_FISH=dx,dy puts a fish in the water that far from the feet.
    if let Ok(at) = std::env::var("SNAP_FISH") {
        let (x, y) = at.split_once(',').expect("SNAP_FISH must be dx,dy");
        game.debug_fish_at((x.parse().expect("dx"), y.parse().expect("dy")));
    }
    // SNAP_RECIPES=all starts with every recipe known.
    if std::env::var("SNAP_RECIPES").is_ok_and(|v| v == "all") {
        game.debug_learn_recipes();
    }
    // SNAP_HOURS=n starts n hours later in the day (before SNAP_GIVE).
    if let Some(h) = std::env::var("SNAP_HOURS").ok().and_then(|h| h.parse().ok()) {
        game.debug_advance_hours(h);
    }
    if let Ok(give) = std::env::var("SNAP_GIVE") {
        for name in give.split(',') {
            let kind = match name {
                "coconut" => ItemKind::Coconut,
                "opened" => ItemKind::OpenedCoconut,
                "timber" => ItemKind::Timber,
                "water" => ItemKind::RawWater,
                "branch" => ItemKind::Branch,
                "stone" => ItemKind::Stone,
                "vine" => ItemKind::Vine,
                "axe" => ItemKind::StoneAxe,
                "knife" => ItemKind::StoneKnife,
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
                "torch" => ItemKind::Torch,
                "shell" => ItemKind::SeaShell,
                "hook" => ItemKind::ShellHook,
                "cone" => ItemKind::PineCone,
                "pine" => ItemKind::PineTimber,
                "bamboo" => ItemKind::Bamboo,
                "rod" => ItemKind::FishingRod,
                "fish" => ItemKind::Fish,
                "grilledfish" => ItemKind::GrilledFish,
                "ladder" => ItemKind::Ladder,
                "log bridge" | "bridge" => ItemKind::LogBridge,
                "water wheel" | "wheel" => ItemKind::WaterWheel,
                _ => panic!("unknown item {name}"),
            };
            game.debug_give(kind);
        }
    }
    // SNAP_HP / SNAP_FOOD / SNAP_WATER=n start with that much (of 64).
    let gauge = |name: &str| std::env::var(name).ok().map(|n| n.parse().expect("gauges run 0..=64"));
    game.debug_set_vitals(gauge("SNAP_HP"), gauge("SNAP_FOOD"), gauge("SNAP_WATER"));
    // SNAP_UPSET=1 starts with an upset stomach.
    if std::env::var("SNAP_UPSET").is_ok_and(|v| v == "1") {
        game.debug_upset_stomach();
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
    // SNAP_EVERY_FRAME=1 draws every frame, as the browser does (so the
    // ground streams in and out as it would there).
    let every_frame = std::env::var("SNAP_EVERY_FRAME").is_ok();
    let mut saves: Vec<&str> = Vec::new();
    for step in script.split(',') {
        let (buttons, frames) = step.split_once(':').expect("step must be buttons:frames");
        let buttons = u16::from_str_radix(buttons, 16).expect("buttons must be hex");
        for _ in 0..frames.parse::<u32>().expect("frames must be a number") {
            game.step(buttons);
            match game.take_save_op() {
                Some(SaveOp::Write(_)) => saves.push("saved"),
                Some(SaveOp::Erase) => saves.push("erased"),
                None => {}
            }
            if every_frame {
                game.render();
            }
        }
    }
    eprintln!("player tile {:?}", game.player_tile());
    if !saves.is_empty() {
        eprintln!("saved game: {}", saves.join(", "));
    }
    if every_frame {
        eprintln!("ground blocks held {}", game.ground_blocks());
    }
    // SNAP_RELOAD=1 saves the game at the end, carries on from that save in
    // a new game (drawn instead), and checks the reloaded game saves the
    // same again. SNAP_SAVE_OUT=path also writes the save there.
    if std::env::var("SNAP_RELOAD").is_ok() || std::env::var("SNAP_SAVE_OUT").is_ok() {
        let json = game.debug_save_json();
        if let Ok(path) = std::env::var("SNAP_SAVE_OUT") {
            std::fs::write(&path, &json).expect("write save");
        }
        if std::env::var("SNAP_RELOAD").is_ok() {
            game = Game::debug_continue(&json).expect("the save loads");
            let again = game.debug_save_json();
            eprintln!("save {} bytes; reloaded game saves the same: {}", json.len(), again == json);
        }
    }
    game.render();
    let mut data = format!("P6\n{SCREEN_W} {SCREEN_H}\n255\n").into_bytes();
    for px in game.rgba().chunks(4) {
        data.extend_from_slice(&px[..3]);
    }
    std::fs::write(&out, data).expect("write snapshot");
}

//! Dev tool: runs the game natively and writes the final frame as a PPM image.
//!
//! Usage: cargo run --release --example snapshot -- <out.ppm> [script] [3h-skips]
//! `script` is a comma-separated list of `buttons:frames` steps, where
//! `buttons` is a SNES joypad word in hex (e.g. `0200:90,0400:40` = left for
//! 90 frames, then down for 40). Default: `0:30`.
use survival_core::game::Game;
use survival_core::ppu::{SCREEN_H, SCREEN_W};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let out = args.get(1).cloned().unwrap_or_else(|| "snapshot.ppm".into());
    let script = args.get(2).cloned().unwrap_or_else(|| "0:30".into());
    let skips: u32 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(0);

    let mut game = Game::new(1);
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
    game.render();
    let mut data = format!("P6\n{SCREEN_W} {SCREEN_H}\n255\n").into_bytes();
    for px in game.rgba().chunks(4) {
        data.extend_from_slice(&px[..3]);
    }
    std::fs::write(&out, data).expect("write snapshot");
}

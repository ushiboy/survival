//! Dev tool: renders a frame natively and writes it as a PPM image.
//! Usage: cargo run --example snapshot -- <out.ppm> [frames] [buttons]
use survival_core::game::Game;
use survival_core::ppu::{SCREEN_H, SCREEN_W};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let out = args.get(1).cloned().unwrap_or_else(|| "snapshot.ppm".into());
    let frames: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(1);
    let buttons: u8 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(0);
    let mut game = Game::new(1);
    for _ in 0..frames {
        game.step(buttons);
    }
    game.render();
    let mut data = format!("P6\n{SCREEN_W} {SCREEN_H}\n255\n").into_bytes();
    for px in game.rgba().chunks(4) {
        data.extend_from_slice(&px[..3]);
    }
    std::fs::write(&out, data).expect("write snapshot");
}

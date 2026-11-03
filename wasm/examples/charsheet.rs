//! Dev tool: renders every pose of each outfit preset into one PPM sheet.
//! Usage: cargo run --release --example charsheet -- <out.ppm> [scale]
use survival_core::assets::character::pose::{View, chop, crouch, drink, kneeling, shake, sitting, walk};
use survival_core::assets::character::render::render;
use survival_core::assets::character::{Outfit, render_lying};
use survival_core::ppu::pattern::PixelArt;

const CELL: usize = 34;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let out = args.get(1).cloned().unwrap_or_else(|| "chars.ppm".into());
    let scale: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(4);
    let views = [View::Front, View::Back, View::Side];
    let cols = 11;
    let rows_per_outfit = views.len() + 1;
    let (w, h) = (cols * CELL * scale, Outfit::PRESETS.len() * rows_per_outfit * CELL * scale);
    let mut img = vec![[200u8, 180, 140]; w * h];

    for (oi, outfit) in Outfit::PRESETS.iter().enumerate() {
        let pal = outfit.palette();
        let mut row: Vec<Vec<PixelArt>> = views
            .iter()
            .map(|&v| (0..4).map(|p| walk(v, p)).chain((0..3).map(|s| chop(v, s))).chain((0..2).map(|s| shake(v, s))).chain([crouch(v), drink(v)]).map(|p| render(&p, outfit).body).collect())
            .collect();
        row.push(vec![
            render_lying(outfit, false),
            render_lying(outfit, true),
            render(&sitting(), outfit).body,
            render(&kneeling(), outfit).body,
        ]);
        for (ri, arts) in row.iter().enumerate() {
            for (ci, art) in arts.iter().enumerate() {
                let (ox, oy) = (ci * CELL + 1, (oi * rows_per_outfit + ri) * CELL + 1);
                for y in 0..art.h {
                    for x in 0..art.w {
                        let v = art.get(x, y);
                        if v == 0 {
                            continue;
                        }
                        let c = pal[v as usize - 1].to_rgba8();
                        for sy in 0..scale {
                            for sx in 0..scale {
                                img[((oy + y) * scale + sy) * w + (ox + x) * scale + sx] = [c[0], c[1], c[2]];
                            }
                        }
                    }
                }
            }
        }
    }
    let mut data = format!("P6\n{w} {h}\n255\n").into_bytes();
    data.extend(img.iter().flatten());
    std::fs::write(out, data).expect("write sheet");
}

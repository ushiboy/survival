//! Dev tool: renders every pose of each outfit preset into one PPM sheet.
//! Usage: cargo run --release --example charsheet -- <out.ppm> [scale]
use survival_core::assets::character::pose::{View, chop, walk};
use survival_core::assets::character::render::render;
use survival_core::assets::character::{CHAR_H, CHAR_W, Outfit};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let out = args.get(1).cloned().unwrap_or_else(|| "chars.ppm".into());
    let scale: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(4);
    let views = [View::Front, View::Back, View::Side];
    let poses: Vec<_> = views
        .iter()
        .flat_map(|&v| (0..4).map(move |p| walk(v, p)).chain((0..3).map(move |s| chop(v, s))))
        .collect();
    let cols = poses.len() / views.len();
    let rows = views.len() * Outfit::PRESETS.len();
    let (cw, ch) = (CHAR_W + 2, CHAR_H + 2);
    let (w, h) = (cols * cw * scale, rows * ch * scale);
    let mut img = vec![[200u8, 180, 140]; w * h];
    for (oi, outfit) in Outfit::PRESETS.iter().enumerate() {
        let pal = outfit.palette();
        for (pi, pose) in poses.iter().enumerate() {
            let art = render(pose, outfit).body;
            let (gx, gy) = (pi % cols, oi * views.len() + pi / cols);
            for y in 0..CHAR_H {
                for x in 0..CHAR_W {
                    let v = art.get(x, y);
                    if v == 0 {
                        continue;
                    }
                    let rgba = pal[v as usize - 1].to_rgba8();
                    for sy in 0..scale {
                        for sx in 0..scale {
                            let (px, py) = ((gx * cw + x + 1) * scale + sx, (gy * ch + y + 1) * scale + sy);
                            img[py * w + px] = [rgba[0], rgba[1], rgba[2]];
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

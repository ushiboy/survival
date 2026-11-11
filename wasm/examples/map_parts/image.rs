//! An RGB image for the map tool: drawing, a tiny font, and PNG in and out
//! (only the plain PNGs written here are read back).

use std::path::Path;

pub type Rgb = [u8; 3];

#[derive(Clone)]
pub struct Image {
    pub w: i32,
    pub h: i32,
    px: Vec<Rgb>,
}

impl Image {
    pub fn new(w: i32, h: i32, fill: Rgb) -> Self {
        Self { w, h, px: vec![fill; (w * h) as usize] }
    }

    pub fn get(&self, x: i32, y: i32) -> Rgb {
        self.px[(y * self.w + x) as usize]
    }

    pub fn set(&mut self, x: i32, y: i32, c: Rgb) {
        if (0..self.w).contains(&x) && (0..self.h).contains(&y) {
            self.px[(y * self.w + x) as usize] = c;
        }
    }

    /// Mixes `c` into a pixel, `a` of 255 parts.
    pub fn blend(&mut self, x: i32, y: i32, c: Rgb, a: u32) {
        if (0..self.w).contains(&x) && (0..self.h).contains(&y) {
            let p = &mut self.px[(y * self.w + x) as usize];
            for i in 0..3 {
                p[i] = ((p[i] as u32 * (255 - a) + c[i] as u32 * a) / 255) as u8;
            }
        }
    }

    pub fn fill_rect(&mut self, x: i32, y: i32, w: i32, h: i32, c: Rgb) {
        for yy in y..y + h {
            for xx in x..x + w {
                self.set(xx, yy, c);
            }
        }
    }

    /// A rectangle's outline, `t` px thick, inside the rectangle.
    pub fn frame(&mut self, x: i32, y: i32, w: i32, h: i32, t: i32, c: Rgb) {
        self.fill_rect(x, y, w, t, c);
        self.fill_rect(x, y + h - t, w, t, c);
        self.fill_rect(x, y, t, h, c);
        self.fill_rect(x + w - t, y, t, h, c);
    }

    /// Copies `src` in with its top-left at (x, y).
    pub fn paste(&mut self, src: &Image, x: i32, y: i32) {
        for sy in 0..src.h {
            for sx in 0..src.w {
                self.set(x + sx, y + sy, src.get(sx, sy));
            }
        }
    }

    /// Each `n`×`n` square averaged into one pixel.
    pub fn shrink(&self, n: i32) -> Image {
        if n <= 1 {
            return self.clone();
        }
        let mut out = Image::new(self.w / n, self.h / n, [0; 3]);
        for y in 0..out.h {
            for x in 0..out.w {
                let mut sum = [0u32; 3];
                for dy in 0..n {
                    for dx in 0..n {
                        let p = self.get(x * n + dx, y * n + dy);
                        (0..3).for_each(|i| sum[i] += p[i] as u32);
                    }
                }
                out.set(x, y, sum.map(|s| (s / (n * n) as u32) as u8));
            }
        }
        out
    }

    /// Text in the tiny font, each font pixel `s`×`s`, with a dark shadow
    /// so it reads on any ground. Returns the width drawn.
    pub fn text(&mut self, x: i32, y: i32, s: i32, c: Rgb, text: &str) -> i32 {
        let o = (s / 2).max(1);
        self.text_plain(x + o, y + o, s, [0, 0, 0], text);
        self.text_plain(x, y, s, c, text);
        text_width(s, text)
    }

    fn text_plain(&mut self, x: i32, y: i32, s: i32, c: Rgb, text: &str) {
        for (i, ch) in text.chars().enumerate() {
            let bits = glyph(ch);
            let gx = x + i as i32 * 4 * s;
            for row in 0..5 {
                for col in 0..3 {
                    if bits >> (14 - (row * 3 + col)) & 1 == 1 {
                        self.fill_rect(gx + col * s, y + row * s, s, s, c);
                    }
                }
            }
        }
    }

    pub fn save_png(&self, path: &Path) {
        let mut raw = Vec::with_capacity((self.w * (self.h * 3 + 1)) as usize);
        for y in 0..self.h {
            raw.push(0); // no filter
            for x in 0..self.w {
                raw.extend_from_slice(&self.get(x, y));
            }
        }
        let mut header = Vec::new();
        header.extend_from_slice(&(self.w as u32).to_be_bytes());
        header.extend_from_slice(&(self.h as u32).to_be_bytes());
        header.extend_from_slice(&[8, 2, 0, 0, 0]); // 8-bit RGB
        let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
        chunk(&mut out, b"IHDR", &header);
        chunk(&mut out, b"IDAT", &miniz_oxide::deflate::compress_to_vec_zlib(&raw, 6));
        chunk(&mut out, b"IEND", &[]);
        std::fs::write(path, out).unwrap_or_else(|e| panic!("write {}: {e}", path.display()));
    }

    /// Reads back a PNG written by `save_png` (None for anything else).
    pub fn load_png(path: &Path) -> Option<Image> {
        let data = std::fs::read(path).ok()?;
        let mut at = 8;
        let (mut w, mut h) = (0, 0);
        let mut idat = Vec::new();
        while at + 8 <= data.len() {
            let len = u32::from_be_bytes(data[at..at + 4].try_into().ok()?) as usize;
            let kind = &data[at + 4..at + 8];
            let body = data.get(at + 8..at + 8 + len)?;
            match kind {
                b"IHDR" => {
                    w = u32::from_be_bytes(body[0..4].try_into().ok()?) as i32;
                    h = u32::from_be_bytes(body[4..8].try_into().ok()?) as i32;
                    if body[8..13] != [8, 2, 0, 0, 0] {
                        return None;
                    }
                }
                b"IDAT" => idat.extend_from_slice(body),
                _ => {}
            }
            at += 12 + len;
        }
        let raw = miniz_oxide::inflate::decompress_to_vec_zlib(&idat).ok()?;
        if raw.len() != (h * (w * 3 + 1)) as usize {
            return None;
        }
        let mut img = Image::new(w, h, [0; 3]);
        for y in 0..h {
            let row = &raw[(y * (w * 3 + 1)) as usize..][..(w * 3 + 1) as usize];
            if row[0] != 0 {
                return None;
            }
            for x in 0..w {
                let p = &row[1 + (x * 3) as usize..][..3];
                img.set(x, y, [p[0], p[1], p[2]]);
            }
        }
        Some(img)
    }
}

fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], body: &[u8]) {
    out.extend_from_slice(&(body.len() as u32).to_be_bytes());
    out.extend_from_slice(kind);
    out.extend_from_slice(body);
    let mut crc = crc32fast::Hasher::new();
    crc.update(kind);
    crc.update(body);
    out.extend_from_slice(&crc.finalize().to_be_bytes());
}

pub fn text_width(s: i32, text: &str) -> i32 {
    (text.chars().count() as i32 * 4 - 1).max(0) * s
}

/// A 3×5 glyph, rows top to bottom, 3 bits each (left bit first).
fn glyph(c: char) -> u16 {
    match c.to_ascii_uppercase() {
        '0' => 0b111_101_101_101_111,
        '1' => 0b010_110_010_010_111,
        '2' => 0b111_001_111_100_111,
        '3' => 0b111_001_111_001_111,
        '4' => 0b101_101_111_001_001,
        '5' => 0b111_100_111_001_111,
        '6' => 0b111_100_111_101_111,
        '7' => 0b111_001_001_010_010,
        '8' => 0b111_101_111_101_111,
        '9' => 0b111_101_111_001_111,
        'A' => 0b010_101_111_101_101,
        'B' => 0b110_101_110_101_110,
        'C' => 0b011_100_100_100_011,
        'D' => 0b110_101_101_101_110,
        'E' => 0b111_100_110_100_111,
        'F' => 0b111_100_110_100_100,
        'G' => 0b011_100_101_101_011,
        'H' => 0b101_101_111_101_101,
        'I' => 0b111_010_010_010_111,
        'J' => 0b001_001_001_101_010,
        'K' => 0b101_101_110_101_101,
        'L' => 0b100_100_100_100_111,
        'M' => 0b101_111_101_101_101,
        'N' => 0b101_111_111_111_101,
        'O' => 0b010_101_101_101_010,
        'P' => 0b110_101_110_100_100,
        'Q' => 0b010_101_101_110_011,
        'R' => 0b110_101_110_101_101,
        'S' => 0b011_100_010_001_110,
        'T' => 0b111_010_010_010_010,
        'U' => 0b101_101_101_101_111,
        'V' => 0b101_101_101_101_010,
        'W' => 0b101_101_111_111_101,
        'X' => 0b101_101_010_101_101,
        'Y' => 0b101_101_010_010_010,
        'Z' => 0b111_001_010_100_111,
        '-' => 0b000_000_111_000_000,
        ',' => 0b000_000_000_010_100,
        '.' => 0b000_000_000_000_010,
        '(' => 0b010_100_100_100_010,
        ')' => 0b010_001_001_001_010,
        '/' => 0b001_001_010_100_100,
        ':' => 0b000_010_000_010_000,
        '=' => 0b000_111_000_111_000,
        '+' => 0b000_010_111_010_000,
        _ => 0,
    }
}

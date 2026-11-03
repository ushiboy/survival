//! 2bpp pattern data ("CHR") and a text-based pixel-art format used to author it.
//!
//! Art is written as rows of characters: `.`/`0` = index 0, `1`..`3` = index 1..3.
//! For sprites index 0 is transparent; for backgrounds it is the universal colour.

pub const TILE8_LEN: usize = 8 * 8;
pub const TILE16_LEN: usize = 16 * 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tile8(pub [u8; TILE8_LEN]);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tile16(pub [u8; TILE16_LEN]);

/// Editable indexed bitmap (values 0..=3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PixelArt {
    pub w: usize,
    pub h: usize,
    pub px: Vec<u8>,
}

fn default_legend(c: char) -> Option<u8> {
    match c {
        '.' | '0' => Some(0),
        '1' => Some(1),
        '2' => Some(2),
        '3' => Some(3),
        _ => None,
    }
}

impl PixelArt {
    pub fn filled(w: usize, h: usize, value: u8) -> Self {
        Self { w, h, px: vec![value; w * h] }
    }

    pub fn parse(rows: &[&str]) -> Self {
        Self::parse_with(rows, default_legend)
    }

    /// Parses rows with a custom character legend. Panics on malformed art so
    /// mistakes surface immediately at startup.
    pub fn parse_with(rows: &[&str], legend: impl Fn(char) -> Option<u8>) -> Self {
        assert!(!rows.is_empty(), "pixel art has no rows");
        let w = rows[0].chars().count();
        let mut px = Vec::with_capacity(w * rows.len());
        for (i, row) in rows.iter().enumerate() {
            assert_eq!(
                row.chars().count(),
                w,
                "pixel art row {i} has wrong width: {row:?}"
            );
            for c in row.chars() {
                px.push(legend(c).unwrap_or_else(|| panic!("bad pixel char {c:?} in row {i}")));
            }
        }
        Self { w, h: rows.len(), px }
    }

    #[inline]
    pub fn get(&self, x: usize, y: usize) -> u8 {
        self.px[y * self.w + x]
    }

    #[inline]
    pub fn set(&mut self, x: usize, y: usize, v: u8) {
        self.px[y * self.w + x] = v;
    }

    /// Draws text rows on top of this image; `.` is skipped (transparent),
    /// pixels outside the image are clipped.
    pub fn stamp(&mut self, rows: &[&str], x: i32, y: i32) {
        for (dy, row) in rows.iter().enumerate() {
            for (dx, c) in row.chars().enumerate() {
                if c == '.' {
                    continue;
                }
                let (px, py) = (x + dx as i32, y + dy as i32);
                if px < 0 || py < 0 || px >= self.w as i32 || py >= self.h as i32 {
                    continue;
                }
                if let Some(v) = default_legend(c) {
                    self.set(px as usize, py as usize, v);
                }
            }
        }
    }

    pub fn sub(&self, x: usize, y: usize, w: usize, h: usize) -> Self {
        let mut out = Self::filled(w, h, 0);
        for yy in 0..h {
            for xx in 0..w {
                out.set(xx, yy, self.get(x + xx, y + yy));
            }
        }
        out
    }

    pub fn flipped_h(&self) -> Self {
        let mut out = self.clone();
        for y in 0..self.h {
            for x in 0..self.w {
                out.set(x, y, self.get(self.w - 1 - x, y));
            }
        }
        out
    }

    /// Clears every row from `from_row` downward.
    pub fn cleared_below(&self, from_row: usize) -> Self {
        let mut out = self.clone();
        for v in out.px[from_row.min(self.h) * self.w..].iter_mut() {
            *v = 0;
        }
        out
    }

    pub fn to_tile16(&self) -> Tile16 {
        assert!(self.w == 16 && self.h == 16, "expected 16x16 art, got {}x{}", self.w, self.h);
        let mut t = [0u8; TILE16_LEN];
        t.copy_from_slice(&self.px);
        Tile16(t)
    }

    pub fn to_tile8(&self) -> Tile8 {
        assert!(self.w == 8 && self.h == 8, "expected 8x8 art, got {}x{}", self.w, self.h);
        let mut t = [0u8; TILE8_LEN];
        t.copy_from_slice(&self.px);
        Tile8(t)
    }

    /// Splits into 8x8 tiles in row-major order.
    pub fn split8(&self) -> Vec<Tile8> {
        assert!(self.w % 8 == 0 && self.h % 8 == 0, "art size must be a multiple of 8");
        let mut out = Vec::new();
        for ty in 0..self.h / 8 {
            for tx in 0..self.w / 8 {
                out.push(self.sub(tx * 8, ty * 8, 8, 8).to_tile8());
            }
        }
        out
    }
}

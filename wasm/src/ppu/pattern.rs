//! Indexed pattern data and an editable bitmap used to author it.
//!
//! Hand-drawn art is written as text rows with a per-image legend mapping
//! characters to palette indices (`.` is always 0 = transparent).

pub const TILE8_LEN: usize = 8 * 8;
pub const TILE16_LEN: usize = 16 * 16;

/// 8x8 2bpp tile (values 0..=3), used by BG3.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tile8(pub [u8; TILE8_LEN]);

/// 16x16 4bpp tile (values 0..=15), used by BG1/BG2.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tile16(pub [u8; TILE16_LEN]);

/// Editable indexed bitmap.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PixelArt {
    pub w: usize,
    pub h: usize,
    pub px: Vec<u8>,
}

impl PixelArt {
    pub fn filled(w: usize, h: usize, value: u8) -> Self {
        Self { w, h, px: vec![value; w * h] }
    }

    /// Parses text rows; `legend` lists `(char, index)` pairs, `.` is 0.
    /// Panics on malformed art so mistakes surface at startup.
    pub fn parse(rows: &[&str], legend: &[(char, u8)]) -> Self {
        assert!(!rows.is_empty(), "pixel art has no rows");
        let w = rows[0].chars().count();
        let mut px = Vec::with_capacity(w * rows.len());
        for (i, row) in rows.iter().enumerate() {
            assert_eq!(row.chars().count(), w, "pixel art row {i} has wrong width: {row:?}");
            for c in row.chars() {
                let v = if c == '.' {
                    0
                } else {
                    legend
                        .iter()
                        .find(|(k, _)| *k == c)
                        .map(|(_, v)| *v)
                        .unwrap_or_else(|| panic!("unknown pixel char {c:?} in row {i}: {row:?}"))
                };
                px.push(v);
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

    /// Bounds-checked set with signed coordinates.
    #[inline]
    pub fn put(&mut self, x: i32, y: i32, v: u8) {
        if x >= 0 && y >= 0 && (x as usize) < self.w && (y as usize) < self.h {
            self.set(x as usize, y as usize, v);
        }
    }

    #[inline]
    pub fn at(&self, x: i32, y: i32) -> u8 {
        if x >= 0 && y >= 0 && (x as usize) < self.w && (y as usize) < self.h {
            self.get(x as usize, y as usize)
        } else {
            0
        }
    }

    /// Wrapping get, for periodic textures.
    #[inline]
    pub fn get_wrap(&self, x: i32, y: i32) -> u8 {
        self.get(x.rem_euclid(self.w as i32) as usize, y.rem_euclid(self.h as i32) as usize)
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

    pub fn flipped_v(&self) -> Self {
        let mut out = self.clone();
        for y in 0..self.h {
            for x in 0..self.w {
                out.set(x, y, self.get(x, self.h - 1 - y));
            }
        }
        out
    }

    /// Swaps rows and columns.
    pub fn transposed(&self) -> Self {
        let mut out = Self::filled(self.h, self.w, 0);
        for y in 0..self.h {
            for x in 0..self.w {
                out.set(y, x, self.get(x, y));
            }
        }
        out
    }

    /// Shifts the given rows horizontally by `dx` (pixels pushed off an edge
    /// are lost). Used to lean a sprite's upper body over its hips.
    pub fn shift_rows(&self, rows: std::ops::Range<usize>, dx: i32) -> Self {
        let mut out = self.clone();
        for y in rows.start..rows.end.min(self.h) {
            for x in 0..self.w as i32 {
                out.set(x as usize, y, self.at(x - dx, y as i32));
            }
        }
        out
    }

    /// Rotated 90° clockwise.
    pub fn rotated_cw(&self) -> Self {
        self.transposed().flipped_h()
    }

    /// Copy with every row outside `rows` cleared.
    pub fn keep_rows(&self, rows: std::ops::Range<usize>) -> Self {
        let mut out = self.clone();
        for y in 0..self.h {
            if !rows.contains(&y) {
                out.px[y * self.w..(y + 1) * self.w].fill(0);
            }
        }
        out
    }

    /// Draws non-zero pixels of `top` onto self at (x, y).
    pub fn overlay(&mut self, top: &PixelArt, x: i32, y: i32) {
        for ty in 0..top.h {
            for tx in 0..top.w {
                let v = top.get(tx, ty);
                if v != 0 {
                    self.put(x + tx as i32, y + ty as i32, v);
                }
            }
        }
    }

    /// Adds an outline of `color` around all non-zero pixels.
    pub fn outlined(&self, color: u8) -> Self {
        let mut out = self.clone();
        for y in 0..self.h as i32 {
            for x in 0..self.w as i32 {
                if self.at(x, y) != 0 {
                    continue;
                }
                let near = [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().any(|(dx, dy)| self.at(x + dx, y + dy) != 0);
                if near {
                    out.put(x, y, color);
                }
            }
        }
        out
    }

    pub fn map(&self, f: impl Fn(u8) -> u8) -> Self {
        Self { w: self.w, h: self.h, px: self.px.iter().map(|&v| f(v)).collect() }
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
}

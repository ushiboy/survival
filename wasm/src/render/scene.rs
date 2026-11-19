//! The rescue ending's cuts on the PPU: a picture painted in full colour
//! (`assets::ending_art`) is fitted to what the SFC can show, as it would
//! be converted for the cartridge: BG1 blocks of 16x16, each in one of six
//! background palettes of 15 colours (palettes 2-7; 0-1 are the HUD's), and
//! sprites in one palette of 15.
//!
//! The palettes are found by grouping the blocks by their colours (each
//! group a palette, cut from the colours of its blocks by median cut), then
//! letting each block take the palette that draws it best and cutting the
//! palettes again from the blocks that took them, a few times over.

use crate::assets::ending_art::Canvas;
use crate::ppu::banks::{MetatileBank, MetatileId, SpriteBank, SpriteId};
use crate::ppu::palette::Rgb555;
use crate::ppu::pattern::PixelArt;
use crate::ppu::{BgCell, BgLayer};

/// Colours to a palette (entry 0 is the backdrop, or see-through).
const COLORS: usize = 15;
/// Background palettes a cut may use, from `FIRST_PALETTE`.
pub const PALETTES: usize = 6;
pub const FIRST_PALETTE: u8 = 2;
const BLOCK: usize = 16;
/// Rounds of letting blocks take their best palette and cutting again.
const ROUNDS: usize = 8;

/// A colour in the SFC's 5 bits a channel.
type C5 = [u8; 3];

fn c5(rgb: [u8; 3]) -> C5 {
    [rgb[0] >> 3, rgb[1] >> 3, rgb[2] >> 3]
}

fn dist(a: C5, b: C5) -> u32 {
    // Green counts most, blue least, roughly as the eye weighs them.
    let d = |i: usize, w: i32| w * (a[i] as i32 - b[i] as i32).pow(2);
    (d(0, 3) + d(1, 4) + d(2, 2)) as u32
}

/// Up to `n` colours standing for `px` (with how often each occurs), by
/// median cut: the box of colours spread widest is split at its weighted
/// median, until there are `n` boxes; each gives its weighted mean.
fn median_cut(px: &[(C5, u32)], n: usize) -> Vec<C5> {
    if px.is_empty() {
        return Vec::new();
    }
    let mut boxes: Vec<Vec<(C5, u32)>> = vec![px.to_vec()];
    while boxes.len() < n {
        let spread = |b: &Vec<(C5, u32)>| -> (u32, usize) {
            (0..3)
                .map(|ch| {
                    let (lo, hi) = b.iter().fold((u8::MAX, 0), |(lo, hi), (c, _)| (lo.min(c[ch]), hi.max(c[ch])));
                    let w: u32 = b.iter().map(|(_, k)| k).sum();
                    ((hi - lo) as u32 * (w as f32).sqrt() as u32, ch)
                })
                .max()
                .unwrap()
        };
        let Some((i, (score, ch))) = boxes.iter().enumerate().filter(|(_, b)| b.len() > 1).map(|(i, b)| (i, spread(b))).max_by_key(|(_, s)| s.0) else {
            break;
        };
        if score == 0 {
            break;
        }
        let mut b = boxes.swap_remove(i);
        b.sort_by_key(|(c, _)| c[ch]);
        let total: u32 = b.iter().map(|(_, k)| k).sum();
        let (mut acc, mut cut) = (0, 1);
        for (j, (_, k)) in b.iter().enumerate() {
            acc += k;
            if acc * 2 >= total {
                cut = (j + 1).clamp(1, b.len() - 1);
                break;
            }
        }
        let rest = b.split_off(cut);
        boxes.push(b);
        boxes.push(rest);
    }
    boxes
        .iter()
        .map(|b| {
            let w: u32 = b.iter().map(|(_, k)| k).sum();
            std::array::from_fn(|ch| ((b.iter().map(|(c, k)| c[ch] as u32 * k).sum::<u32>() + w / 2) / w) as u8)
        })
        .collect()
}

/// The colours of some pixels, each with how often it occurs.
fn histogram(px: impl Iterator<Item = C5>) -> Vec<(C5, u32)> {
    let mut counts: std::collections::HashMap<C5, u32> = std::collections::HashMap::new();
    for c in px {
        *counts.entry(c).or_default() += 1;
    }
    let mut v: Vec<(C5, u32)> = counts.into_iter().collect();
    v.sort();
    v
}

/// The entry of `palette` nearest `c` (1-based: 0 is left clear).
fn nearest(palette: &[C5], c: C5) -> (u8, u32) {
    palette.iter().enumerate().map(|(i, &p)| (i as u8 + 1, dist(p, c))).min_by_key(|&(_, d)| d).unwrap_or((1, u32::MAX))
}

fn to_rgb555(palette: &[C5]) -> Vec<Rgb555> {
    let mut v: Vec<Rgb555> = palette.iter().map(|c| Rgb555::new(c[0], c[1], c[2])).collect();
    v.resize(COLORS, Rgb555::BLACK);
    v
}

/// A cut fitted to the PPU: its blocks in VRAM, laid out 16 across and 8
/// down, and the colours of its six palettes.
pub struct Scene {
    cells: Vec<MetatileId>,
    cols: usize,
    rows: usize,
    pub palettes: Vec<Vec<Rgb555>>,
}

impl Scene {
    /// Fits the picture to blocks and palettes and puts the blocks in VRAM.
    pub fn load(picture: &Canvas, bank: &mut MetatileBank) -> Self {
        let (cols, rows) = (picture.w / BLOCK, picture.h / BLOCK);
        let block_px = |b: usize| {
            let (bx, by) = (b % cols * BLOCK, b / cols * BLOCK);
            (0..BLOCK * BLOCK).map(move |i| (bx + i % BLOCK, by + i / BLOCK))
        };
        let colour = |(x, y): (usize, usize)| c5(picture.get(x, y).unwrap_or([0, 0, 0]));
        let hist: Vec<Vec<(C5, u32)>> = (0..cols * rows).map(|b| histogram(block_px(b).map(colour))).collect();
        // Group the blocks by their mean colour: the first group from the
        // first block, each next one from the block farthest from all
        // groups so far, then a few rounds of k-means.
        let mean = |h: &Vec<(C5, u32)>| -> C5 {
            let w: u32 = h.iter().map(|(_, k)| k).sum();
            std::array::from_fn(|ch| (h.iter().map(|(c, k)| c[ch] as u32 * k).sum::<u32>() / w) as u8)
        };
        let means: Vec<C5> = hist.iter().map(mean).collect();
        let mut centres = vec![means[0]];
        while centres.len() < PALETTES {
            let far = (0..means.len()).max_by_key(|&b| centres.iter().map(|&c| dist(means[b], c)).min().unwrap()).unwrap();
            centres.push(means[far]);
        }
        let mut group = vec![0usize; means.len()];
        for _ in 0..8 {
            for (b, m) in means.iter().enumerate() {
                group[b] = (0..PALETTES).min_by_key(|&g| dist(*m, centres[g])).unwrap();
            }
            for (g, centre) in centres.iter_mut().enumerate() {
                let members: Vec<(C5, u32)> = means.iter().zip(&group).filter(|(_, gg)| **gg == g).map(|(m, _)| (*m, 1)).collect();
                if let Some(c) = median_cut(&members, 1).first() {
                    *centre = *c;
                }
            }
        }
        // Each group's palette; then each block takes the palette that
        // draws it best, and the palettes are cut again from their blocks.
        let mut palettes: Vec<Vec<C5>> = Vec::new();
        for round in 0..=ROUNDS {
            palettes = (0..PALETTES)
                .map(|g| {
                    let mut px: Vec<(C5, u32)> = Vec::new();
                    for b in (0..hist.len()).filter(|&b| group[b] == g) {
                        px.extend(&hist[b]);
                    }
                    // Weighed by the square root of how often each colour
                    // occurs, so a few pixels of a colour of their own (the
                    // sky beside a peak) still get one.
                    let weighed: Vec<(C5, u32)> = histogram_merge(px).into_iter().map(|(c, k)| (c, (k as f32).sqrt().ceil() as u32)).collect();
                    median_cut(&weighed, COLORS)
                })
                .collect();
            if round == ROUNDS {
                break;
            }
            for (b, h) in hist.iter().enumerate() {
                let err = |p: &Vec<C5>| -> u64 { h.iter().map(|&(c, k)| nearest(p, c).1 as u64 * k as u64).sum() };
                group[b] = (0..PALETTES).filter(|&g| !palettes[g].is_empty()).min_by_key(|&g| err(&palettes[g])).unwrap_or(group[b]);
            }
        }
        let cells = (0..cols * rows)
            .map(|b| {
                let p = &palettes[group[b]];
                let mut art = PixelArt::filled(BLOCK, BLOCK, 0);
                for (i, xy) in block_px(b).enumerate() {
                    art.px[i] = nearest(p, colour(xy)).0;
                }
                bank.add_art(FIRST_PALETTE + group[b] as u8, &art)
            })
            .collect();
        Self { cells, cols, rows, palettes: palettes.iter().map(|p| to_rgb555(p)).collect() }
    }

    /// Takes its blocks out of VRAM.
    pub fn unload(&self, bank: &mut MetatileBank) {
        for &id in &self.cells {
            bank.remove(id);
        }
    }
}

/// Merges counts of the same colour (from several blocks' histograms).
fn histogram_merge(px: Vec<(C5, u32)>) -> Vec<(C5, u32)> {
    let mut counts: std::collections::HashMap<C5, u32> = std::collections::HashMap::new();
    for (c, k) in px {
        *counts.entry(c).or_default() += k;
    }
    let mut v: Vec<(C5, u32)> = counts.into_iter().collect();
    v.sort();
    v
}

impl BgLayer for Scene {
    fn cell(&self, mx: i32, my: i32) -> BgCell {
        if mx < 0 || my < 0 || mx as usize >= self.cols || my as usize >= self.rows {
            return BgCell::EMPTY;
        }
        BgCell::new(self.cells[my as usize * self.cols + mx as usize])
    }
}

/// A layer with nothing on it (BG2 while a cut shows).
pub struct NoLayer;

impl BgLayer for NoLayer {
    fn cell(&self, _: i32, _: i32) -> BgCell {
        BgCell::EMPTY
    }
}

/// A picture on a see-through ground fitted to sprites: cut into 64x64
/// pieces (those with nothing in them left out), all in one palette.
pub struct SpriteSet {
    /// Each piece and where it goes from the picture's top-left.
    pub pieces: Vec<(SpriteId, i32, i32)>,
    pub palette: Vec<Rgb555>,
}

impl SpriteSet {
    pub const PIECE: usize = 64;

    pub fn load(picture: &Canvas, sprites: &mut SpriteBank) -> Self {
        let palette = median_cut(&histogram(picture.px.iter().flatten().map(|&c| c5(c))), COLORS);
        let mut pieces = Vec::new();
        for py in (0..picture.h).step_by(Self::PIECE) {
            for px in (0..picture.w).step_by(Self::PIECE) {
                let mut art = PixelArt::filled(Self::PIECE, Self::PIECE, 0);
                let mut any = false;
                for y in 0..Self::PIECE.min(picture.h - py) {
                    for x in 0..Self::PIECE.min(picture.w - px) {
                        if let Some(c) = picture.get(px + x, py + y) {
                            art.set(x, y, nearest(&palette, c5(c)).0);
                            any = true;
                        }
                    }
                }
                if any {
                    pieces.push((sprites.add(&art), px as i32, py as i32));
                }
            }
        }
        Self { pieces, palette: to_rgb555(&palette) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn median_cut_keeps_few_colours_as_they_are() {
        let px = [([1, 2, 3], 5), ([20, 20, 20], 1)];
        let mut got = median_cut(&px, 15);
        got.sort();
        assert_eq!(got, vec![[1, 2, 3], [20, 20, 20]]);
    }

    #[test]
    fn median_cut_gives_no_more_than_asked() {
        let px: Vec<(C5, u32)> = (0..32).map(|i| ([i, 31 - i, i / 2], 1)).collect();
        assert_eq!(median_cut(&px, 4).len(), 4);
    }

    #[test]
    fn a_cut_fits_six_palettes_of_fifteen_in_its_own_blocks() {
        let mut bank = MetatileBank::default();
        let scene = Scene::load(&crate::assets::ending_art::island_from_afar(), &mut bank);
        assert_eq!(scene.palettes.len(), PALETTES);
        assert!(scene.palettes.iter().all(|p| p.len() == COLORS));
        assert_eq!(bank.len(), 16 * 8, "a block for each 16x16 of the picture");
        scene.unload(&mut bank);
        assert_eq!(bank.len(), 0, "and they go once it's done");
    }
}

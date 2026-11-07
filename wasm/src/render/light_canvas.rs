//! The night's light map. Every light on screen — a torch carried, each
//! campfire — is laid each frame onto a screen-sized patch of the BG3 map,
//! one copy of the radial light mask per light, on the world's 8px tile
//! grid. BG3 is then scrolled by the camera's fine offset, so the map moves
//! with the world: a campfire's light stays put on it, while a carried
//! torch's light steps in 8px with the castaway.
//!
//! Where two lights overlap a cell, a tile taking the brighter of the two at
//! every pixel is drawn into one of a few spare tiles (as a game would DMA
//! them into VRAM during the blank), up to `SCRATCH_TILES` a frame.

use crate::assets::HudTiles;
use crate::assets::hud_art::LIGHT_MASK_TILES;
use crate::assets::palettes::HUD_LIGHT_MASK;
use crate::ppu::banks::{HudTileBank, HudTileId};
use crate::ppu::pattern::PixelArt;
use crate::ppu::{Bg3Layer, SCREEN_H, SCREEN_W};

/// Top-left cell of the light map in the BG3 map: below the HUD and bag
/// screen, and far enough down that no visible line wraps onto them.
pub const ORIGIN: (usize, usize) = (0, 32);
/// Cells laid: a screen's worth plus one for the fine scroll.
const COLS: usize = SCREEN_W / 8 + 1;
const ROWS: usize = SCREEN_H / 8 + 1;
/// Spare tiles for overlaps, reused every frame.
pub const SCRATCH_TILES: usize = 96;

const MASK_N: i32 = LIGHT_MASK_TILES as i32;

/// BG3 scroll that shows the light map under a camera at `camera`.
pub fn scroll(camera: (i32, i32)) -> (i32, i32) {
    ((ORIGIN.0 * 8) as i32 + camera.0.rem_euclid(8), (ORIGIN.1 * 8) as i32 + camera.1.rem_euclid(8))
}

/// Lays the lights (centres in world px) for a camera at `camera`, writing
/// overlap tiles into `bank`. Returns whether any light falls on screen.
pub fn lay(bg3: &mut Bg3Layer, bank: &mut HudTileBank, hud: &HudTiles, camera: (i32, i32), lights: &[(i32, i32)]) -> bool {
    bg3.clear_rect(ORIGIN.0, ORIGIN.1, COLS, ROWS);
    let cam = (camera.0.div_euclid(8), camera.1.div_euclid(8));
    // Mask tiles falling in each cell.
    let mut cells: Vec<Vec<u16>> = vec![Vec::new(); COLS * ROWS];
    let mut any = false;
    for &(x, y) in lights {
        // The mask's top-left cell, the centre rounded to the nearest one.
        let (tx, ty) = ((x + 4).div_euclid(8) - MASK_N / 2, (y + 4).div_euclid(8) - MASK_N / 2);
        for my in 0..MASK_N {
            for mx in 0..MASK_N {
                let (c, r) = (tx + mx - cam.0, ty + my - cam.1);
                let i = (my * MASK_N + mx) as usize;
                if !(0..COLS as i32).contains(&c) || !(0..ROWS as i32).contains(&r) || hud.light_mask_empty[i] {
                    continue;
                }
                cells[r as usize * COLS + c as usize].push(i as u16);
                any = true;
            }
        }
    }
    let mut made: Vec<(Vec<u16>, HudTileId)> = Vec::new();
    for (k, parts) in cells.iter_mut().enumerate() {
        let tile = match parts.len() {
            0 => continue,
            1 => hud.light_mask[parts[0] as usize],
            _ => {
                parts.sort_unstable();
                parts.dedup();
                overlap(bank, hud, parts, &mut made)
            }
        };
        bg3.put(ORIGIN.0 + k % COLS, ORIGIN.1 + k / COLS, tile, HUD_LIGHT_MASK);
    }
    any
}

/// The tile for a cell where several mask tiles fall: one already solid
/// light needs nothing else; otherwise their combination, made once a frame
/// into a spare tile (the first part alone if none are left).
fn overlap(bank: &mut HudTileBank, hud: &HudTiles, parts: &[u16], made: &mut Vec<(Vec<u16>, HudTileId)>) -> HudTileId {
    if let Some(&solid) = parts.iter().find(|&&i| hud.light_mask_solid[i as usize]) {
        return hud.light_mask[solid as usize];
    }
    if let Some((_, id)) = made.iter().find(|(p, _)| p == parts) {
        return *id;
    }
    let Some(&id) = hud.light_scratch.get(made.len()) else {
        return hud.light_mask[parts[0] as usize];
    };
    let mut art = PixelArt::filled(8, 8, 0);
    for (n, px) in art.px.iter_mut().enumerate() {
        // Mask levels: 1 is brightest, 3 dimmest, 0 none at all.
        let dark = |v: u8| if v == 0 { 4 } else { v };
        let best = parts.iter().map(|&i| hud.light_mask_art[i as usize].px[n]).min_by_key(|&v| dark(v)).unwrap_or(0);
        *px = best;
    }
    bank.replace(id, &art);
    made.push((parts.to_vec(), id));
    id
}

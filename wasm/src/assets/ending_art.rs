//! The rescue ending's cuts, painted in full colour (then fitted to the
//! SFC's palettes and tiles by `render::scene`): the island from afar with
//! its smoke; the view out of a passing plane's cockpit, the island far off
//! to the right; the plane turning toward the island. Each is a 256x128
//! picture, letterboxed in the middle of the screen. The plane of the third
//! is painted apart, to be drawn over it as sprites.
//!
//! Painted from noise, ramps and ordered dither (an SFC look: few colours,
//! dithered between), lit from the upper left.

use std::f32::consts::TAU;

/// The size of a cut.
pub const CUT_W: usize = 256;
pub const CUT_H: usize = 128;

pub type Rgb = [u8; 3];

/// A picture in full colour; `None` is see-through (for sprites).
#[derive(Clone, Debug)]
pub struct Canvas {
    pub w: usize,
    pub h: usize,
    pub px: Vec<Option<Rgb>>,
}

impl Canvas {
    pub fn new(w: usize, h: usize) -> Self {
        Self { w, h, px: vec![None; w * h] }
    }

    fn put(&mut self, x: f32, y: f32, c: Rgb) {
        let (x, y) = (x.floor() as i32, y.floor() as i32);
        if x >= 0 && y >= 0 && (x as usize) < self.w && (y as usize) < self.h {
            self.px[y as usize * self.w + x as usize] = Some(c);
        }
    }

    pub fn get(&self, x: usize, y: usize) -> Option<Rgb> {
        self.px[y * self.w + x]
    }
}

const fn hex(v: u32) -> Rgb {
    [(v >> 16) as u8, (v >> 8) as u8, v as u8]
}

const SKY: [Rgb; 7] = [hex(0x3a64b0), hex(0x4a7cc4), hex(0x6094d4), hex(0x7cacdc), hex(0x98c0e4), hex(0xb4d2ea), hex(0xcadfec)];
const SEA: [Rgb; 7] = [hex(0x7aa4c8), hex(0x5c8cc0), hex(0x4878b0), hex(0x3a68a4), hex(0x2e5894), hex(0x244a84), hex(0x1c3c70)];
const HAZE: Rgb = hex(0xb8d0e4);
const WAVE: Rgb = hex(0xa8cce4);
const GLINT: Rgb = hex(0xeef8ff);
const FOREST: [Rgb; 5] = [hex(0x1e3a20), hex(0x2a4e28), hex(0x3a6430), hex(0x4c7a38), hex(0x62903e)];
const ROCK: [Rgb; 4] = [hex(0x4a4038), hex(0x665a4e), hex(0x847664), hex(0xa29482)];
const SAND: [Rgb; 3] = [hex(0xb49c74), hex(0xcfba8c), hex(0xe4d4a8)];
const SURF: Rgb = hex(0xf0f6fa);
const CLOUD: [Rgb; 5] = [hex(0x9eb2cc), hex(0xbccadc), hex(0xd8e2ec), hex(0xeef3f8), hex(0xfbfdff)];
const SMOKE: [Rgb; 5] = [hex(0x8c8c8c), hex(0xa8a8a6), hex(0xc2c2be), hex(0xd8d8d4), hex(0xecece8)];
const DARK: [Rgb; 5] = [hex(0x0b0d10), hex(0x15181c), hex(0x1f2329), hex(0x2b3037), hex(0x3a4048)];
const GAUGE: [Rgb; 4] = [hex(0x07080a), hex(0x2c3034), hex(0x9aa2a8), hex(0xdfe4e6)];
const RADIO: [Rgb; 4] = [hex(0x1a1c1a), hex(0x3a3e38), hex(0xc8b070), hex(0x7ce07c)];
const WHITE: [Rgb; 5] = [hex(0x7c828c), hex(0xa2a8b0), hex(0xc8ccd0), hex(0xe4e6e8), hex(0xf8f8f6)];
const STRIPE: [Rgb; 3] = [hex(0x5a1a18), hex(0x8c2a24), hex(0xb8402e)];
const GLASS: [Rgb; 4] = [hex(0x202c3c), hex(0x34475e), hex(0x5a7088), hex(0x8ca4b8)];
const TYRE: Rgb = hex(0x121418);

const BAYER: [[u8; 4]; 4] = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];

fn bay(x: i32, y: i32) -> f32 {
    (BAYER[y.rem_euclid(4) as usize][x.rem_euclid(4) as usize] as f32 + 0.5) / 16.0
}

fn h2(x: i64, y: i64, s: i64) -> f32 {
    let n = (x.wrapping_mul(374_761_393).wrapping_add(y.wrapping_mul(668_265_263)).wrapping_add(s.wrapping_mul(982_451_653))) as u32;
    let n = (n ^ (n >> 13)).wrapping_mul(1_274_126_177);
    (n & 0xffff) as f32 / 65535.0
}

fn vnoise(x: f32, y: f32, s: i64) -> f32 {
    let (xi, yi) = (x.floor(), y.floor());
    let (xf, yf) = (x - xi, y - yi);
    let (u, v) = (xf * xf * (3.0 - 2.0 * xf), yf * yf * (3.0 - 2.0 * yf));
    let (xi, yi) = (xi as i64, yi as i64);
    let (a, b, c, d) = (h2(xi, yi, s), h2(xi + 1, yi, s), h2(xi, yi + 1, s), h2(xi + 1, yi + 1, s));
    a + (b - a) * u + (c - a) * v + (a - b - c + d) * u * v
}

fn fbm(x: f32, y: f32, s: i64) -> f32 {
    let (mut t, mut amp, mut f, mut norm) = (0.0, 1.0, 1.0, 0.0);
    for i in 0..3 {
        t += vnoise(x * f, y * f, s + i) * amp;
        norm += amp;
        amp *= 0.5;
        f *= 2.0;
    }
    t / norm
}

fn lerp(a: Rgb, b: Rgb, t: f32) -> Rgb {
    std::array::from_fn(|i| (a[i] as f32 + (b[i] as f32 - a[i] as f32) * t).round() as u8)
}

/// A colour from a ramp, dithered between its steps.
fn ramp(cols: &[Rgb], t: f32, x: i32, y: i32) -> Rgb {
    let t = t.clamp(0.0, 0.9999) * (cols.len() - 1) as f32;
    let i = t as usize;
    if bay(x, y) < t - i as f32 { cols[i + 1] } else { cols[i] }
}

/// Fills a polygon (pixel centres inside), coloured by `shade(x, y)`.
fn fill_poly(c: &mut Canvas, pts: &[(f32, f32)], mut shade: impl FnMut(i32, i32) -> Option<Rgb>) {
    let (y0, y1) = pts.iter().fold((f32::MAX, f32::MIN), |(a, b), p| (a.min(p.1), b.max(p.1)));
    let (x0, x1) = pts.iter().fold((f32::MAX, f32::MIN), |(a, b), p| (a.min(p.0), b.max(p.0)));
    for y in y0 as i32..=y1 as i32 {
        for x in x0 as i32..=x1 as i32 {
            let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
            let mut inside = false;
            for k in 0..pts.len() {
                let ((xa, ya), (xb, yb)) = (pts[k], pts[(k + pts.len() - 1) % pts.len()]);
                if (ya > py) != (yb > py) && px < (xb - xa) * (py - ya) / (yb - ya) + xa {
                    inside = !inside;
                }
            }
            if inside && let Some(col) = shade(x, y) {
                c.put(x as f32, y as f32, col);
            }
        }
    }
}

/// Sky over the sea, the horizon at `hz`: the sky deepening upward, the
/// sea darkening toward us with streaks of waves.
fn backdrop(c: &mut Canvas, hz: i32) {
    let h = c.h as i32;
    for y in 0..h {
        for x in 0..c.w as i32 {
            let col = if y < hz {
                ramp(&SKY, (y as f32 / hz as f32).powf(1.3), x, y)
            } else {
                let d = (y - hz) as f32 / (h - hz) as f32;
                let mut col = ramp(&SEA, d.powf(0.7), x, y);
                let n = fbm(x as f32 / (4.0 + d * 10.0), (y - hz) as f32 / (0.7 + d * 1.6), 7);
                if n > 0.7 {
                    col = lerp(col, WAVE, 0.35 + d * 0.4);
                } else if n < 0.32 {
                    col = lerp(col, SEA[6], 0.25 + d * 0.2);
                }
                col
            };
            c.put(x as f32, y as f32, col);
        }
    }
    for x in 0..c.w as i32 {
        c.put(x as f32, hz as f32, lerp(SKY[6], SEA[0], 0.4));
        c.put(x as f32, (hz - 1) as f32, HAZE);
    }
}

/// The sun glinting on the sea, in a band widening toward us.
fn sun_glints(c: &mut Canvas, hz: i32, cx: f32, w: f32) {
    let h = c.h as i32;
    for y in hz + 1..h {
        let d = (y - hz) as f32 / (h - hz) as f32;
        let half = w * (1.0 + d * 2.0);
        for x in (cx - half) as i32..(cx + half) as i32 {
            let k = 1.0 - (x as f32 - cx).abs() / half;
            if h2((x.div_euclid(3)) as i64, y as i64, 3) > 0.9 - 0.06 * k && fbm(x as f32 / 3.0, y as f32, 7) > 0.55 {
                c.put(x as f32, y as f32, lerp(GLINT, SEA[0], 0.2 * d));
            }
        }
    }
}

/// A cumulus: a heaped, lumpy top lit from the upper left, a flat base.
fn cumulus(c: &mut Canvas, cx: f32, cy: f32, w: f32, h: f32, seed: i64) {
    for y in (cy - h) as i32..(cy + h * 0.35) as i32 {
        for x in (cx - w) as i32..(cx + w) as i32 {
            let (nx, ny) = ((x as f32 - cx) / w, (y as f32 - cy) / h);
            if nx.abs() > 1.0 || ny > 0.3 {
                continue;
            }
            let top = -(1.0 - nx * nx).powf(0.6) * (0.75 + 0.45 * fbm(x as f32 / 7.0, 0.0, seed));
            if ny < top {
                continue;
            }
            let edge = fbm(x as f32 / 5.0, y as f32 / 5.0, seed + 9);
            if ny - top < 0.12 && edge < 0.45 {
                continue;
            }
            let light = 0.65 - ny * 0.9 - nx * 0.35 + (edge - 0.5) * 0.6;
            c.put(x as f32, y as f32, ramp(&CLOUD, light, x, y));
        }
    }
}

/// A volcanic island: a cone with concave flanks and a broken summit,
/// forested lower slopes, rocky heights, a beach and surf; `haze` blends it
/// toward the sky for distance.
fn island(c: &mut Canvas, cx: f32, base: f32, s: f32, haze: f32) {
    let height = |x: f32| {
        let dx = (x - (cx - 12.0 * s)) / (60.0 * s);
        let mut cone = if dx.abs() < 1.2 { 34.0 * s * (-dx.abs() * 3.2).exp() } else { 0.0 };
        if cone > 0.0 {
            cone = cone.min(29.0 * s + fbm(x / (2.0 * s), 1.0, 3) * 2.0 * s);
        }
        let hill = 13.0 * s * (1.0 - ((x - (cx + 18.0 * s)) / (34.0 * s)).powi(2)).max(0.0);
        let shore = 5.0 * s * (1.0 - ((x - cx) / (56.0 * s)).powi(2)).max(0.0).sqrt();
        cone.max(hill).max(shore) + fbm(x / (4.0 * s), 0.0, 2) * 2.0 * s * if cone > 3.0 * s { 1.0 } else { 0.5 }
    };
    let hazed = |col: Rgb, k: f32| if haze > 0.0 { lerp(col, HAZE, k) } else { col };
    let (left, right) = ((cx - 58.0 * s) as i32, (cx + 56.0 * s) as i32);
    for x in left..right {
        let xf = x as f32;
        let hh = height(xf);
        let top = base - hh;
        for y in top as i32..base as i32 {
            let yf = y as f32;
            let depth = (yf - top) / hh.max(1.0);
            let alt = (base - yf) / (34.0 * s);
            let slope = height(xf + 1.0) - height(xf - 1.0);
            let lit = 0.5 + slope * 0.35 / s.max(0.3);
            let col = if alt > 0.55 + fbm(xf / 6.0, yf / 6.0, 4) * 0.2 {
                ramp(&ROCK, lit * 0.8 + (1.0 - depth) * 0.2 + fbm(xf / 3.0, yf / 2.0, 5) * 0.2, x, y)
            } else {
                let tex = fbm(xf / (2.2 * s), yf / (1.6 * s), 6);
                ramp(&FOREST, lit * 0.55 + tex * 0.5 - depth * 0.15, x, y)
            };
            c.put(xf, yf, hazed(col, haze));
        }
    }
    let base_i = base as i32;
    for x in left + (4.0 * s) as i32..right - (4.0 * s) as i32 {
        let w = 1 + (1.5 * s * (0.5 + fbm(x as f32 / 9.0, 0.0, 8))) as i32;
        for k in 0..w {
            let col = ramp(&SAND, 0.8 - k as f32 / w as f32 * 0.6, x, base_i + k);
            c.put(x as f32, (base_i + k) as f32, hazed(col, haze));
        }
        if h2(x as i64, 1, 9) < 0.7 {
            c.put(x as f32, (base_i + w) as f32, hazed(SURF, haze * 0.5));
        }
        if h2(x as i64, 2, 9) < 0.3 {
            c.put((x + 1) as f32, (base_i + w + 1) as f32, lerp(SURF, SEA[1], 0.4));
        }
    }
}

/// A column of smoke from `(x0, y0)` up to `top`: dense and narrow where it
/// rises, billowing out, drifting `lean` with the wind and thinning away.
#[allow(clippy::too_many_arguments)]
fn smoke(c: &mut Canvas, x0: f32, y0: f32, top: f32, lean: f32, w0: f32, w1: f32, seed: i64, haze: f32) {
    let length = y0 - top;
    for y in (top - w1) as i32..(y0 + 2.0) as i32 {
        let t = (y0 - y as f32) / length;
        if t < -0.02 {
            continue;
        }
        let tt = t.max(0.0);
        let cx = x0 + lean * tt.powf(1.7) + (tt * 7.0 + seed as f32).sin() * 2.0 * tt;
        let r = w0 + (w1 - w0) * tt.powf(1.1);
        for x in (cx - r * 1.4) as i32 - 2..(cx + r * 1.4) as i32 + 3 {
            let nx = (x as f32 - cx) / r;
            let billow = fbm(x as f32 / 4.5, y as f32 / 4.5 + seed as f32, seed + 11);
            let edge = 1.0 - nx.abs() + (billow - 0.5) * 0.9;
            if edge < 0.0 {
                continue;
            }
            let fade = ((tt - 0.65) / 0.35).max(0.0);
            if bay(x, y) < fade * 1.1 - edge * 0.4 {
                continue;
            }
            let light = 0.55 - nx * 0.3 + (billow - 0.5) * 0.8 + edge * 0.2 - tt * 0.1;
            let col = ramp(&SMOKE, light, x, y);
            c.put(x as f32, y as f32, if haze > 0.0 { lerp(col, HAZE, haze) } else { col });
        }
    }
}

/// Cut 1: the island from afar, a column of smoke rising off it.
pub fn island_from_afar() -> Canvas {
    let mut c = Canvas::new(CUT_W, CUT_H);
    let hz = 84;
    backdrop(&mut c, hz);
    cumulus(&mut c, 46.0, 30.0, 34.0, 16.0, 1);
    cumulus(&mut c, 206.0, 44.0, 26.0, 11.0, 2);
    cumulus(&mut c, 150.0, 20.0, 14.0, 6.0, 3);
    sun_glints(&mut c, hz, 60.0, 10.0);
    island(&mut c, 128.0, (hz + 1) as f32, 1.15, 0.18);
    smoke(&mut c, 150.0, (hz - 5) as f32, 4.0, 54.0, 1.3, 15.0, 1, 0.08);
    c
}

/// Cut 2: the pilot's view out of the windshield of a passing plane: open
/// sky and sea, the island small and far off to the right with its smoke,
/// the top of the panel along the bottom.
pub fn cockpit_view() -> Canvas {
    let mut c = Canvas::new(CUT_W, CUT_H);
    let (hz, h) = (72, CUT_H as f32);
    backdrop(&mut c, hz);
    cumulus(&mut c, 120.0, 30.0, 22.0, 7.0, 4);
    cumulus(&mut c, 66.0, 18.0, 14.0, 5.0, 5);
    sun_glints(&mut c, hz, 110.0, 9.0);
    island(&mut c, 226.0, (hz + 1) as f32, 0.34, 0.34);
    smoke(&mut c, 232.0, (hz - 3) as f32, 36.0, 8.0, 0.9, 6.0, 2, 0.18);
    // the glare shield, a low arch along the bottom
    let shield = |x: f32| 106.0 + ((x - 150.0) / 130.0).powi(2) * 6.0;
    let mut pts = vec![(0.0, shield(0.0) + 4.0)];
    pts.extend((0..=256).step_by(8).map(|x| (x as f32, shield(x as f32))));
    pts.extend([(256.0, h), (0.0, h)]);
    fill_poly(&mut c, &pts, |x, y| {
        let (xf, yf) = (x as f32, y as f32);
        let lit = if yf < shield(xf) + 5.0 { 0.75 } else { 0.4 };
        Some(ramp(&DARK, lit - (yf - shield(xf)) / 90.0 + fbm(xf / 4.0, yf / 4.0, 61) * 0.12, x, y))
    });
    // the gauges just showing under it: the flight instruments (the
    // attitude indicator third), small engine gauges, the radio stack
    for k in 0..6 {
        let (gx, gy, r) = (22.0 + k as f32 * 15.0, 124.0, 6.2_f32);
        for y in (gy - r) as i32 - 1..CUT_H as i32 {
            for x in (gx - r) as i32 - 1..(gx + r) as i32 + 2 {
                let d = (x as f32 - gx).hypot(y as f32 - gy);
                if d >= r {
                    continue;
                }
                let col = if d > r - 1.1 {
                    GAUGE[1]
                } else if k == 2 {
                    if (y as f32) < gy + (x as f32 - gx) * 0.1 { hex(0x4a7cc0) } else { hex(0x7a5432) }
                } else {
                    ramp(&GAUGE, 0.15 + (1.0 - d / r) * 0.2, x, y)
                };
                c.put(x as f32, y as f32, col);
            }
        }
        if k != 2 {
            let a = 3.6 + k as f32 * 0.7;
            for t in 0..5 {
                c.put(gx + a.cos() * t as f32, gy + a.sin() * t as f32, GAUGE[3]);
            }
        }
    }
    for k in 0..4 {
        let (gx, gy, r) = (118.0 + k as f32 * 11.0, 125.0, 4.4_f32);
        for y in (gy - r) as i32 - 1..CUT_H as i32 {
            for x in (gx - r) as i32 - 1..(gx + r) as i32 + 2 {
                let d = (x as f32 - gx).hypot(y as f32 - gy);
                if d < r {
                    c.put(x as f32, y as f32, if d > r - 1.0 { GAUGE[1] } else { GAUGE[0] });
                }
            }
        }
        for t in 0..4 {
            c.put(gx - t as f32 * 0.7, gy - t as f32 * 0.7, GAUGE[3]);
        }
    }
    for k in 0..2 {
        let y0 = 117.0 + k as f32 * 7.0;
        fill_poly(&mut c, &[(168.0, y0), (240.0, y0), (240.0, y0 + 5.0), (168.0, y0 + 5.0)], |x, y| {
            Some(ramp(&RADIO[..2], 0.4 + (y as f32 - y0) / 12.0, x, y))
        });
        let lamp = |on: bool| if on { RADIO[3] } else { RADIO[2] };
        for x in 174..186 {
            c.put(x as f32, y0 + 2.0, lamp(k == 1));
        }
        for x in 198..206 {
            c.put(x as f32, y0 + 2.0, RADIO[2]);
        }
        for x in 220..232 {
            c.put(x as f32, y0 + 2.0, lamp(k == 0));
        }
    }
    c
}

/// Cut 3's sky, sea and island (the plane is drawn over it).
pub fn turning_for_the_island() -> Canvas {
    let mut c = Canvas::new(CUT_W, CUT_H);
    let hz = 96;
    backdrop(&mut c, hz);
    cumulus(&mut c, 40.0, 26.0, 30.0, 12.0, 6);
    cumulus(&mut c, 132.0, 14.0, 18.0, 6.0, 7);
    cumulus(&mut c, 226.0, 58.0, 22.0, 8.0, 8);
    sun_glints(&mut c, hz, 80.0, 12.0);
    island(&mut c, 206.0, (hz + 1) as f32, 0.78, 0.24);
    smoke(&mut c, 222.0, (hz - 6) as f32, 6.0, 34.0, 0.9, 10.0, 3, 0.12);
    c
}

/// The plane's size as painted apart (two 64x64 sprites side by side), and
/// where in it the plane's middle is.
pub const PLANE_W: usize = 128;
pub const PLANE_H: usize = 64;
pub const PLANE_MIDDLE: (f32, f32) = (78.0, 32.0);

/// A high-winged light plane side on, flying right (its left side toward
/// us), the nose dipped a little as it banks toward the island: white with a
/// red stripe, the cabin's windows, the wing on its strut, the tail, the
/// gear and the blur of the propeller. On a see-through canvas.
pub fn plane() -> Canvas {
    let mut c = Canvas::new(PLANE_W, PLANE_H);
    let (cx, cy, s, bank) = (PLANE_MIDDLE.0, PLANE_MIDDLE.1, 2.1_f32, 0.12_f32);
    let (ca, sa) = (bank.cos(), bank.sin());
    let to = move |u: f32, v: f32| (cx + (u * ca - v * sa) * s, cy + (u * sa + v * ca) * s);
    let back = move |x: i32, y: i32| {
        let (dx, dy) = ((x as f32 + 0.5 - cx) / s, (y as f32 + 0.5 - cy) / s);
        (dx * ca + dy * sa, -dx * sa + dy * ca)
    };
    let poly = |c: &mut Canvas, pts: &[(f32, f32)], shade: &dyn Fn(f32, f32, i32, i32) -> Rgb| {
        let p: Vec<(f32, f32)> = pts.iter().map(|&(u, v)| to(u, v)).collect();
        fill_poly(c, &p, |x, y| {
            let (u, v) = back(x, y);
            Some(shade(u, v, x, y))
        });
    };
    let line = |c: &mut Canvas, (u0, v0): (f32, f32), (u1, v1): (f32, f32), col: Rgb, w: i32| {
        let n = ((u1 - u0).abs().max((v1 - v0).abs()) * s * 2.0) as i32 + 1;
        for k in 0..=n {
            let t = k as f32 / n as f32;
            let (x, y) = to(u0 + (u1 - u0) * t, v0 + (v1 - v0) * t);
            for d in 0..w {
                c.put(x, y + d as f32, col);
            }
        }
    };
    // fin and tailplane (behind the fuselage)
    poly(&mut c, &[(-25.0, -2.6), (-31.5, -11.5), (-35.0, -11.5), (-36.0, -1.2)], &|_, v, x, y| {
        if v < -9.6 { ramp(&STRIPE, 0.65, x, y) } else { ramp(&WHITE, 0.8 - (v + 11.0) * 0.02, x, y) }
    });
    poly(&mut c, &[(-27.0, -0.6), (-36.5, -0.8), (-37.0, 0.6), (-27.0, 0.8)], &|_, _, x, y| ramp(&WHITE, 0.45, x, y));
    // fuselage: cowl, windshield, cabin, the back window sloping down into
    // the tailcone; the belly
    let outline = [
        (21.0, -1.2),
        (19.5, -3.1),
        (12.5, -3.4),
        (8.0, -6.2),
        (-4.0, -6.2),
        (-11.0, -3.6),
        (-36.0, -1.6),
        (-36.0, 0.3),
        (-12.0, 2.4),
        (-2.0, 3.6),
        (10.0, 3.6),
        (18.0, 2.4),
        (21.0, 0.8),
    ];
    poly(&mut c, &outline, &|u, v, x, y| {
        let r = (v + 6.2) / 9.8;
        let mut col = ramp(&WHITE, 0.95 - r * 0.7, x, y);
        if (v - (0.2 + (-u - 12.0).max(0.0) * -0.05)).abs() < 0.45 && u < 18.0 {
            col = ramp(&STRIPE, 0.7, x, y);
        }
        let window =
            (-4.0 < u && u < 9.5 && -5.4 < v && v < -2.6 && !(2.4 < u && u < 3.2)) || (-9.0 < u && u < -4.6 && v > -5.2 + (-4.6 - u) * 0.45 && v < -2.8);
        if window {
            col = ramp(&GLASS, 0.85 - (v + 5.4) * 0.22, x, y);
        }
        if 12.6 < u && u < 18.0 && v < -1.4 && v > -3.2 {
            col = ramp(&WHITE, 0.55, x, y);
        }
        col
    });
    poly(&mut c, &[(12.5, -3.4), (8.0, -6.2), (7.2, -5.6), (11.4, -3.2)], &|_, _, x, y| ramp(&GLASS, 0.55, x, y));
    // spinner and the blur of the propeller
    poly(&mut c, &[(21.0, -1.2), (23.4, -0.3), (23.4, 0.4), (21.0, 0.8)], &|_, _, x, y| ramp(&WHITE, 0.6, x, y));
    for k in -90_i32..=90 {
        let (x, y) = to(23.6 + k.abs() as f32 * 0.006, k as f32 / 10.0);
        if h2(x as i64, y as i64, 31) < 0.4 + 0.3 * (1.0 - k.abs() as f32 / 90.0) {
            c.put(x, y, ramp(&WHITE, 0.3, x as i32, y as i32));
        }
    }
    // gear: main legs and wheels, nose leg and wheel
    line(&mut c, (1.0, 3.4), (-1.5, 7.6), WHITE[1], 2);
    line(&mut c, (16.0, 2.6), (16.4, 6.6), WHITE[1], 1);
    for (u, v, r) in [(-1.8, 8.2, 1.6_f32), (16.5, 7.0, 1.2)] {
        for a in 0..60 {
            for rr in [r, r * 0.7, r * 0.4] {
                let ang = a as f32 / 60.0 * TAU;
                let (x, y) = to(u + ang.cos() * rr, v + ang.sin() * rr);
                c.put(x, y, if rr > r * 0.5 { TYRE } else { WHITE[1] });
            }
        }
    }
    // the wing above the cabin, nearly edge on, its underside showing a
    // little as the plane banks; and its strut
    poly(&mut c, &[(-5.5, -7.6), (10.0, -7.6), (10.8, -6.9), (10.0, -6.2), (-5.5, -6.2)], &|_, v, x, y| ramp(&WHITE, if v < -7.0 { 0.9 } else { 0.45 }, x, y));
    poly(&mut c, &[(-5.5, -6.2), (10.0, -6.2), (9.4, -5.6), (-5.0, -5.6)], &|_, _, x, y| ramp(&WHITE, 0.25, x, y));
    line(&mut c, (7.5, -5.8), (2.5, 2.2), WHITE[1], 1);
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cuts_fill_their_frame() {
        for cut in [island_from_afar(), cockpit_view(), turning_for_the_island()] {
            assert_eq!((cut.w, cut.h), (CUT_W, CUT_H));
            assert!(cut.px.iter().all(Option::is_some), "a cut has no holes");
        }
    }

    #[test]
    fn the_plane_stands_on_a_see_through_ground() {
        let p = plane();
        assert!(p.px.iter().any(Option::is_none) && p.px.iter().any(Option::is_some));
        assert!(p.get(0, 0).is_none(), "its corners are clear");
    }
}

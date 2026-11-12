//! Rasterises a pose + outfit into a 24x32 indexed sprite.
//!
//! Each body part is drawn as a shaded shape (limbs as capsules, torso as a
//! tapered cylinder, head as a sphere) with a z value; clothing is decided
//! per pixel by which part of a limb it covers. Afterwards a selective
//! outline pass darkens edges where a part overlaps one behind it and puts a
//! dark rim around the silhouette.

use super::pose::{Limb, P, Pose, View};
use super::{CHAR_H, CHAR_W, Footwear, Headwear, Legwear, Outfit, Sleeve, Top, slot};
use crate::ppu::pattern::PixelArt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mat {
    Skin,
    Hair,
    Top,
    Bottom,
    Accent,
}

impl Mat {
    fn ramp(self) -> [u8; 3] {
        match self {
            Mat::Skin => slot::SKIN,
            Mat::Hair => slot::HAIR,
            Mat::Top => slot::TOP,
            Mat::Bottom => slot::BOTTOM,
            Mat::Accent => [slot::ACCENT[0], slot::ACCENT[1], slot::ACCENT[1]],
        }
    }
}

// Alias: inside part functions `group` is also a parameter name.
use group as group_ids;

/// Body-part groups. Outline lines are drawn where different groups overlap.
mod group {
    pub const LEG_A: u8 = 1;
    pub const LEG_B: u8 = 2;
    pub const HIPS: u8 = 3;
    pub const TORSO: u8 = 4;
    pub const ARM_A: u8 = 5;
    pub const ARM_B: u8 = 6;
    pub const HEAD: u8 = 7;
    pub const HAIR: u8 = 8;
    pub const HAT: u8 = 9;
}

#[derive(Clone, Copy, Default)]
struct Px {
    idx: u8,
    mat: Option<Mat>,
    group: u8,
    z: i32,
    hand: bool,
}

struct Canvas {
    px: Vec<Px>,
}

impl Canvas {
    fn new() -> Self {
        Self { px: vec![Px { z: i32::MIN, ..Default::default() }; CHAR_W * CHAR_H] }
    }

    fn get(&self, x: i32, y: i32) -> Option<&Px> {
        if x < 0 || y < 0 || x >= CHAR_W as i32 || y >= CHAR_H as i32 {
            None
        } else {
            Some(&self.px[y as usize * CHAR_W + x as usize])
        }
    }

    /// Writes a pixel if it is not behind what's already there.
    fn plot(&mut self, x: i32, y: i32, mat: Mat, tone: usize, group: u8, z: i32) {
        if x < 0 || y < 0 || x >= CHAR_W as i32 || y >= CHAR_H as i32 {
            return;
        }
        let p = &mut self.px[y as usize * CHAR_W + x as usize];
        if z >= p.z {
            *p = Px { idx: mat.ramp()[tone.min(2)], mat: Some(mat), group, z, hand: false };
        }
    }

    /// Overwrites the colour of an existing pixel (face details etc.).
    fn paint(&mut self, x: i32, y: i32, idx: u8) {
        if x >= 0 && y >= 0 && x < CHAR_W as i32 && y < CHAR_H as i32 {
            let p = &mut self.px[y as usize * CHAR_W + x as usize];
            if p.idx != 0 {
                p.idx = idx;
            }
        }
    }
}

/// Light from the top-left, slightly toward the viewer.
const LIGHT: [f32; 3] = [-0.46, -0.58, 0.67];

fn tone(n: [f32; 3]) -> usize {
    let b = n[0] * LIGHT[0] + n[1] * LIGHT[1] + n[2] * LIGHT[2];
    if b < 0.30 {
        0
    } else if b < 0.76 {
        1
    } else {
        2
    }
}

/// Normal of a cylinder seen side-on: `u` in -1..1 across `perp`.
fn cyl_normal(perp: P, u: f32) -> [f32; 3] {
    [perp.0 * u, perp.1 * u, (1.0 - u * u).max(0.0).sqrt()]
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Draws a capsule from `a` to `b`. `mat(t, edge)` picks the material along
/// the segment (t = 0..1) and may force a tone.
/// `mat(t, u)` gets the position along the segment (0..1) and the distance
/// from its axis (0..1 of the radius).
fn capsule(cv: &mut Canvas, a: P, b: P, r: f32, group: u8, z: i32, mat: impl Fn(f32, f32) -> (Mat, Option<usize>)) {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len2 = (dx * dx + dy * dy).max(1e-4);
    let x0 = (a.0.min(b.0) - r - 1.0).floor() as i32;
    let x1 = (a.0.max(b.0) + r + 1.0).ceil() as i32;
    let y0 = (a.1.min(b.1) - r - 1.0).floor() as i32;
    let y1 = (a.1.max(b.1) + r + 1.0).ceil() as i32;
    for y in y0..=y1 {
        for x in x0..=x1 {
            let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
            let t = (((px - a.0) * dx + (py - a.1) * dy) / len2).clamp(0.0, 1.0);
            let (cx, cy) = (a.0 + dx * t, a.1 + dy * t);
            let (vx, vy) = (px - cx, py - cy);
            let d = (vx * vx + vy * vy).sqrt();
            if d > r {
                continue;
            }
            let u = d / r;
            let perp = if d > 1e-4 { (vx / d, vy / d) } else { (0.0, 0.0) };
            let (m, forced) = mat(t, u);
            cv.plot(x, y, m, forced.unwrap_or_else(|| tone(cyl_normal(perp, u))), group, z);
        }
    }
}

fn ellipse(cv: &mut Canvas, c: P, rx: f32, ry: f32, group: u8, z: i32, mut mat: impl FnMut(i32, i32, f32, f32) -> Option<(Mat, Option<usize>)>) {
    for y in (c.1 - ry - 1.0).floor() as i32..=(c.1 + ry + 1.0).ceil() as i32 {
        for x in (c.0 - rx - 1.0).floor() as i32..=(c.0 + rx + 1.0).ceil() as i32 {
            let nx = (x as f32 + 0.5 - c.0) / rx;
            let ny = (y as f32 + 0.5 - c.1) / ry;
            let r2 = nx * nx + ny * ny;
            if r2 > 1.0 {
                continue;
            }
            if let Some((m, forced)) = mat(x, y, nx, ny) {
                let n = [nx, ny, (1.0 - r2).max(0.0).sqrt()];
                cv.plot(x, y, m, forced.unwrap_or_else(|| tone(n)), group, z);
            }
        }
    }
}

// ---------------------------------------------------------------- body parts

fn arm(cv: &mut Canvas, limb: &Limb, outfit: &Outfit, group: u8, z: i32) {
    let sleeve = match outfit.top {
        Top::Shirt { sleeve, .. } => sleeve,
        Top::Bare => Sleeve::None,
    };
    let upper_cover = match sleeve {
        Sleeve::None => 0.0,
        Sleeve::Short => 0.55,
        Sleeve::Rolled | Sleeve::Long => 1.0,
    };
    let sleeve_r = if sleeve == Sleeve::None { 1.55 } else { 1.8 };
    capsule(cv, limb.a, limb.b, sleeve_r, group, z, |t, _| {
        if t < upper_cover { (Mat::Top, None) } else { (Mat::Skin, None) }
    });
    // How far down the forearm the sleeve reaches, and where its cuff starts.
    let (cover, cuff) = match sleeve {
        Sleeve::Long => (0.78, 0.62),
        Sleeve::Rolled => (0.32, 0.12),
        _ => (0.0, 0.0),
    };
    capsule(cv, limb.b, limb.c, if cover > 0.0 { 1.55 } else { 1.35 }, group, z, |t, _| {
        if t < cover {
            (Mat::Top, if t > cuff { Some(2) } else { None })
        } else {
            (Mat::Skin, None)
        }
    });
    // Hand, flagged so it can be extracted as an overlay.
    let (hx, hy) = limb.c;
    for y in (hy - 2.0).floor() as i32..=(hy + 2.0).ceil() as i32 {
        for x in (hx - 2.0).floor() as i32..=(hx + 2.0).ceil() as i32 {
            let (dx, dy) = (x as f32 + 0.5 - hx, y as f32 + 0.5 - hy);
            let d2 = dx * dx + dy * dy;
            if d2 > 1.6 * 1.6 || x < 0 || y < 0 || x >= CHAR_W as i32 || y >= CHAR_H as i32 {
                continue;
            }
            let n = [dx / 1.6, dy / 1.6, (1.0 - d2 / 2.56).max(0.0).sqrt()];
            cv.plot(x, y, Mat::Skin, tone(n), group, z + 1);
            let p = &mut cv.px[y as usize * CHAR_W + x as usize];
            if p.z == z + 1 && p.group == group {
                p.hand = true;
            }
        }
    }
}

fn leg(cv: &mut Canvas, limb: &Limb, view: View, outfit: &Outfit, group: u8, z: i32) {
    let shorts = outfit.legs == Legwear::Shorts;
    let jeans = outfit.legs == Legwear::Jeans;
    // The rip is on one knee only.
    let ripped = jeans && group == group_ids::LEG_A;
    capsule(cv, limb.a, limb.b, if shorts { 2.0 } else { 1.85 }, group, z, |t, u| {
        if ripped && t > 0.86 && u < 0.55 {
            (Mat::Skin, None)
        } else if !shorts || t < 0.78 {
            // Rolled cuff on shorts; faded fronts on jeans.
            let tone = if (shorts && t > 0.62) || (jeans && u < 0.3 && t > 0.2) { Some(2) } else { None };
            (Mat::Bottom, tone)
        } else {
            (Mat::Skin, None)
        }
    });
    let boots = outfit.feet == Footwear::Boots;
    capsule(cv, limb.b, limb.c, 1.5, group, z, |t, u| {
        if boots && t > 0.45 {
            (Mat::Hair, if t < 0.55 { Some(2) } else { None })
        } else if ripped && t < 0.16 && u < 0.6 {
            (Mat::Skin, None)
        } else if !shorts && t < 0.95 {
            // Rolled hem at the ankle on jeans.
            (Mat::Bottom, if jeans && t > 0.8 { Some(2) } else { None })
        } else {
            (Mat::Skin, None)
        }
    });
    // Foot.
    let (fx, fy) = limb.c;
    let feet = outfit.feet;
    let sneakers = feet == Footwear::Sneakers;
    let (cx, rx, ry) = match (view == View::Side, sneakers) {
        (true, true) => (fx + 1.5, 3.0, 1.5),
        (true, false) => (fx + 1.4, 2.6, 1.3),
        (false, true) => (fx, 2.1, 1.5),
        (false, false) => (fx, 1.9, 1.3),
    };
    let lace_x = cx.round() as i32;
    ellipse(cv, (cx, fy + 0.6), rx, ry, group, z + 1, |x, y, _, ny| {
        Some(match feet {
            Footwear::Boots => (Mat::Hair, None),
            Footwear::Sandals if ny > 0.3 => (Mat::Accent, Some(0)),
            Footwear::Sandals if (y as f32) < fy => (Mat::Accent, Some(1)),
            // Sole along the bottom, laces down the middle of the upper.
            Footwear::Sneakers if ny > 0.4 => (Mat::Accent, Some(0)),
            Footwear::Sneakers if x == lace_x && ny < 0.1 && view != View::Side => (Mat::Accent, Some(0)),
            Footwear::Sneakers => (Mat::Accent, Some(1)),
            _ => (Mat::Skin, None),
        })
    });
}

fn hips(cv: &mut Canvas, pose: &Pose, outfit: &Outfit, z: i32) {
    let (cx, cy) = pose.pelvis;
    let w = pose.hip_w + 0.4;
    for y in (cy - 1.5) as i32..=(cy + 2.0) as i32 {
        for x in (cx - w - 1.0) as i32..=(cx + w + 1.0) as i32 {
            let u = (x as f32 + 0.5 - cx) / w;
            if u.abs() > 1.0 {
                continue;
            }
            let top_row = (y as f32) < cy - 0.5;
            // Waistband: a belt on trousers, a dark band on shorts.
            let (m, t) = if top_row && outfit.legs != Legwear::Shorts {
                (Mat::Hair, Some(0))
            } else if top_row {
                (Mat::Bottom, Some(0))
            } else {
                (Mat::Bottom, Some(tone(cyl_normal((1.0, 0.0), u))))
            };
            cv.plot(x, y, m, t.unwrap(), group::HIPS, z);
        }
    }
}

fn torso(cv: &mut Canvas, pose: &Pose, outfit: &Outfit, z: i32) {
    let sw = pose.shoulder_w;
    let (shirt, torn, collar) = match outfit.top {
        Top::Shirt { torn, collar, .. } => (true, torn, collar),
        Top::Bare => (false, false, false),
    };
    // A bare torso stops above the waistband; a shirt hangs over it.
    let (top, bottom) = (pose.neck.1 + 0.3, pose.pelvis.1 + if shirt { 0.6 } else { -1.2 });
    for y in top.floor() as i32..=bottom.ceil() as i32 {
        let yc = y as f32 + 0.5;
        let t = ((yc - pose.neck.1) / (pose.pelvis.1 - pose.neck.1)).clamp(0.0, 1.0);
        let cx = lerp(pose.neck.0, pose.pelvis.0, t);
        let hw = if t < 0.16 {
            sw * (0.55 + 0.45 * t / 0.16)
        } else if t < 0.72 {
            lerp(sw, sw * 0.8, (t - 0.16) / 0.56)
        } else {
            lerp(sw * 0.8, pose.hip_w, (t - 0.72) / 0.28)
        };
        for x in (cx - hw - 1.0).floor() as i32..=(cx + hw + 1.0).ceil() as i32 {
            let xc = x as f32 + 0.5;
            let u = (xc - cx) / hw;
            if u.abs() > 1.0 || yc < top {
                continue;
            }
            // Torn hem lets the shorts show through.
            let jag = ((x * 7 + 3) % 4) as f32 * 0.45;
            if torn && yc > pose.pelvis.1 - 0.4 + jag {
                continue;
            }
            let mut n = cyl_normal((1.0, 0.0), u);
            n[1] = -0.35 * (1.0 - t); // chest faces slightly up
            let mut tn = tone(n);
            let mut m = if shirt { Mat::Top } else { Mat::Skin };
            let below_neck = yc - pose.neck.1;
            let dx = xc - cx;
            if shirt && pose.view == View::Front {
                // Open neck, with collar points either side on a dress shirt.
                let v = 2.8 - below_neck;
                if v > 0.0 && dx.abs() < v * 0.75 {
                    m = Mat::Skin;
                    tn = 1;
                } else if collar && below_neck < 3.4 && dx.abs() < 3.2 {
                    tn = 2;
                } else if collar && dx.abs() < 0.5 && below_neck > 3.6 {
                    // Button placket.
                    tn = if (below_neck as i32) % 2 == 0 { 0 } else { 2 };
                }
            } else if shirt && collar && below_neck < 1.4 && (pose.view == View::Back || u > 0.1) {
                tn = 2;
            }
            if torn && pose.view != View::Side {
                // A couple of rips showing skin.
                let rips = [(2.2f32, 5.6f32), (-2.6, 7.4)];
                if rips.iter().any(|&(rx, ry)| (dx - rx).abs() < 0.9 && (below_neck - ry).abs() < 0.7) {
                    m = Mat::Skin;
                    tn = 1;
                }
            }
            if shirt && t > 0.55 && (x * 2 + y) % 7 == 0 && tn == 1 {
                tn = 0; // cloth folds
            }
            if !shirt && pose.view == View::Front {
                // Chest and stomach definition.
                let row = (yc - pose.neck.1).round() as i32;
                if (row == 4 && u.abs() > 0.15 && u.abs() < 0.75) || ((xc - cx).abs() < 0.5 && (6..=8).contains(&row)) {
                    tn = 0;
                }
            }
            cv.plot(x, y, m, tn, group::TORSO, z);
        }
    }
}

fn head(cv: &mut Canvas, pose: &Pose, z: i32) {
    let (hx, hy) = pose.head;
    let rx = if pose.view == View::Side { 5.0 } else { 5.2 };
    // Neck.
    capsule(cv, (pose.neck.0, hy + 3.5), (pose.neck.0, pose.neck.1 + 1.0), 1.7, group::HEAD, z - 1, |_, _| (Mat::Skin, None));
    // Ears.
    match pose.view {
        View::Side => {}
        _ => {
            for sx in [-1.0f32, 1.0] {
                ellipse(cv, (hx + sx * 5.1, hy + 1.6), 1.1, 1.5, group::HEAD, z - 1, |_, _, _, _| Some((Mat::Skin, None)));
            }
        }
    }
    ellipse(cv, (hx, hy), rx, 5.7, group::HEAD, z, |_, _, _, _| Some((Mat::Skin, None)));
    if pose.view == View::Side {
        // Nose and chin profile.
        cv.plot((hx + 5.2) as i32, (hy + 1.6) as i32, Mat::Skin, 1, group::HEAD, z);
        ellipse(cv, (hx - 0.6, hy + 1.6), 1.2, 1.6, group::HEAD, z + 1, |_, _, _, ny| Some((Mat::Skin, Some(if ny > 0.0 { 0 } else { 1 }))));
    }
}

/// Fringe spikes per column (in pixels below the base line).
const SPIKES: [f32; 8] = [0.4, 1.8, 0.8, 2.2, 1.0, 1.6, 0.2, 2.0];

fn spike(x: i32) -> f32 {
    SPIKES[x.rem_euclid(8) as usize]
}

fn hair(cv: &mut Canvas, pose: &Pose, z: i32) {
    let (hx, hy) = pose.head;
    let view = pose.view;
    let (ecx, ecy, erx, ery) = match view {
        View::Side => (hx - 0.5, hy - 0.5, 5.8, 6.2),
        View::Back => (hx, hy + 0.1, 5.9, 6.3),
        View::Front => (hx, hy - 0.6, 5.9, 6.1),
    };
    ellipse(cv, (ecx, ecy), erx, ery, group::HAIR, z, |x, y, nx, _| {
        let (xc, yc) = (x as f32 + 0.5, y as f32 + 0.5);
        let inside = match view {
            View::Front => yc < hy - 2.6 + spike(x) * 0.8 || ((xc - hx).abs() > 4.4 && yc < hy + 2.4),
            View::Back => yc < hy + 4.4 + spike(x) * 0.6,
            View::Side => yc < hy - 1.4 + spike(x) * 0.7 || (xc < hx + 0.4 - (yc - hy) * 0.15 && yc < hy + 4.4 + spike(x) * 0.4),
        };
        if !inside {
            return None;
        }
        // Strand texture: short diagonal streaks combing down and outward.
        let comb = if nx < 0.0 { x + y } else { x - y };
        let streak = comb.rem_euclid(4) == 0 && (y + x / 3).rem_euclid(3) != 0 && nx.abs() > 0.15;
        Some((Mat::Hair, if streak { Some(0) } else { None }))
    });
    // A few tufts sticking up from the crown.
    for (dx, h) in [(-3.0f32, 1.0f32), (0.0, 1.5), (2.5, 1.0)] {
        let x = (ecx + dx) as i32;
        let top = (ecy - ery) as i32;
        for k in 0..h as i32 {
            cv.plot(x, top - k, Mat::Hair, 1, group::HAIR, z);
        }
    }
}

fn headwear(cv: &mut Canvas, pose: &Pose, outfit: &Outfit, z: i32) {
    let (hx, hy) = pose.head;
    match outfit.head {
        Headwear::None => {}
        Headwear::Bandana => {
            let rx = if pose.view == View::Side { 5.5 } else { 5.8 };
            ellipse(cv, (hx, hy), rx, 6.0, group::HAT, z, |_, y, _, _| {
                let yc = y as f32 + 0.5;
                if (hy - 3.2..hy - 1.2).contains(&yc) {
                    Some((Mat::Accent, Some(if yc < hy - 2.2 { 1 } else { 0 })))
                } else {
                    None
                }
            });
            // Knot and tails at the back of the head.
            let (kx, tail_dir) = match pose.view {
                View::Back => (hx, 0.0),
                View::Side => (hx - 5.4, -1.0),
                View::Front => return,
            };
            ellipse(cv, (kx, hy - 2.0), 1.0, 0.9, group::HAT, z + 1, |_, _, _, ny| Some((Mat::Accent, Some(if ny < 0.0 { 1 } else { 0 }))));
            for (ex, ey) in [(kx - 1.6 + tail_dir * 1.5, hy + 1.6), (kx + 1.2 + tail_dir * 2.5, hy + 2.4)] {
                capsule(cv, (kx, hy - 1.6), (ex, ey), 0.55, group::HAT, z + 1, |t, _| (Mat::Accent, Some(if t < 0.4 { 1 } else { 0 })));
            }
        }
        Headwear::StrawHat => {
            let bx = if pose.view == View::Side { hx + 0.6 } else { hx };
            // Brim.
            ellipse(cv, (bx, hy - 2.0), 8.6, 2.5, group::HAT, z, |x, y, _, ny| {
                let weave = (x + y) % 2 == 0;
                Some((Mat::Accent, Some(if ny > 0.35 || (weave && ny < -0.2) { 0 } else { 1 })))
            });
            // Crown with a ribbon.
            ellipse(cv, (hx, hy - 4.6), 4.6, 3.2, group::HAT, z + 1, |x, y, _, ny| {
                let yc = y as f32 + 0.5;
                if yc > hy - 2.3 {
                    return None;
                }
                if yc > hy - 3.4 {
                    return Some((Mat::Hair, Some(0)));
                }
                let weave = (x * 2 + y) % 3 == 0;
                Some((Mat::Accent, if weave { Some(0) } else if ny < -0.4 { Some(1) } else { None }))
            });
        }
    }
}

/// Eyes, brows, nose and mouth, painted over the shaded head.
fn face(cv: &mut Canvas, pose: &Pose) {
    let (hx, hy) = pose.head;
    let r = |v: f32| v.round() as i32;
    let not_hair = |cv: &Canvas, x: i32, y: i32| cv.get(x, y).is_some_and(|p| p.group == group::HEAD);
    match pose.view {
        View::Front => {
            for ex in [r(hx - 3.0), r(hx + 1.0)] {
                let ey = r(hy + 0.6);
                if pose.closed_eyes {
                    // Closed: a lash line with the lid above it.
                    cv.paint(ex, ey + 1, slot::OUTLINE);
                    cv.paint(ex + 1, ey + 1, slot::OUTLINE);
                    cv.paint(ex, ey, slot::SKIN[0]);
                    cv.paint(ex + 1, ey, slot::SKIN[0]);
                } else {
                    // 2x2 eye with a glint in the upper-left corner.
                    cv.paint(ex, ey, slot::SKIN[2]);
                    cv.paint(ex + 1, ey, slot::OUTLINE);
                    cv.paint(ex, ey + 1, slot::OUTLINE);
                    cv.paint(ex + 1, ey + 1, slot::OUTLINE);
                }
                for dx in 0..2 {
                    if not_hair(cv, ex + dx, ey - 2) {
                        cv.paint(ex + dx, ey - 2, slot::HAIR[0]);
                    }
                }
            }
            cv.paint(r(hx), r(hy + 2.6), slot::SKIN[0]);
            cv.paint(r(hx - 0.5), r(hy + 4.0), slot::SKIN[0]);
            cv.paint(r(hx + 0.5), r(hy + 4.0), slot::SKIN[0]);
        }
        View::Side => {
            let (ex, ey) = (r(hx + 2.4), r(hy + 0.6));
            if pose.closed_eyes {
                cv.paint(ex, ey + 1, slot::OUTLINE);
                cv.paint(ex + 1, ey + 1, slot::OUTLINE);
            } else {
                cv.paint(ex, ey, slot::OUTLINE);
                cv.paint(ex, ey + 1, slot::OUTLINE);
                cv.paint(ex + 1, ey + 1, slot::SKIN[2]);
            }
            if not_hair(cv, ex, ey - 2) {
                cv.paint(ex, ey - 2, slot::HAIR[0]);
                cv.paint(ex + 1, ey - 2, slot::HAIR[0]);
            }
            cv.paint(r(hx + 3.8), r(hy + 4.0), slot::SKIN[0]);
        }
        View::Back => {}
    }
}

// ---------------------------------------------------------------- assembly

pub struct Rendered {
    pub body: PixelArt,
    /// Only the hands (with their outline), to draw over a held tool.
    pub hands: PixelArt,
}

/// Whether an overlap between two groups should get a dividing line.
fn divides(front: u8, back: u8) -> bool {
    if front == back {
        return false;
    }
    // Hips flow into the thighs.
    !(front == group::HIPS && (back == group::LEG_A || back == group::LEG_B))
}

pub fn render(pose: &Pose, outfit: &Outfit) -> Rendered {
    let mut cv = Canvas::new();

    // Legs, hips, torso, arms, head, hair, headwear — each with its depth.
    // The second leg sits a little nearer so a dividing line separates them.
    let leg_z = |i: usize| 10 + i as i32 * 3;
    leg(&mut cv, &pose.legs[0], pose.view, outfit, group::LEG_A, leg_z(0));
    leg(&mut cv, &pose.legs[1], pose.view, outfit, group::LEG_B, leg_z(1));
    hips(&mut cv, pose, outfit, 20);
    torso(&mut cv, pose, outfit, 30);
    for (i, arm_limb) in pose.arms.iter().enumerate() {
        // From behind, arms reaching forward disappear behind the torso.
        let z = if arm_limb.front { 40 + i as i32 } else { 5 + i as i32 };
        arm(&mut cv, arm_limb, outfit, if i == 0 { group::ARM_A } else { group::ARM_B }, z);
    }
    head(&mut cv, pose, 50);
    hair(&mut cv, pose, 55);
    headwear(&mut cv, pose, outfit, 60);
    face(&mut cv, pose);

    // Selective outline: darken pixels where a nearer group overlaps them.
    let snapshot = cv.px.clone();
    let at = |x: i32, y: i32| -> Option<&Px> {
        if x < 0 || y < 0 || x >= CHAR_W as i32 || y >= CHAR_H as i32 { None } else { Some(&snapshot[y as usize * CHAR_W + x as usize]) }
    };
    for y in 0..CHAR_H as i32 {
        for x in 0..CHAR_W as i32 {
            let me = snapshot[y as usize * CHAR_W + x as usize];
            let Some(mat) = me.mat else { continue };
            let overlapped = [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().any(|&(dx, dy)| {
                at(x + dx, y + dy).is_some_and(|n| n.mat.is_some() && n.z > me.z + 2 && divides(n.group, me.group))
            });
            if overlapped && me.idx != slot::OUTLINE {
                cv.px[y as usize * CHAR_W + x as usize].idx = mat.ramp()[0];
            }
        }
    }

    // Silhouette rim.
    let mut body = PixelArt::filled(CHAR_W, CHAR_H, 0);
    let mut hands = PixelArt::filled(CHAR_W, CHAR_H, 0);
    for y in 0..CHAR_H as i32 {
        for x in 0..CHAR_W as i32 {
            let p = cv.px[y as usize * CHAR_W + x as usize];
            if p.mat.is_some() {
                body.set(x as usize, y as usize, p.idx);
                if p.hand {
                    hands.set(x as usize, y as usize, p.idx);
                }
                continue;
            }
            let near = |f: &dyn Fn(&Px) -> bool| {
                [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().any(|&(dx, dy)| cv.get(x + dx, y + dy).is_some_and(f))
            };
            if near(&|n| n.mat.is_some()) {
                body.set(x as usize, y as usize, slot::OUTLINE);
                if near(&|n| n.hand) {
                    hands.set(x as usize, y as usize, slot::OUTLINE);
                }
            }
        }
    }
    Rendered { body, hands }
}

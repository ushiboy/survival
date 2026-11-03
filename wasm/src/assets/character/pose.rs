//! Pose skeletons in canvas coordinates (24x32, feet on y≈30). Side views
//! face right; facing left is the mirrored sprite.

pub type P = (f32, f32);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum View {
    Front,
    Back,
    Side,
}

/// A three-joint limb (shoulder–elbow–hand or hip–knee–foot).
#[derive(Clone, Copy, Debug)]
pub struct Limb {
    pub a: P,
    pub b: P,
    pub c: P,
    /// Drawn in front of the torso.
    pub front: bool,
}

/// How a held axe is drawn relative to the grip (the midpoint of the hands).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AxeArt {
    /// Horizontal, head toward +x.
    Side,
    /// Vertical, head toward -y.
    Up,
    /// 45°, head toward +x/-y.
    Diag,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ToolHold {
    pub art: AxeArt,
    pub flip_h: bool,
    pub flip_v: bool,
    /// Drawn in front of the body (hands are then overlaid on the handle).
    pub front: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct Pose {
    pub view: View,
    pub head: P,
    pub neck: P,
    pub pelvis: P,
    pub shoulder_w: f32,
    pub hip_w: f32,
    /// Front/back: [screen-left, screen-right]. Side: [far, near].
    pub arms: [Limb; 2],
    pub legs: [Limb; 2],
    pub tool: Option<ToolHold>,
}

impl Pose {
    /// Midpoint between the hands.
    pub fn grip(&self) -> P {
        let (a, b) = (self.arms[0].c, self.arms[1].c);
        ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0)
    }

    fn shift_upper(mut self, dy: f32) -> Self {
        let s = |p: &mut P| p.1 += dy;
        s(&mut self.head);
        s(&mut self.neck);
        s(&mut self.pelvis);
        for arm in &mut self.arms {
            s(&mut arm.a);
            s(&mut arm.b);
            s(&mut arm.c);
        }
        for leg in &mut self.legs {
            s(&mut leg.a);
        }
        self
    }
}

const fn limb(a: P, b: P, c: P, front: bool) -> Limb {
    Limb { a, b, c, front }
}

fn front_stand(view: View) -> Pose {
    Pose {
        view,
        head: (12.0, 7.0),
        neck: (12.0, 13.0),
        pelvis: (12.0, 21.0),
        shoulder_w: 4.4,
        hip_w: 3.7,
        arms: [
            limb((7.9, 14.8), (7.0, 18.6), (7.1, 22.0), true),
            limb((16.1, 14.8), (17.0, 18.6), (16.9, 22.0), true),
        ],
        legs: [
            limb((10.0, 21.5), (9.8, 25.8), (9.8, 29.6), false),
            limb((14.0, 21.5), (14.2, 25.8), (14.2, 29.6), false),
        ],
        tool: None,
    }
}

fn side_stand() -> Pose {
    Pose {
        view: View::Side,
        head: (12.6, 7.0),
        neck: (12.2, 13.0),
        pelvis: (11.8, 21.0),
        shoulder_w: 3.5,
        hip_w: 3.2,
        arms: [
            limb((12.4, 14.8), (11.9, 18.8), (12.3, 22.2), false),
            limb((11.6, 15.0), (11.2, 19.0), (11.7, 22.4), true),
        ],
        legs: [
            limb((12.2, 21.5), (12.4, 25.8), (12.5, 29.6), false),
            limb((11.4, 21.5), (11.4, 25.8), (11.3, 29.6), false),
        ],
        tool: None,
    }
}

/// Four-frame walk: stand, step A, stand, step B.
pub fn walk(view: View, phase: usize) -> Pose {
    let s = [0.0, 1.0, 0.0, -1.0][phase % 4];
    let contact = phase % 2 == 1;
    let mut p = if view == View::Side { side_stand() } else { front_stand(view) };
    if view == View::Side {
        // Near leg forward on step A; arms swing opposite to the legs.
        let [far, near] = &mut p.legs;
        near.c.0 += 3.2 * s;
        near.b.0 += 1.8 * s + 0.6;
        far.c.0 -= 3.2 * s;
        far.b.0 -= 1.8 * s - 0.6;
        // Trailing heel lifts.
        if s > 0.0 { far.c.1 -= 0.9 } else if s < 0.0 { near.c.1 -= 0.9 }
        let [far_arm, near_arm] = &mut p.arms;
        near_arm.c.0 -= 2.4 * s;
        near_arm.b.0 -= 1.0 * s;
        far_arm.c.0 += 2.4 * s;
        far_arm.b.0 += 1.0 * s;
    } else {
        // Facing the camera "forward" is down the screen.
        let [l, r] = &mut p.legs;
        let lift = |leg: &mut Limb, k: f32| {
            leg.c.1 -= 1.3 * k;
            leg.b.1 -= 0.8 * k;
        };
        if s > 0.0 { lift(l, 1.0) } else if s < 0.0 { lift(r, 1.0) }
        let [la, ra] = &mut p.arms;
        la.c.1 -= 1.0 * s;
        ra.c.1 += 1.0 * s;
        la.b.1 -= 0.4 * s;
        ra.b.1 += 0.4 * s;
    }
    if contact { p.shift_upper(1.0) } else { p }
}

/// Axe swing poses: 0 = wound up, 1 = sweeping past the body, 2 = strike.
/// Hips stay planted in a wide stance while the shoulders twist.
pub fn chop(view: View, stage: usize) -> Pose {
    let hold = |art, flip_h, flip_v, front| Some(ToolHold { art, flip_h, flip_v, front });
    match view {
        View::Side => {
            let legs = [
                limb((11.2, 21.6), (8.8, 25.6), (7.0, 29.6), false),
                limb((12.8, 21.6), (15.2, 25.4), (16.8, 29.6), false),
            ];
            match stage {
                0 => Pose {
                    view,
                    head: (11.4, 7.3),
                    neck: (11.2, 13.2),
                    pelvis: (12.0, 21.0),
                    shoulder_w: 3.9,
                    hip_w: 3.2,
                    arms: [
                        limb((12.6, 15.0), (9.0, 18.4), (5.8, 21.2), false),
                        limb((10.6, 15.2), (7.8, 18.8), (5.2, 21.6), true),
                    ],
                    legs,
                    tool: hold(AxeArt::Side, true, false, false),
                },
                1 => Pose {
                    view,
                    head: (12.6, 7.4),
                    neck: (12.3, 13.3),
                    pelvis: (12.0, 21.0),
                    shoulder_w: 3.4,
                    hip_w: 3.2,
                    arms: [
                        limb((12.8, 15.0), (13.6, 19.2), (14.4, 22.6), true),
                        limb((11.4, 15.2), (12.4, 19.4), (13.8, 23.0), true),
                    ],
                    legs,
                    tool: hold(AxeArt::Up, false, true, true),
                },
                _ => Pose {
                    view,
                    head: (14.2, 7.6),
                    neck: (13.6, 13.4),
                    pelvis: (12.2, 21.0),
                    shoulder_w: 2.7,
                    hip_w: 3.2,
                    arms: [
                        limb((14.2, 14.9), (17.0, 17.0), (19.6, 18.8), false),
                        limb((13.0, 15.1), (16.2, 17.6), (19.2, 19.4), true),
                    ],
                    legs: [
                        limb((11.4, 21.6), (9.0, 25.6), (7.0, 29.6), false),
                        limb((13.0, 21.6), (15.8, 25.2), (17.6, 29.6), false),
                    ],
                    tool: hold(AxeArt::Side, false, false, false),
                },
            }
        }
        View::Front => {
            let legs = [
                limb((10.0, 21.5), (8.6, 25.6), (7.6, 29.6), false),
                limb((14.0, 21.5), (15.4, 25.6), (16.4, 29.6), false),
            ];
            match stage {
                0 => Pose {
                    view,
                    head: (10.9, 7.3),
                    neck: (11.2, 13.1),
                    pelvis: (12.0, 21.0),
                    shoulder_w: 4.0,
                    hip_w: 3.7,
                    arms: [
                        limb((7.6, 14.8), (5.6, 18.2), (4.8, 21.4), true),
                        limb((14.8, 14.8), (10.8, 19.0), (5.6, 21.0), true),
                    ],
                    legs,
                    tool: hold(AxeArt::Side, true, false, false),
                },
                1 => Pose {
                    view,
                    head: (11.6, 7.4),
                    neck: (11.8, 13.2),
                    pelvis: (12.0, 21.0),
                    shoulder_w: 4.2,
                    hip_w: 3.7,
                    arms: [
                        limb((7.8, 14.8), (7.4, 19.0), (8.6, 23.2), true),
                        limb((15.8, 14.8), (12.6, 19.6), (9.4, 23.4), true),
                    ],
                    legs,
                    tool: hold(AxeArt::Diag, true, true, true),
                },
                _ => Pose {
                    view,
                    head: (13.0, 7.4),
                    neck: (12.7, 13.2),
                    pelvis: (12.0, 21.0),
                    shoulder_w: 4.2,
                    hip_w: 3.7,
                    arms: [
                        limb((8.6, 15.0), (9.8, 19.6), (12.0, 23.4), true),
                        limb((16.6, 15.0), (15.6, 19.8), (13.2, 23.4), true),
                    ],
                    legs,
                    tool: hold(AxeArt::Up, false, true, true),
                },
            }
        }
        View::Back => {
            let legs = [
                limb((10.0, 21.5), (8.6, 25.6), (7.6, 29.6), false),
                limb((14.0, 21.5), (15.4, 25.6), (16.4, 29.6), false),
            ];
            match stage {
                0 => Pose {
                    view,
                    head: (13.1, 7.3),
                    neck: (12.8, 13.1),
                    pelvis: (12.0, 21.0),
                    shoulder_w: 4.0,
                    hip_w: 3.7,
                    arms: [
                        limb((9.2, 14.8), (13.2, 19.0), (18.4, 21.0), false),
                        limb((16.4, 14.8), (18.4, 18.2), (19.2, 21.4), true),
                    ],
                    legs,
                    tool: hold(AxeArt::Side, false, false, false),
                },
                1 => Pose {
                    view,
                    head: (12.6, 7.2),
                    neck: (12.4, 13.0),
                    pelvis: (12.0, 21.0),
                    shoulder_w: 4.2,
                    hip_w: 3.7,
                    arms: [
                        limb((8.4, 14.8), (11.4, 17.0), (14.6, 17.6), false),
                        limb((16.2, 14.8), (17.4, 16.4), (15.6, 17.2), true),
                    ],
                    legs,
                    tool: hold(AxeArt::Diag, false, false, false),
                },
                _ => Pose {
                    view,
                    head: (11.2, 7.2),
                    neck: (11.4, 13.0),
                    pelvis: (12.0, 21.0),
                    shoulder_w: 4.2,
                    hip_w: 3.7,
                    arms: [
                        limb((7.6, 14.8), (9.6, 16.2), (11.4, 16.8), false),
                        limb((15.4, 14.8), (13.6, 16.4), (12.0, 16.8), false),
                    ],
                    legs,
                    tool: hold(AxeArt::Up, false, false, false),
                },
            }
        }
    }
}

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
    pub closed_eyes: bool,
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
        closed_eyes: false,
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
        closed_eyes: false,
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
                    closed_eyes: false,
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
                    closed_eyes: false,
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
                    closed_eyes: false,
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
                    closed_eyes: false,
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
                    closed_eyes: false,
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
                    closed_eyes: false,
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
                    closed_eyes: false,
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
                    closed_eyes: false,
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
                    closed_eyes: false,
                },
            }
        }
    }
}

// ---------------------------------------------------------------- spear thrust

/// Spear thrust poses: 0 = drawn back, 1 = lunging with the arms out. The
/// spear is a separate sprite the player lines up with the grip; facing the
/// camera or away it's held on the right so the shaft clears the head.
pub fn thrust(view: View, stage: usize) -> Pose {
    let out = stage == 1;
    match view {
        View::Side => {
            let mut p = side_stand();
            p.legs = [
                limb((12.0, 21.6), (9.6, 25.6), (7.6, 29.6), false),
                limb((12.6, 21.6), (14.8, 25.4), (16.4, 29.6), false),
            ];
            if out {
                p.head = (14.0, 7.6);
                p.neck = (13.4, 13.4);
                p.pelvis = (12.4, 21.0);
                p.arms = [
                    limb((13.8, 15.0), (16.8, 16.8), (19.8, 18.2), false),
                    limb((12.8, 15.2), (15.8, 17.6), (18.6, 18.6), true),
                ];
            } else {
                p.head = (11.8, 7.2);
                p.neck = (11.6, 13.1);
                p.arms = [
                    limb((12.6, 14.9), (10.6, 18.2), (8.0, 20.4), false),
                    limb((11.0, 15.1), (9.6, 18.8), (8.8, 21.4), true),
                ];
            }
            p
        }
        View::Front => {
            let mut p = front_stand(view);
            p.legs = [
                limb((10.0, 21.5), (9.0, 25.6), (8.4, 29.6), false),
                limb((14.0, 21.5), (15.0, 25.6), (15.6, 29.6), false),
            ];
            if out {
                p.head = (12.0, 7.6);
                p.neck = (12.0, 13.4);
                p.arms = [
                    limb((7.9, 15.0), (10.6, 20.2), (15.0, 24.2), true),
                    limb((16.1, 15.0), (17.6, 20.0), (16.6, 25.2), true),
                ];
            } else {
                p.arms = [
                    limb((7.9, 14.8), (9.6, 18.8), (14.6, 18.8), true),
                    limb((16.1, 14.8), (18.2, 18.2), (16.8, 20.0), true),
                ];
            }
            p
        }
        View::Back => {
            let mut p = front_stand(view);
            p.legs = [
                limb((10.0, 21.5), (9.0, 25.6), (8.4, 29.6), false),
                limb((14.0, 21.5), (15.0, 25.6), (15.6, 29.6), false),
            ];
            if out {
                p.head = (12.0, 6.8);
                p.arms = [
                    limb((7.9, 14.6), (11.4, 13.6), (15.0, 11.4), false),
                    limb((16.1, 14.6), (18.0, 13.4), (16.2, 11.0), true),
                ];
            } else {
                p.arms = [
                    limb((7.9, 14.8), (10.8, 18.0), (15.0, 17.4), false),
                    limb((16.1, 14.8), (17.8, 17.4), (16.4, 17.0), true),
                ];
            }
            p
        }
    }
}

// ---------------------------------------------------------------- shaking a tree

/// Gripping a trunk with both hands and rocking it: 0 = leaning in and
/// pushing, 1 = hanging back and pulling. The hands stay where the trunk is
/// while the body rocks, so the grip reads as fixed.
pub fn shake(view: View, stage: usize) -> Pose {
    let push = stage % 2 == 0;
    match view {
        View::Side => {
            let grip = [(19.4, 16.4), (19.0, 16.8)];
            if push {
                Pose {
                    view,
                    head: (14.4, 7.8),
                    neck: (13.6, 13.6),
                    pelvis: (12.0, 21.0),
                    shoulder_w: 3.5,
                    hip_w: 3.2,
                    arms: [
                        limb((13.8, 15.2), (15.4, 19.0), grip[0], false),
                        limb((13.0, 15.4), (14.8, 19.4), grip[1], true),
                    ],
                    legs: [
                        limb((12.4, 21.6), (10.2, 25.6), (7.4, 29.6), false),
                        limb((11.6, 21.6), (14.6, 25.0), (15.0, 29.6), false),
                    ],
                    tool: None,
                    closed_eyes: false,
                }
            } else {
                Pose {
                    view,
                    head: (10.4, 7.4),
                    neck: (10.8, 13.2),
                    pelvis: (11.4, 21.2),
                    shoulder_w: 3.5,
                    hip_w: 3.2,
                    arms: [
                        limb((11.6, 15.0), (15.4, 15.8), grip[0], false),
                        limb((10.8, 15.2), (14.8, 16.2), grip[1], true),
                    ],
                    legs: [
                        limb((11.8, 21.8), (9.8, 25.4), (8.6, 29.6), false),
                        limb((11.0, 21.8), (13.8, 25.4), (15.4, 29.6), false),
                    ],
                    tool: None,
                    closed_eyes: false,
                }
            }
        }
        View::Front => {
            // The trunk is toward the viewer, just below the chest.
            let (lean, elbow) = if push { (0.9, 2.2) } else { (-0.6, 0.6) };
            let p = front_stand(view);
            Pose {
                head: (12.0, p.head.1 + lean * 1.2),
                neck: (12.0, p.neck.1 + lean),
                shoulder_w: if push { 4.6 } else { 4.2 },
                arms: [
                    limb((7.8, 14.8 + lean), (7.6 - elbow, 18.4 + lean * 0.5), (9.8, 20.4), true),
                    limb((16.2, 14.8 + lean), (16.4 + elbow, 18.4 + lean * 0.5), (14.2, 20.4), true),
                ],
                legs: [
                    limb((10.0, 21.5), (8.8, 25.6), (8.4, 29.6), false),
                    limb((14.0, 21.5), (15.2, 25.6), (15.6, 29.6), false),
                ],
                ..p
            }
        }
        View::Back => {
            // The trunk is away from the viewer: the hands show either side
            // of the shoulders, the head dips into the push.
            let (lean, elbow) = if push { (-0.9, 1.2) } else { (0.7, 0.4) };
            let p = front_stand(view);
            Pose {
                head: (12.0, p.head.1 + lean * 1.2),
                neck: (12.0, p.neck.1 + lean),
                shoulder_w: if push { 4.6 } else { 4.2 },
                arms: [
                    limb((7.8, 14.8 + lean), (6.6 - elbow, 17.4), (8.2, 13.6), false),
                    limb((16.2, 14.8 + lean), (17.4 + elbow, 17.4), (15.8, 13.6), false),
                ],
                legs: [
                    limb((10.0, 21.5), (8.8, 25.6), (8.4, 29.6), false),
                    limb((14.0, 21.5), (15.2, 25.6), (15.6, 29.6), false),
                ],
                ..p
            }
        }
    }
}

// ---------------------------------------------------------------- picking up

/// Squatting to scoop something off the ground in front of the feet.
pub fn crouch(view: View) -> Pose {
    match view {
        View::Side => {
            let mut p = side_stand().shift_upper(3.4);
            p.head.0 += 2.4;
            p.neck.0 += 1.6;
            p.legs = [
                limb((12.0, 24.8), (15.4, 26.0), (12.8, 29.6), false),
                limb((11.2, 24.8), (14.6, 26.4), (11.6, 29.6), false),
            ];
            p.arms = [
                limb((13.6, 18.2), (15.6, 22.6), (18.0, 27.6), false),
                limb((12.8, 18.4), (15.0, 23.0), (17.4, 28.0), true),
            ];
            p
        }
        View::Front | View::Back => {
            let front = view == View::Front;
            let mut p = front_stand(view).shift_upper(3.4);
            // Bending forward: toward the viewer (lower) from the front,
            // away (higher) from the back.
            let bow = if front { 0.8 } else { -0.8 };
            p.head.1 += bow;
            p.neck.1 += bow * 0.6;
            p.legs = [
                limb((10.0, 24.9), (7.4, 26.6), (8.6, 29.6), false),
                limb((14.0, 24.9), (16.6, 26.6), (15.4, 29.6), false),
            ];
            let hand_y = if front { 28.0 } else { 25.6 };
            p.arms = [
                limb((7.9, 18.2), (8.0, 22.4), (10.6, hand_y), front),
                limb((16.1, 18.2), (16.0, 22.4), (13.4, hand_y), front),
            ];
            p
        }
    }
}

/// Drinking from cupped hands raised to the mouth, head tipped back.
pub fn drink(view: View) -> Pose {
    match view {
        View::Side => {
            let mut p = side_stand();
            p.head = (12.0, 6.6);
            p.arms = [
                limb((12.4, 14.8), (15.0, 17.4), (15.6, 11.8), false),
                limb((11.6, 15.0), (14.4, 17.8), (15.0, 12.2), true),
            ];
            p
        }
        View::Front | View::Back => {
            let front = view == View::Front;
            let mut p = front_stand(view);
            p.head.1 -= 0.6;
            // Hands meet in front of the chin (hidden behind the head from
            // the back, so only the elbows show).
            let hand_y = if front { 13.4 } else { 12.6 };
            p.arms = [
                limb((7.9, 14.8), (7.2, 18.6), (11.0, hand_y), front),
                limb((16.1, 14.8), (16.8, 18.6), (13.0, hand_y), front),
            ];
            p
        }
    }
}

// ---------------------------------------------------------------- waking up

/// Sprawled on his back, one arm flung above the head. Rendered front-on and
/// then rotated a quarter turn so the body lies head-left on the ground.
pub fn lying(eyes_open: bool) -> Pose {
    Pose {
        head: (11.4, 7.2),
        arms: [
            limb((7.9, 14.8), (5.4, 10.6), (4.6, 6.2), true),
            limb((16.1, 14.8), (19.0, 17.6), (20.8, 21.0), true),
        ],
        legs: [
            limb((10.0, 21.5), (9.0, 25.8), (8.2, 29.6), false),
            limb((14.0, 21.5), (15.6, 25.6), (16.6, 29.4), false),
        ],
        closed_eyes: !eyes_open,
        ..front_stand(View::Front)
    }
}

/// Sitting up, facing right: legs out in front, propped on one hand.
pub fn sitting() -> Pose {
    Pose {
        view: View::Side,
        head: (9.4, 11.4),
        neck: (8.8, 17.4),
        pelvis: (9.6, 25.2),
        shoulder_w: 3.5,
        hip_w: 3.2,
        arms: [
            limb((9.4, 18.8), (7.4, 22.4), (5.6, 26.6), false),
            limb((8.6, 19.0), (11.0, 22.0), (13.8, 23.6), true),
        ],
        legs: [
            limb((10.0, 25.6), (15.4, 25.2), (20.2, 28.4), false),
            limb((9.4, 25.8), (14.6, 24.4), (19.2, 28.6), false),
        ],
        tool: None,
        closed_eyes: false,
    }
}

/// Getting up, facing right: one knee down, a hand pushing off the other.
pub fn kneeling() -> Pose {
    Pose {
        view: View::Side,
        head: (13.6, 11.4),
        neck: (12.8, 17.2),
        pelvis: (11.0, 24.6),
        shoulder_w: 3.5,
        hip_w: 3.2,
        arms: [
            limb((13.0, 18.4), (13.0, 22.0), (13.6, 25.4), false),
            limb((12.4, 18.6), (14.6, 21.6), (15.8, 24.0), true),
        ],
        legs: [
            limb((11.2, 25.0), (12.2, 29.2), (6.8, 29.6), false),
            limb((10.8, 25.0), (16.0, 24.6), (16.2, 29.6), false),
        ],
        tool: None,
        closed_eyes: false,
    }
}

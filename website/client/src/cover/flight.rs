//! Motion of the little guide between controls and along the scrolling page.
#[derive(Clone, Copy, Default)]
pub(super) struct Point {
    pub x: f64,
    pub y: f64,
}
impl Point {
    pub(super) fn distance(self, to: Point) -> f64 {
        (self.x - to.x).hypot(self.y - to.y)
    }
    pub(super) fn blend(self, to: Point, u: f64) -> Point {
        let t = smooth(u.clamp(0.0, 1.0));
        Point {
            x: self.x + (to.x - self.x) * t,
            y: self.y + (to.y - self.y) * t,
        }
    }
}
fn smooth(u: f64) -> f64 {
    u.powi(3) * (10.0 + u * (-15.0 + 6.0 * u))
}
fn duration(span: f64) -> f64 {
    (0.08 + 0.017 * span.abs().sqrt()).clamp(0.12, 0.49)
}

struct Trip {
    start: f64,
    from: Point,
    velocity: Point,
    delay: f64,
    tx: f64,
    ty: f64,
}

#[derive(PartialEq, Clone, Copy)]
pub(super) enum Mode {
    Pointer,
    Scroll,
    Held,
    Menu,
}

#[derive(Default)]
pub(super) struct Flight {
    pub position: Point,
    pub stretch: f64,
    pub shear: f64,
    velocity: Point,
    goal: Point,
    trip: Option<Trip>,
    last: f64,
}

impl Flight {
    pub fn place(&mut self, p: Point) {
        self.position = p;
        self.goal = p;
    }
    pub fn aim(&mut self, to: Point, mode: Mode, now: f64) {
        if mode == Mode::Pointer && self.goal.distance(to) > 0.35 {
            let dx = to.x - self.position.x;
            let dy = to.y - self.position.y;
            let ty = duration(dy);
            let delay = if self.velocity.x.abs() < 40.0
                && self.velocity.y.abs() < 40.0
                && dy.abs() > 46.0
                && dx.abs() > 28.0
            {
                ty * 0.86
            } else {
                0.0
            };
            self.trip = Some(Trip {
                start: now,
                from: self.position,
                velocity: self.velocity,
                delay,
                tx: duration(dx),
                ty,
            });
        }
        self.goal = to;
    }
    pub fn step(&mut self, now: f64, mode: Mode, reduced: bool) -> bool {
        let dt = ((now - self.last) / 1000.0).clamp(0.0, 0.064);
        self.last = now;
        if reduced || matches!(mode, Mode::Held | Mode::Menu) {
            self.position = self.goal;
            self.velocity = Point::default();
            self.trip = None;
        } else if mode == Mode::Scroll || self.trip.is_none() {
            let decay = (-18.0 * dt).exp();
            let follow = |p: f64, v: f64, goal: f64| {
                let d = p - goal;
                let slope = v + 18.0 * d;
                (
                    goal + (d + slope * dt) * decay,
                    (v - 18.0 * slope * dt) * decay,
                )
            };
            (self.position.x, self.velocity.x) =
                follow(self.position.x, self.velocity.x, self.goal.x);
            (self.position.y, self.velocity.y) =
                follow(self.position.y, self.velocity.y, self.goal.y);
            self.trip = None;
        } else if let Some(m) = &self.trip {
            let t = (now - m.start) / 1000.0;
            // The carry term matches incoming velocity and vanishes at the destination.
            let axis = |from: f64, to: f64, speed: f64, t: f64, seconds: f64| {
                let u = (t / seconds).clamp(0.0, 1.0);
                let rest = 1.0 - u;
                (
                    from + (to - from) * smooth(u) + speed * seconds * u * rest.powi(3),
                    (to - from) * 30.0 * u * u * rest * rest / seconds
                        + speed * rest * rest * (1.0 - 4.0 * u),
                )
            };
            (self.position.x, self.velocity.x) =
                axis(m.from.x, self.goal.x, m.velocity.x, t - m.delay, m.tx);
            (self.position.y, self.velocity.y) = axis(m.from.y, self.goal.y, m.velocity.y, t, m.ty);
            if t >= (m.tx + m.delay).max(m.ty) {
                self.trip = None;
            }
        }
        let resting = self.trip.is_none()
            && self.position.distance(self.goal) < 0.35
            && self.velocity.x.hypot(self.velocity.y) < 5.0;
        if resting {
            self.position = self.goal;
            self.velocity = Point::default();
        }
        let (vx, vy) = (self.velocity.x, self.velocity.y);
        let speed = vx.hypot(vy);
        let power = (speed / 10500.0).min(0.13);
        let norm = (speed * speed).max(1.0);
        let blend = if reduced {
            1.0
        } else {
            1.0 - (-dt * 24.0).exp()
        };
        self.stretch += (power * (vx * vx - vy * vy) / norm - self.stretch) * blend;
        self.shear += (power * 2.0 * vx * vy / norm - self.shear) * blend;
        resting
    }
}

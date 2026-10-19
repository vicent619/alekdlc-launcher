use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

static FROZEN: AtomicBool = AtomicBool::new(false);

pub fn freeze() {
    FROZEN.store(true, Ordering::Relaxed);
}

fn frozen() -> bool {
    FROZEN.load(Ordering::Relaxed)
}

#[derive(Clone, Copy, Debug)]
pub struct Bezier {
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
}

impl Bezier {
    pub const EASE: Self = Self::new(0.16, 1.0, 0.3, 1.0);
    pub const EASE_IN_OUT: Self = Self::new(0.65, 0.0, 0.35, 1.0);
    pub const LINEAR: Self = Self::new(0.0, 0.0, 1.0, 1.0);

    pub const fn new(x1: f32, y1: f32, x2: f32, y2: f32) -> Self {
        Self { x1, y1, x2, y2 }
    }

    fn curve(a: f32, b: f32, t: f32) -> f32 {
        let u = 1.0 - t;
        3.0 * u * u * t * a + 3.0 * u * t * t * b + t * t * t
    }

    fn slope(a: f32, b: f32, t: f32) -> f32 {
        let u = 1.0 - t;
        3.0 * u * u * a + 6.0 * u * t * (b - a) + 3.0 * t * t * (1.0 - b)
    }

    pub fn apply(&self, x: f32) -> f32 {
        let x = x.clamp(0.0, 1.0);
        if x == 0.0 || x == 1.0 {
            return x;
        }
        let mut t = x;
        for _ in 0..8 {
            let error = Self::curve(self.x1, self.x2, t) - x;
            let d = Self::slope(self.x1, self.x2, t);
            if error.abs() < 1e-6 || d.abs() < 1e-6 {
                break;
            }
            t = (t - error / d).clamp(0.0, 1.0);
        }
        let (mut lo, mut hi) = (0.0f32, 1.0f32);
        while (Self::curve(self.x1, self.x2, t) - x).abs() > 1e-5 && hi - lo > 1e-7 {
            if Self::curve(self.x1, self.x2, t) < x {
                lo = t;
            } else {
                hi = t;
            }
            t = (lo + hi) * 0.5;
        }
        Self::curve(self.y1, self.y2, t)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Tween {
    from: f32,
    to: f32,
    start: Instant,
    duration: f32,
    easing: Bezier,
}

impl Tween {
    pub fn at(value: f32) -> Self {
        Self {
            from: value,
            to: value,
            start: Instant::now(),
            duration: 0.0,
            easing: Bezier::EASE,
        }
    }

    pub fn eased(mut self, easing: Bezier) -> Self {
        self.easing = easing;
        self
    }

    pub fn over(mut self, duration: f32) -> Self {
        self.duration = duration;
        self
    }

    pub fn from(mut self, value: f32) -> Self {
        self.from = value;
        self.start = Instant::now();
        self
    }

    pub fn target(&self) -> f32 {
        self.to
    }

    pub fn set(&mut self, to: f32) {
        if (to - self.to).abs() < f32::EPSILON {
            return;
        }
        self.from = self.value();
        self.to = to;
        self.start = Instant::now();
    }

    pub fn progress(&self) -> f32 {
        if self.duration <= 0.0 || frozen() {
            return 1.0;
        }
        (self.start.elapsed().as_secs_f32() / self.duration).min(1.0)
    }

    pub fn value(&self) -> f32 {
        self.from + (self.to - self.from) * self.easing.apply(self.progress())
    }

    pub fn running(&self) -> bool {
        self.progress() < 1.0
    }
}

pub struct Clock {
    origin: Instant,
}

impl Clock {
    pub fn new() -> Self {
        Self { origin: Instant::now() }
    }

    pub fn seconds(&self) -> f32 {
        if frozen() {
            return 0.0;
        }
        self.origin.elapsed().as_secs_f32()
    }
}

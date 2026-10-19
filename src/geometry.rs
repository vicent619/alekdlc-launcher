#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    pub fn centered(center: Point, w: f32, h: f32) -> Self {
        Self::new(center.x - w * 0.5, center.y - h * 0.5, w, h)
    }

    pub fn right(&self) -> f32 {
        self.x + self.w
    }

    pub fn bottom(&self) -> f32 {
        self.y + self.h
    }

    pub fn center(&self) -> Point {
        Point::new(self.x + self.w * 0.5, self.y + self.h * 0.5)
    }

    pub fn contains(&self, p: Point) -> bool {
        p.x >= self.x && p.x < self.right() && p.y >= self.y && p.y < self.bottom()
    }

    pub fn inset(&self, dx: f32, dy: f32) -> Self {
        Self::new(self.x + dx, self.y + dy, self.w - dx * 2.0, self.h - dy * 2.0)
    }

    pub fn offset(&self, dx: f32, dy: f32) -> Self {
        Self::new(self.x + dx, self.y + dy, self.w, self.h)
    }

    pub fn scaled_about(&self, origin: Point, s: f32) -> Self {
        Self::new(
            origin.x + (self.x - origin.x) * s,
            origin.y + (self.y - origin.y) * s,
            self.w * s,
            self.h * s,
        )
    }
}

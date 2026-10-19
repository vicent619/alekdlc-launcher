#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const fn rgb(hex: u32) -> Self {
        Self::rgba(hex, 1.0)
    }

    pub const fn rgba(hex: u32, a: f32) -> Self {
        Self {
            r: ((hex >> 16) & 0xff) as f32 / 255.0,
            g: ((hex >> 8) & 0xff) as f32 / 255.0,
            b: (hex & 0xff) as f32 / 255.0,
            a,
        }
    }

    pub const TRANSPARENT: Self = Self::rgba(0, 0.0);

    pub fn alpha(self, factor: f32) -> Self {
        Self { a: self.a * factor, ..self }
    }

    pub fn mix(self, other: Self, t: f32) -> Self {
        let lerp = |a: f32, b: f32| a + (b - a) * t;
        Self {
            r: lerp(self.r, other.r),
            g: lerp(self.g, other.g),
            b: lerp(self.b, other.b),
            a: lerp(self.a, other.a),
        }
    }

    pub fn luminance(self) -> f32 {
        0.2126 * self.r + 0.7152 * self.g + 0.0722 * self.b
    }

    pub fn array(self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }
}

pub struct Palette;

impl Palette {
    pub const BG: Color = Color::rgb(0x000000);
    pub const SURFACE: Color = Color::rgb(0x111113);
    pub const SURFACE_2: Color = Color::rgb(0x19191c);
    pub const SURFACE_3: Color = Color::rgb(0x242428);
    pub const BORDER: Color = Color::rgba(0xffffff, 0.08);
    pub const BORDER_STRONG: Color = Color::rgba(0xffffff, 0.15);
    pub const TEXT: Color = Color::rgb(0xf5f5f7);
    pub const TEXT_2: Color = Color::rgb(0xa1a1a6);
    pub const TEXT_3: Color = Color::rgb(0x86868b);
    pub const PRIMARY: Color = Color::rgb(0xf5f5f7);
    pub const PRIMARY_TEXT: Color = Color::rgb(0x000000);
    pub const PRIMARY_HOVER: Color = Color::rgb(0xffffff);
    pub const FOCUS: Color = Color::rgb(0x0a84ff);
    pub const GREEN: Color = Color::rgb(0x30d158);
    pub const RED: Color = Color::rgb(0xff453a);
    pub const AMBER: Color = Color::rgb(0xff9f0a);
    pub const BLUE: Color = Color::rgb(0x0a84ff);
    pub const VIOLET: Color = Color::rgb(0xbf5af2);
    pub const PINK: Color = Color::rgb(0xff375f);
    pub const GREEN_SOFT: Color = Color::rgba(0x30d158, 0.14);
    pub const RED_SOFT: Color = Color::rgba(0xff453a, 0.14);
    pub const AMBER_SOFT: Color = Color::rgba(0xff9f0a, 0.15);
    pub const BLUE_SOFT: Color = Color::rgba(0x0a84ff, 0.15);
    pub const VIOLET_SOFT: Color = Color::rgba(0xbf5af2, 0.15);
    pub const PINK_SOFT: Color = Color::rgba(0xff375f, 0.15);
    pub const WINDOW_RING: Color = Color::rgba(0xffffff, 0.09);
    pub const GLOW: Color = Color::rgba(0xffffff, 0.09);
}

pub struct Metrics;

impl Metrics {
    pub const WINDOW_W: f32 = 960.0;
    pub const WINDOW_H: f32 = 600.0;
    pub const WINDOW_RADIUS: f32 = 14.0;
    pub const BAR_H: f32 = 56.0;
    pub const RADIUS_XS: f32 = 8.0;
    pub const RADIUS_SM: f32 = 12.0;
    pub const RADIUS: f32 = 16.0;
    pub const RADIUS_LG: f32 = 22.0;
    pub const PILL: f32 = 999.0;
}

pub struct Timing;

impl Timing {
    pub const FAST: f32 = 0.150;
    pub const BASE: f32 = 0.260;
    pub const SLOW: f32 = 0.480;
}

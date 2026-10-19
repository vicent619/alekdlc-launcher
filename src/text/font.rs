use swash::FontRef;

static INTER: &[u8] = include_bytes!("../../assets/fonts/InterVariable.ttf");

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextStyle {
    pub size: f32,
    pub weight: f32,
    pub tracking: f32,
    pub line_height: f32,
    pub tabular: bool,
}

impl TextStyle {
    pub const fn new(size: f32, weight: f32) -> Self {
        Self {
            size,
            weight,
            tracking: -0.011,
            line_height: 1.5,
            tabular: false,
        }
    }

    pub const fn tracking(mut self, em: f32) -> Self {
        self.tracking = em;
        self
    }

    pub const fn line_height(mut self, factor: f32) -> Self {
        self.line_height = factor;
        self
    }

    pub const fn tabular(mut self) -> Self {
        self.tabular = true;
        self
    }

    pub fn optical_size(&self) -> f32 {
        self.size.clamp(14.0, 32.0)
    }
}

pub struct Font;

impl Font {
    pub fn inter() -> FontRef<'static> {
        FontRef::from_index(INTER, 0).expect("inter font")
    }
}

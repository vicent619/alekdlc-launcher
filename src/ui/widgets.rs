use crate::geometry::{Point, Rect};
use crate::gfx::canvas::{Canvas, Layer};
use crate::text::font::TextStyle;
use crate::theme::{Color, Metrics, Palette};
use crate::ui::interaction::{Hover, Pointer};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonVariant {
    Primary,
    Outline,
    Ghost,
    Neutral,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonSize {
    Large,
    Medium,
    Small,
}

impl ButtonSize {
    fn height(self) -> f32 {
        match self {
            ButtonSize::Large => 52.0,
            ButtonSize::Medium => 44.0,
            ButtonSize::Small => 34.0,
        }
    }

    fn font(self) -> f32 {
        match self {
            ButtonSize::Large => 16.0,
            ButtonSize::Medium => 15.0,
            ButtonSize::Small => 14.0,
        }
    }

    fn icon(self) -> f32 {
        match self {
            ButtonSize::Large => 17.0,
            ButtonSize::Medium => 17.0,
            ButtonSize::Small => 15.0,
        }
    }

    fn gap(self) -> f32 {
        if self == ButtonSize::Small { 6.0 } else { 8.0 }
    }
}

pub struct Button {
    pub variant: ButtonVariant,
    pub size: ButtonSize,
    pub icon: Option<&'static str>,
    pub label: String,
    pub loading: bool,
    pub disabled: bool,
    pub radius: f32,
    state: Hover,
}

impl Button {
    pub fn new(variant: ButtonVariant, size: ButtonSize, label: impl Into<String>) -> Self {
        Self {
            variant,
            size,
            icon: None,
            label: label.into(),
            loading: false,
            disabled: false,
            radius: Metrics::PILL,
            state: Hover::new(),
        }
    }

    pub fn icon(mut self, icon: &'static str) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    pub fn height(&self) -> f32 {
        self.size.height()
    }

    fn colors(&self, hover: f32) -> (Color, Color, Option<Color>) {
        match self.variant {
            ButtonVariant::Primary => (
                Palette::PRIMARY.mix(Palette::PRIMARY_HOVER, hover),
                Palette::PRIMARY_TEXT,
                None,
            ),
            ButtonVariant::Neutral => (
                Palette::SURFACE_2.mix(Palette::SURFACE_3, hover),
                Palette::TEXT,
                None,
            ),
            ButtonVariant::Outline => (
                Color::rgba(0xf5f5f7, 0.05 * hover),
                Palette::TEXT,
                Some(Palette::BORDER_STRONG),
            ),
            ButtonVariant::Ghost => (Color::rgba(0xf5f5f7, 0.07 * hover), Palette::TEXT, None),
        }
    }

    pub fn draw(&self, canvas: &mut Canvas, area: Rect) {
        let hover = self.state.hover();
        let scale = 1.0 - 0.03 * self.state.pressed();
        let opacity = if self.disabled { 0.45 } else { 1.0 };
        let (bg, fg, border) = self.colors(hover);
        canvas.with(Layer::new().scale(scale, area.center()).opacity(opacity), |c| {
            c.fill(area, self.radius, bg);
            if let Some(color) = border {
                c.inset_border(area, self.radius, 1.0, color);
            }
            if self.loading {
                return;
            }
            let style = TextStyle::new(self.size.font(), 600.0).tracking(-0.012);
            let layout = c.layout(&self.label, style);
            let icon_size = self.size.icon();
            let gap = self.size.gap();
            let content = layout.width + self.icon.map(|_| icon_size + gap).unwrap_or(0.0);
            let mut x = area.center().x - content * 0.5;
            if let Some(icon) = self.icon {
                c.icon(icon, Rect::new(x, area.center().y - icon_size * 0.5, icon_size, icon_size), fg);
                x += icon_size + gap;
            }
            let baseline = layout.baseline_in(area.y + (area.h - layout.line_height()) * 0.5, layout.line_height());
            c.draw_layout(&layout, x, baseline, fg);
        });
    }

    pub fn track(&mut self, area: Rect, pointer: &Pointer) {
        if self.disabled {
            return;
        }
        self.state.track(area, pointer);
    }

    pub fn press(&mut self, area: Rect, pointer: &Pointer) -> bool {
        if self.disabled {
            return false;
        }
        self.state.press(area, pointer)
    }

    pub fn release(&mut self, area: Rect, pointer: &Pointer) -> bool {
        if self.disabled {
            return false;
        }
        self.state.release(area, pointer)
    }

    pub fn animating(&self) -> bool {
        self.state.animating()
    }
}

pub struct IconTile;

impl IconTile {
    pub fn large(canvas: &mut Canvas, center: Point, icon: &'static str, fill: Color, tint: Color) {
        let rect = Rect::centered(center, 52.0, 52.0);
        canvas.fill(rect, 16.0, fill);
        canvas.icon(icon, Rect::centered(center, 24.0, 24.0), tint);
    }
}

pub fn error_banner(canvas: &mut Canvas, area: Rect, message: &str) -> f32 {
    let style = TextStyle::new(14.0, 400.0).line_height(1.45);
    let text_x = area.x + 12.0 + 16.0 + 8.0;
    let text_width = area.right() - 12.0 - text_x;
    let lines = canvas.wrap(message, style, text_width);
    let line_height = lines.first().map(|l| l.line_height()).unwrap_or(0.0);
    let height = (line_height * lines.len() as f32 + 20.0).max(40.0);
    let rect = Rect::new(area.x, area.y, area.w, height);
    canvas.fill(rect, Metrics::RADIUS_SM, Palette::RED_SOFT);
    canvas.icon("alert", Rect::new(rect.x + 12.0, rect.y + (height - 16.0) * 0.5, 16.0, 16.0), Palette::RED);
    let mut y = rect.y + 10.0;
    for layout in &lines {
        let baseline = layout.baseline_in(y, line_height);
        canvas.draw_layout(layout, text_x, baseline, Palette::RED);
        y += line_height;
    }
    height
}

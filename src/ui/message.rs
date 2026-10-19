use crate::geometry::{Point, Rect};
use crate::gfx::canvas::Canvas;
use crate::text::font::TextStyle;
use crate::theme::{Color, Metrics, Palette};
use crate::ui::interaction::Pointer;
use crate::ui::widgets::{Button, ButtonSize, ButtonVariant, IconTile};

const CARD_W: f32 = 360.0;
const CENTER_X: f32 = Metrics::WINDOW_W * 0.5;
const BAND_TOP: f32 = Metrics::BAR_H;
const BAND_BOTTOM: f32 = Metrics::WINDOW_H - Metrics::BAR_H;

pub struct MessageScreen {
    icon: &'static str,
    tile_fill: Color,
    tile_tint: Color,
    title_key: &'static str,
    lead_key: &'static str,
    button: Button,
}

impl MessageScreen {
    pub fn offline() -> Self {
        Self {
            icon: "cloud",
            tile_fill: Palette::RED_SOFT,
            tile_tint: Palette::RED,
            title_key: "offline.title",
            lead_key: "offline.lead",
            button: Button::new(ButtonVariant::Primary, ButtonSize::Medium, String::new()).icon("refresh"),
        }
    }

    pub fn update() -> Self {
        Self {
            icon: "download",
            tile_fill: Palette::BLUE_SOFT,
            tile_tint: Palette::BLUE,
            title_key: "update.title",
            lead_key: "update.lead",
            button: Button::new(ButtonVariant::Primary, ButtonSize::Medium, String::new()).icon("download"),
        }
    }

    fn button_area(&self, card_top: f32, lead_height: f32) -> Rect {
        let y = card_top + 52.0 + 20.0 + 30.8 + 10.0 + lead_height + 28.0;
        Rect::new(CENTER_X - CARD_W * 0.5, y, CARD_W, 44.0)
    }

    fn lead_lines(&self, canvas: &mut Canvas, lead: &str) -> f32 {
        let style = TextStyle::new(15.0, 400.0);
        canvas.wrap(lead, style, CARD_W).len() as f32 * 22.5
    }

    pub fn layout(&self, canvas: &mut Canvas, lead: &str) -> (f32, f32, Rect) {
        let lead_height = self.lead_lines(canvas, lead);
        let card_height = 52.0 + 20.0 + 30.8 + 10.0 + lead_height + 28.0 + 44.0;
        let card_top = (BAND_TOP + BAND_BOTTOM) * 0.5 - card_height * 0.5;
        (card_top, lead_height, self.button_area(card_top, lead_height))
    }

    pub fn set_label(&mut self, label: impl Into<String>) {
        self.button.label = label.into();
    }

    pub fn draw(&self, canvas: &mut Canvas, title: &str, lead: &str) {
        let (card_top, _, button_area) = self.layout(canvas, lead);
        IconTile::large(
            canvas,
            Point::new(CENTER_X, card_top + 26.0),
            self.icon,
            self.tile_fill,
            self.tile_tint,
        );
        let title_style = TextStyle::new(28.0, 700.0).tracking(-0.03).line_height(1.1);
        canvas.text_centered(title, title_style, CENTER_X, card_top + 72.0, Palette::TEXT);
        let lead_style = TextStyle::new(15.0, 400.0);
        canvas.paragraph_centered(lead, lead_style, CENTER_X, card_top + 112.8, CARD_W, Palette::TEXT_2);
        self.button.draw(canvas, button_area);
    }

    pub fn button_rect(&self, canvas: &mut Canvas, lead: &str) -> Rect {
        self.layout(canvas, lead).2
    }

    pub fn track(&mut self, area: Rect, pointer: &Pointer) {
        self.button.track(area, pointer);
    }

    pub fn press(&mut self, area: Rect, pointer: &Pointer) -> bool {
        self.button.press(area, pointer)
    }

    pub fn release(&mut self, area: Rect, pointer: &Pointer) -> bool {
        self.button.release(area, pointer)
    }

    pub fn animating(&self) -> bool {
        self.button.animating()
    }
}

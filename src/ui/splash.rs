use std::f32::consts::TAU;

use crate::anim::{Bezier, Clock};
use crate::geometry::{Point, Rect};
use crate::gfx::canvas::{Canvas, Layer};
use crate::text::font::TextStyle;
use crate::theme::{Metrics, Palette};

const MARK: f32 = 56.0;
const GAP: f32 = 22.0;
const SPINNER: f32 = 16.0;
const BREATHE: f32 = 2.4;
const SPIN: f32 = 0.8;

pub struct Splash {
    clock: Clock,
    status: String,
}

impl Splash {
    pub fn new(status: impl Into<String>) -> Self {
        Self {
            clock: Clock::new(),
            status: status.into(),
        }
    }

    pub fn set_status(&mut self, status: impl Into<String>) {
        self.status = status.into();
    }

    fn breathe(&self) -> (f32, f32) {
        let t = (self.clock.seconds() % BREATHE) / BREATHE;
        let (from, to, local) = if t < 0.5 { (0.0, 1.0, t * 2.0) } else { (1.0, 0.0, (t - 0.5) * 2.0) };
        let k = from + (to - from) * Bezier::EASE_IN_OUT.apply(local);
        (1.0 - 0.45 * k, 1.0 - 0.04 * k)
    }

    pub fn draw(&self, canvas: &mut Canvas) {
        let top = Metrics::BAR_H;
        let bottom = Metrics::WINDOW_H - Metrics::BAR_H;
        let center_y = (top + bottom) * 0.5;
        let style = TextStyle::new(14.0, 400.0);
        let layout = canvas.layout(&self.status, style);
        let row_h = layout.line_height();
        let block = MARK + GAP + row_h;
        let y = center_y - block * 0.5;
        let cx = Metrics::WINDOW_W * 0.5;
        let mark = Rect::new(cx - MARK * 0.5, y, MARK, MARK);
        let (opacity, scale) = self.breathe();
        canvas.with(Layer::new().opacity(opacity).scale(scale, mark.center()), |c| {
            c.icon("mark", mark, Palette::TEXT);
        });
        let row_y = y + MARK + GAP;
        let row_w = SPINNER + 10.0 + layout.width;
        let row_x = cx - row_w * 0.5;
        let angle = (self.clock.seconds() % SPIN) / SPIN * TAU;
        canvas.spinner(
            Point::new(row_x + SPINNER * 0.5, row_y + row_h * 0.5),
            SPINNER,
            2.0,
            angle,
            Palette::TEXT,
            Palette::SURFACE_3,
        );
        let baseline = layout.baseline_in(row_y, row_h);
        canvas.draw_layout(&layout, row_x + SPINNER + 10.0, baseline, Palette::TEXT_2);
    }
}

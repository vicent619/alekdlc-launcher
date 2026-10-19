use crate::anim::Bezier;
use crate::core::state::{State, ToastKind};
use crate::geometry::{Point, Rect};
use crate::gfx::canvas::{Canvas, Layer, Shadow};
use crate::text::font::TextStyle;
use crate::theme::{Color, Metrics, Palette};

const MARGIN: f32 = 20.0;
const GAP: f32 = 8.0;
const PAD_X: f32 = 16.0;
const HEIGHT: f32 = 50.0;
const ICON: f32 = 18.0;
const MIN_W: f32 = 280.0;
const MAX_W: f32 = 380.0;

pub fn draw(canvas: &mut Canvas, state: &State) {
    let mut bottom = Metrics::WINDOW_H - MARGIN;
    for toast in state.toasts.iter().rev() {
        let style = TextStyle::new(14.0, 400.0);
        let layout = canvas.layout(&toast.text, style);
        let width = (PAD_X + ICON + 12.0 + layout.width + PAD_X).clamp(MIN_W, MAX_W);
        let rect = Rect::new(Metrics::WINDOW_W - MARGIN - width, bottom - HEIGHT, width, HEIGHT);
        let appear = (toast.born.elapsed().as_secs_f32() / 0.42).min(1.0);
        let eased = Bezier::EASE.apply(appear);
        let (opacity, shift) = match toast.dying {
            Some(start) => {
                let t = (start.elapsed().as_secs_f32() / 0.2).min(1.0);
                (1.0 - t, 8.0 * t)
            }
            None => (eased, 16.0 * (1.0 - eased)),
        };
        canvas.with(Layer::new().opacity(opacity).translate(0.0, shift), |c| {
            c.shadow(rect, Metrics::RADIUS, Shadow::new(0.0, 24.0, 64.0, -24.0, Color::rgba(0, 0.9)));
            c.fill(rect, Metrics::RADIUS, Palette::SURFACE_2);
            c.inset_border(rect, Metrics::RADIUS, 1.0, Color::rgba(0xffffff, 0.06));
            let (icon, color) = glyph(toast.kind);
            c.icon(icon, Rect::centered(Point::new(rect.x + PAD_X + ICON * 0.5, rect.center().y), ICON, ICON), color);
            let baseline = layout.baseline_in(rect.center().y - layout.line_height() * 0.5, layout.line_height());
            c.draw_layout(&layout, rect.x + PAD_X + ICON + 12.0, baseline, Palette::TEXT);
        });
        bottom -= HEIGHT + GAP;
    }
}

fn glyph(kind: ToastKind) -> (&'static str, Color) {
    match kind {
        ToastKind::Success => ("circle-check", Palette::GREEN),
        ToastKind::Error => ("circle-x", Palette::RED),
        ToastKind::Info => ("info", Palette::TEXT_2),
    }
}

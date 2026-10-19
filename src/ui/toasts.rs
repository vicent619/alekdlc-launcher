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
const ICON_GAP: f32 = 12.0;
const PAD_Y: f32 = 14.0;
const MIN_W: f32 = 280.0;
const MAX_W: f32 = 400.0;

pub fn draw(canvas: &mut Canvas, state: &State) {
    let mut bottom = Metrics::WINDOW_H - MARGIN;
    for toast in state.toasts.iter().rev() {
        let style = TextStyle::new(14.0, 400.0).line_height(1.45);
        let lines = canvas.wrap(&toast.text, style, MAX_W - PAD_X * 2.0 - ICON - ICON_GAP);
        let line_h = lines.first().map(|l| l.line_height()).unwrap_or(0.0);
        let text_w = lines.iter().map(|l| l.width).fold(0.0, f32::max);
        let text_h = line_h * lines.len() as f32;
        let width = (PAD_X + ICON + ICON_GAP + text_w + PAD_X).clamp(MIN_W, MAX_W);
        let height = (text_h + PAD_Y * 2.0).max(HEIGHT);
        let rect = Rect::new(Metrics::WINDOW_W - MARGIN - width, bottom - height, width, height);
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
            c.icon(&icon, Rect::centered(Point::new(rect.x + PAD_X + ICON * 0.5, rect.center().y), ICON, ICON), color);
            let mut top = rect.center().y - text_h * 0.5;
            for line in &lines {
                c.draw_layout(line, rect.x + PAD_X + ICON + ICON_GAP, line.baseline_in(top, line_h), Palette::TEXT);
                top += line_h;
            }
        });
        bottom -= height + GAP;
    }
}

fn glyph(kind: ToastKind) -> (String, Color) {
    match kind {
        ToastKind::Success => (crate::secret!("circle-check").to_owned(), Palette::GREEN),
        ToastKind::Error => (crate::secret!("circle-x").to_owned(), Palette::RED),
        ToastKind::Info => (crate::secret!("info").to_owned(), Palette::TEXT_2),
    }
}

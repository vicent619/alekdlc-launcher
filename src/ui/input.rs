use crate::anim::{Bezier, Tween};
use crate::geometry::{Point, Rect};
use crate::gfx::canvas::Canvas;
use crate::text::font::TextStyle;
use crate::theme::{Color, Metrics, Palette, Timing};
use crate::ui::interaction::{Hover, Pointer};

pub struct TextField {
    pub icon: &'static str,
    pub placeholder: String,
    pub value: String,
    pub secret: bool,
    pub revealed: bool,
    pub invalid: bool,
    pub focused: bool,
    pub caret: usize,
    anchor: usize,
    hover: Hover,
    reveal: Hover,
    focus: Tween,
    blink: f32,
}

const HEIGHT: f32 = 46.0;
const ICON_X: f32 = 14.0;
const TEXT_X: f32 = 44.0;
const REVEAL_W: f32 = 34.0;

impl TextField {
    pub fn new(icon: &'static str, placeholder: impl Into<String>, secret: bool) -> Self {
        Self {
            icon,
            placeholder: placeholder.into(),
            value: String::new(),
            secret,
            revealed: false,
            invalid: false,
            focused: false,
            caret: 0,
            anchor: 0,
            hover: Hover::new(),
            reveal: Hover::new(),
            focus: Tween::at(0.0).over(Timing::FAST).eased(Bezier::EASE),
            blink: 0.0,
        }
    }

    pub fn height(&self) -> f32 {
        HEIGHT
    }

    fn len(&self) -> usize {
        self.value.chars().count()
    }

    fn display(&self) -> String {
        if self.secret && !self.revealed {
            "•".repeat(self.len())
        } else {
            self.value.clone()
        }
    }

    fn reveal_area(&self, area: Rect) -> Rect {
        Rect::new(area.right() - 6.0 - REVEAL_W, area.y + (area.h - REVEAL_W) * 0.5, REVEAL_W, REVEAL_W)
    }

    pub fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
        self.focus.set(if focused { 1.0 } else { 0.0 });
        self.blink = 0.0;
        if focused {
            self.caret = self.len();
            self.anchor = self.caret;
        }
    }

    pub fn has_selection(&self) -> bool {
        self.anchor != self.caret
    }

    fn selection(&self) -> (usize, usize) {
        (self.caret.min(self.anchor), self.caret.max(self.anchor))
    }

    pub fn select_all(&mut self) {
        self.anchor = 0;
        self.caret = self.len();
        self.blink = 0.0;
    }

    pub fn selected_text(&self) -> String {
        if self.secret && !self.revealed {
            return String::new();
        }
        let (lo, hi) = self.selection();
        self.value.chars().skip(lo).take(hi - lo).collect()
    }

    fn delete_selection(&mut self) -> bool {
        if !self.has_selection() {
            return false;
        }
        let (lo, hi) = self.selection();
        let start = self.byte_index(lo);
        let end = self.byte_index(hi);
        self.value.replace_range(start..end, "");
        self.caret = lo;
        self.anchor = lo;
        true
    }

    pub fn insert(&mut self, text: &str) {
        let filtered: String = text.chars().filter(|c| !c.is_control()).collect();
        if filtered.is_empty() {
            return;
        }
        self.delete_selection();
        let byte = self.byte_index(self.caret);
        self.value.insert_str(byte, &filtered);
        self.caret += filtered.chars().count();
        self.anchor = self.caret;
        self.invalid = false;
        self.blink = 0.0;
    }

    pub fn backspace(&mut self) {
        if self.delete_selection() {
            self.invalid = false;
            self.blink = 0.0;
            return;
        }
        if self.caret == 0 {
            return;
        }
        let start = self.byte_index(self.caret - 1);
        let end = self.byte_index(self.caret);
        self.value.replace_range(start..end, "");
        self.caret -= 1;
        self.anchor = self.caret;
        self.invalid = false;
        self.blink = 0.0;
    }

    pub fn move_caret(&mut self, delta: i32, extend: bool) {
        if !extend && self.has_selection() {
            let (lo, hi) = self.selection();
            self.caret = if delta < 0 { lo } else { hi };
            self.anchor = self.caret;
            self.blink = 0.0;
            return;
        }
        let len = self.len() as i32;
        self.caret = (self.caret as i32 + delta).clamp(0, len) as usize;
        if !extend {
            self.anchor = self.caret;
        }
        self.blink = 0.0;
    }

    pub fn caret_home(&mut self, extend: bool) {
        self.caret = 0;
        if !extend {
            self.anchor = 0;
        }
        self.blink = 0.0;
    }

    pub fn caret_end(&mut self, extend: bool) {
        self.caret = self.len();
        if !extend {
            self.anchor = self.caret;
        }
        self.blink = 0.0;
    }

    pub fn set_caret(&mut self, index: usize, extend: bool) {
        self.caret = index.min(self.len());
        if !extend {
            self.anchor = self.caret;
        }
        self.blink = 0.0;
    }

    pub fn caret_from_x(&self, canvas: &mut Canvas, area: Rect, x: f32) -> usize {
        let style = TextStyle::new(15.0, 400.0);
        let display = self.display();
        let text_x = area.x + TEXT_X;
        let chars = display.chars().count();
        let mut best = 0;
        let mut best_dx = f32::MAX;
        for i in 0..=chars {
            let prefix: String = display.chars().take(i).collect();
            let w = canvas.layout(&prefix, style).width;
            let dx = (text_x + w - x).abs();
            if dx < best_dx {
                best_dx = dx;
                best = i;
            }
        }
        best
    }

    fn byte_index(&self, chars: usize) -> usize {
        self.value
            .char_indices()
            .nth(chars)
            .map(|(i, _)| i)
            .unwrap_or(self.value.len())
    }

    pub fn tick(&mut self, dt: f32) {
        if self.focused {
            self.blink = (self.blink + dt) % 1.06;
        }
    }

    pub fn toggle_reveal(&mut self) {
        self.revealed = !self.revealed;
    }

    pub fn track(&mut self, area: Rect, pointer: &Pointer) {
        self.hover.track(area, pointer);
        if self.secret {
            self.reveal.track(self.reveal_area(area), pointer);
        }
    }

    pub fn hit_reveal(&self, area: Rect, pointer: &Pointer) -> bool {
        self.secret && self.reveal_area(area).contains(pointer.position)
    }

    pub fn draw(&mut self, canvas: &mut Canvas, area: Rect) {
        let focus = self.focus.value();
        let background = Palette::SURFACE_2.mix(Palette::SURFACE, focus);
        canvas.fill(area, Metrics::RADIUS_SM, background);
        if self.invalid {
            canvas.inset_border(area, Metrics::RADIUS_SM, 1.5, Palette::RED);
        } else if focus > 0.01 {
            let hover = self.hover.hover();
            let base = Palette::BORDER.mix(Palette::BORDER_STRONG, hover);
            canvas.inset_border(area, Metrics::RADIUS_SM, 1.0, base.alpha(1.0 - focus));
            canvas.inset_border(area, Metrics::RADIUS_SM, 1.5, Palette::FOCUS.alpha(focus));
            canvas.outline(area, Metrics::RADIUS_SM, 4.0, Palette::FOCUS.alpha(0.22 * focus));
        } else {
            let hover = self.hover.hover();
            canvas.inset_border(area, Metrics::RADIUS_SM, 1.0, Palette::BORDER.mix(Palette::BORDER_STRONG, hover));
        }
        let icon_color = if focus > 0.5 { Palette::FOCUS } else { Palette::TEXT_3 };
        canvas.icon(self.icon, Rect::new(area.x + ICON_X, area.center().y - 9.0, 18.0, 18.0), icon_color);
        let style = TextStyle::new(15.0, 400.0);
        let text = self.display();
        let has_text = !text.is_empty();
        let shown = if has_text { text.clone() } else { self.placeholder.clone() };
        let color = if has_text { Palette::TEXT } else { Palette::TEXT_3 };
        let text_x = area.x + TEXT_X;
        if self.focused && self.has_selection() && has_text {
            let (lo, hi) = self.selection();
            let lo_w = canvas.layout(&text.chars().take(lo).collect::<String>(), style).width;
            let hi_w = canvas.layout(&text.chars().take(hi).collect::<String>(), style).width;
            canvas.fill(Rect::new(text_x + lo_w, area.center().y - 10.0, hi_w - lo_w, 20.0), 2.0, Palette::FOCUS.alpha(0.3));
        }
        let layout = canvas.layout(&shown, style);
        let baseline = layout.baseline_in(area.y + (area.h - layout.line_height()) * 0.5, layout.line_height());
        canvas.draw_layout(&layout, text_x, baseline, color);
        if self.focused && !self.has_selection() && self.blink < 0.53 {
            let measured = canvas.layout(&self.char_prefix(&text), style);
            let caret_x = text_x + measured.width;
            let caret = Rect::new(caret_x, area.center().y - 9.0, 1.5, 18.0);
            canvas.fill(caret, 1.0, Palette::TEXT);
        }
        if self.secret {
            let reveal = self.reveal_area(area);
            let hover = self.reveal.hover();
            canvas.fill(reveal, Metrics::PILL, Color::rgba(0xf5f5f7, 0.08 * hover));
            let glyph = if self.revealed { "eye-off" } else { "eye" };
            canvas.icon(glyph, Rect::centered(reveal.center(), 16.0, 16.0), Palette::TEXT_2.mix(Palette::TEXT, hover));
        }
    }

    fn char_prefix(&self, display: &str) -> String {
        display.chars().take(self.caret).collect()
    }

    pub fn caret_point(&self, area: Rect) -> Point {
        Point::new(area.x + TEXT_X, area.center().y)
    }

    pub fn animating(&self) -> bool {
        self.focus.running() || self.hover.animating() || self.reveal.animating() || self.focused
    }
}

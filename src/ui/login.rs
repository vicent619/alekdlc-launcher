use crate::core::state::State;
use crate::geometry::{Point, Rect};
use crate::gfx::canvas::Canvas;
use crate::i18n::text;
use crate::text::font::TextStyle;
use crate::theme::{Metrics, Palette};
use crate::ui::input::TextField;
use crate::ui::interaction::{Hover, Pointer};
use crate::ui::widgets::{Button, ButtonSize, ButtonVariant, error_banner};

const CARD_W: f32 = 360.0;
const CENTER_X: f32 = Metrics::WINDOW_W * 0.5;
const CARD_LEFT: f32 = CENTER_X - CARD_W * 0.5;
const BAND_TOP: f32 = Metrics::BAR_H;
const BAND_BOTTOM: f32 = Metrics::WINDOW_H - Metrics::BAR_H;
const GAP: f32 = 14.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoginAction {
    None,
    Submit,
    Forgot,
}

pub struct LoginScreen {
    pub login: TextField,
    pub password: TextField,
    submit: Button,
    forgot: Hover,
    focus: usize,
    dragging: Option<usize>,
}

struct Geometry {
    card_top: f32,
    error_height: f32,
    inputs: [Rect; 2],
    forgot: Rect,
    submit: Rect,
}

impl LoginScreen {
    pub fn new(lang: crate::i18n::Language) -> Self {
        let mut screen = Self {
            login: TextField::new("user", String::new(), false),
            password: TextField::new("lock", String::new(), true),
            submit: Button::new(ButtonVariant::Primary, ButtonSize::Medium, String::new()).icon("log-in"),
            forgot: Hover::new(),
            focus: 0,
            dragging: None,
        };
        screen.relabel(lang);
        screen.login.set_focused(true);
        screen
    }

    pub fn relabel(&mut self, lang: crate::i18n::Language) {
        self.login.placeholder = text(lang, "login.login");
        self.password.placeholder = text(lang, "login.password");
        self.submit.label = text(lang, "login.submit");
    }

    fn geometry(&self, canvas: &mut Canvas, state: &State) -> Geometry {
        let error_height = state
            .login
            .error
            .as_ref()
            .map(|message| error_banner_height(canvas, message))
            .unwrap_or(0.0);
        let error_gap = if error_height > 0.0 { error_height + GAP } else { 0.0 };
        let form_height = error_gap + 46.0 + GAP + 46.0 + GAP + 22.5 + GAP + 6.0 + 44.0;
        let card_height = 44.0 + 20.0 + 30.8 + 10.0 + 22.5 + 28.0 + form_height;
        let card_top = (BAND_TOP + BAND_BOTTOM) * 0.5 - card_height * 0.5;
        let form_top = card_top + 44.0 + 20.0 + 30.8 + 10.0 + 22.5 + 28.0;
        let input1_top = form_top + error_gap;
        let inputs = [
            Rect::new(CARD_LEFT, input1_top, CARD_W, 46.0),
            Rect::new(CARD_LEFT, input1_top + 46.0 + GAP, CARD_W, 46.0),
        ];
        let forgot_row = inputs[1].bottom() + GAP;
        let submit = Rect::new(CARD_LEFT, forgot_row + 22.5 + GAP + 6.0, CARD_W, 44.0);
        Geometry {
            card_top,
            error_height,
            inputs,
            forgot: Rect::new(CARD_LEFT, forgot_row, CARD_W, 22.5),
            submit,
        }
    }

    pub fn draw(&mut self, canvas: &mut Canvas, state: &State) {
        let geometry = self.geometry(canvas, state);
        let lang = state.language;
        canvas.icon("mark", Rect::new(CENTER_X - 22.0, geometry.card_top, 44.0, 44.0), Palette::TEXT);
        let title_style = TextStyle::new(28.0, 700.0).tracking(-0.03).line_height(1.1);
        canvas.text_centered(&text(lang, "login.title"), title_style, CENTER_X, geometry.card_top + 64.0, Palette::TEXT);
        let lead_style = TextStyle::new(15.0, 400.0);
        canvas.text_centered(&text(lang, "login.lead"), lead_style, CENTER_X, geometry.card_top + 104.8, Palette::TEXT_2);
        let form_top = geometry.inputs[0].y - if geometry.error_height > 0.0 { geometry.error_height + GAP } else { 0.0 };
        if let Some(message) = &state.login.error {
            error_banner(canvas, Rect::new(CARD_LEFT, form_top, CARD_W, 0.0), message);
        }
        self.login.invalid = state.login.invalid;
        self.password.invalid = state.login.invalid;
        self.login.draw(canvas, geometry.inputs[0]);
        self.password.draw(canvas, geometry.inputs[1]);
        let forgot = &text(lang, "login.forgot");
        let link_style = TextStyle::new(14.0, 500.0);
        let link_layout = canvas.layout(forgot, link_style);
        let link_x = geometry.forgot.right() - link_layout.width;
        let baseline = link_layout.baseline_in(geometry.forgot.y, geometry.forgot.h);
        canvas.draw_layout(&link_layout, link_x, baseline, Palette::FOCUS);
        self.submit.loading = state.login.busy;
        self.submit.draw(canvas, geometry.submit);
    }

    fn forgot_area(&self, canvas: &mut Canvas, state: &State) -> Rect {
        let geometry = self.geometry(canvas, state);
        let forgot = &text(state.language, "login.forgot");
        let link_style = TextStyle::new(14.0, 500.0);
        let width = canvas.layout(forgot, link_style).width;
        Rect::new(geometry.forgot.right() - width, geometry.forgot.y, width, geometry.forgot.h)
    }

    pub fn track(&mut self, canvas: &mut Canvas, state: &State, pointer: &Pointer) {
        let geometry = self.geometry(canvas, state);
        self.login.track(geometry.inputs[0], pointer);
        self.password.track(geometry.inputs[1], pointer);
        self.submit.track(geometry.submit, pointer);
        self.forgot.track(self.forgot_area(canvas, state), pointer);
        if pointer.pressed {
            if let Some(field) = self.dragging {
                let (target, area) = if field == 0 {
                    (&mut self.login, geometry.inputs[0])
                } else {
                    (&mut self.password, geometry.inputs[1])
                };
                let idx = target.caret_from_x(canvas, area, pointer.position.x);
                target.set_caret(idx, true);
            }
        } else {
            self.dragging = None;
        }
    }

    pub fn press(&mut self, canvas: &mut Canvas, state: &State, pointer: &Pointer) {
        let geometry = self.geometry(canvas, state);
        self.submit.press(geometry.submit, pointer);
        self.dragging = None;
        if self.password.hit_reveal(geometry.inputs[1], pointer) {
            self.password.toggle_reveal();
            return;
        }
        if geometry.inputs[0].contains(pointer.position) {
            self.focus_field(0);
            let idx = self.login.caret_from_x(canvas, geometry.inputs[0], pointer.position.x);
            self.login.set_caret(idx, false);
            self.dragging = Some(0);
        } else if geometry.inputs[1].contains(pointer.position) {
            self.focus_field(1);
            let idx = self.password.caret_from_x(canvas, geometry.inputs[1], pointer.position.x);
            self.password.set_caret(idx, false);
            self.dragging = Some(1);
        }
    }

    pub fn release(&mut self, canvas: &mut Canvas, state: &State, pointer: &Pointer) -> LoginAction {
        self.dragging = None;
        let geometry = self.geometry(canvas, state);
        if self.submit.release(geometry.submit, pointer) {
            return LoginAction::Submit;
        }
        if self.forgot_area(canvas, state).contains(pointer.position) && !pointer.pressed {
            return LoginAction::Forgot;
        }
        LoginAction::None
    }

    pub fn focus_field(&mut self, index: usize) {
        self.focus = index;
        self.login.set_focused(index == 0);
        self.password.set_focused(index == 1);
    }

    pub fn focused(&mut self) -> &mut TextField {
        if self.focus == 0 {
            &mut self.login
        } else {
            &mut self.password
        }
    }

    pub fn advance_focus(&mut self) {
        self.focus_field((self.focus + 1) % 2);
    }

    pub fn tick(&mut self, dt: f32) {
        self.login.tick(dt);
        self.password.tick(dt);
    }

    pub fn animating(&self) -> bool {
        self.login.animating() || self.password.animating() || self.submit.animating() || self.forgot.animating()
    }

    pub fn reset(&mut self) {
        self.login.value.clear();
        self.password.value.clear();
        self.login.caret = 0;
        self.password.caret = 0;
        self.focus_field(0);
    }

    pub fn caret_anchor(&self, _state: &State) -> Point {
        Point::new(CARD_LEFT, 0.0)
    }
}

fn error_banner_height(canvas: &mut Canvas, message: &str) -> f32 {
    let style = TextStyle::new(14.0, 400.0).line_height(1.45);
    let text_x = 12.0 + 16.0 + 8.0;
    let lines = canvas.wrap(message, style, CARD_W - 12.0 - text_x);
    (lines.first().map(|l| l.line_height()).unwrap_or(0.0) * lines.len() as f32 + 20.0).max(40.0)
}

use crate::core::state::State;
use crate::geometry::{Point, Rect};
use crate::gfx::canvas::Canvas;
use crate::i18n::{format, format_bytes, text};
use crate::net::api::Profile;
use crate::text::font::TextStyle;
use crate::theme::{Color, Metrics, Palette};
use crate::ui::interaction::{Hover, Pointer};
use crate::ui::widgets::{Button, ButtonSize, ButtonVariant};

const PAD_X: f32 = 40.0;
const CARD_GAP: f32 = 14.0;
const CARD_W: f32 = 433.0;
const CARD_H: f32 = 173.1;
const CARD_Y: f32 = 258.4;
const CARD_RADIUS: f32 = Metrics::RADIUS_LG;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HomeAction {
    None,
    Play,
    Cancel,
    Buy,
    Cabinet,
}

pub struct HomeScreen {
    play: Button,
    cancel: Button,
    buy: Hover,
    cabinet: Hover,
    pulse: crate::anim::Clock,
}

impl HomeScreen {
    pub fn new() -> Self {
        Self {
            play: Button::new(ButtonVariant::Primary, ButtonSize::Large, String::new()).radius(Metrics::RADIUS_LG),
            cancel: Button::new(ButtonVariant::Outline, ButtonSize::Large, String::new()).icon("x").radius(Metrics::RADIUS_LG),
            buy: Hover::new(),
            cabinet: Hover::new(),
            pulse: crate::anim::Clock::new(),
        }
    }

    fn cards(&self) -> [Rect; 2] {
        [
            Rect::new(PAD_X, CARD_Y, CARD_W, CARD_H),
            Rect::new(PAD_X + CARD_W + CARD_GAP, CARD_Y, CARD_W, CARD_H),
        ]
    }

    fn play_area(&self) -> Rect {
        Rect::new(Metrics::WINDOW_W - PAD_X - 16.0 - 200.0, 500.0, 200.0, 52.0)
    }

    fn buy_link(&self, canvas: &mut Canvas, state: &State) -> Rect {
        link_rect(canvas, self.cards()[0], buy_label(state), state.language)
    }

    fn cabinet_link(&self, canvas: &mut Canvas, lang: crate::i18n::Language) -> Rect {
        link_rect(canvas, self.cards()[1], "card.cabinet", lang)
    }

    fn play_glyph(state: &State) -> (&'static str, &'static str) {
        if state.pending_bytes() > 0 {
            ("download", "play.install")
        } else {
            ("gamepad", "play.play")
        }
    }

    pub fn draw(&mut self, canvas: &mut Canvas, state: &State) {
        let Some(account) = &state.account else { return };
        let profile = account.profile.clone();
        let lang = state.language;
        self.badge(canvas, state);
        let greeting = format(lang, "home.greeting", &[("name", &profile.username)]);
        let h1 = TextStyle::new(44.0, 700.0).tracking(-0.035).line_height(1.1);
        canvas.text(&greeting, h1, PAD_X, 122.0, Palette::TEXT);
        let lead = TextStyle::new(17.0, 400.0);
        canvas.text(&home_lead(state), lead, PAD_X, 180.4, Palette::TEXT_2);
        let cards = self.cards();
        self.premium_card(canvas, state, cards[0]);
        self.account_card(canvas, state, cards[1], &profile);
        self.play_bar(canvas, state);
    }

    fn badge(&self, canvas: &mut Canvas, state: &State) {
        let label = format!("Minecraft {}", state.game_version().unwrap_or_else(|| "1.21.11".into()));
        let style = TextStyle::new(13.0, 600.0).tracking(0.0);
        let layout = canvas.layout(&label, style);
        let width = 10.0 + 13.0 + 5.0 + layout.width + 10.0;
        let rect = Rect::new(PAD_X, 83.0, width, 24.0);
        canvas.fill(rect, Metrics::PILL, Color::rgba(0xf5f5f7, 0.08));
        canvas.icon("gamepad", Rect::new(rect.x + 10.0, rect.center().y - 6.5, 13.0, 13.0), Palette::TEXT_2);
        let baseline = layout.baseline_in(rect.y + (rect.h - layout.line_height()) * 0.5, layout.line_height());
        canvas.draw_layout(&layout, rect.x + 10.0 + 13.0 + 5.0, baseline, Palette::TEXT_2);
    }

    fn card_frame(&self, canvas: &mut Canvas, area: Rect) {
        canvas.fill(area, CARD_RADIUS, Palette::SURFACE);
        canvas.inset_border(area, CARD_RADIUS, 1.0, Palette::BORDER);
    }

    fn icon_tile(&self, canvas: &mut Canvas, origin: Point, icon: &'static str, fill: Color, tint: Color) {
        let rect = Rect::new(origin.x, origin.y, 40.0, 40.0);
        canvas.fill(rect, 12.0, fill);
        canvas.icon(icon, Rect::centered(rect.center(), 20.0, 20.0), tint);
    }

    fn premium_card(&self, canvas: &mut Canvas, state: &State, area: Rect) {
        self.card_frame(canvas, area);
        let premium = state.account.as_ref().and_then(|a| a.profile.premium.as_ref());
        let active = premium.map(|p| p.active).unwrap_or(false);
        self.icon_tile(canvas, Point::new(area.x + 20.0, area.y + 20.0), "crown", Palette::AMBER_SOFT, Palette::AMBER);
        if active {
            dot_badge(canvas, area, &text(state.language, "card.premiumForever"), Palette::GREEN_SOFT, Palette::GREEN);
        } else {
            dot_badge(canvas, area, &text(state.language, "card.premiumNotActive"), Palette::RED_SOFT, Palette::RED);
        }
        let h3 = TextStyle::new(16.0, 600.0).tracking(-0.02);
        canvas.text(&text(state.language, "card.premium"), h3, area.x + 20.0, area.y + 74.0, Palette::TEXT);
        let body = premium_text(state, active);
        let p = TextStyle::new(14.0, 400.0).line_height(1.45);
        canvas.paragraph_left(&body, p, area.x + 20.0, area.y + 96.0, area.w - 40.0, Palette::TEXT_2);
        self.link(canvas, area, buy_label(state), state.language, self.buy.hover());
    }

    fn account_card(&self, canvas: &mut Canvas, state: &State, area: Rect, profile: &Profile) {
        self.card_frame(canvas, area);
        self.icon_tile(canvas, Point::new(area.x + 20.0, area.y + 20.0), "user", Palette::BLUE_SOFT, Palette::BLUE);
        role_badge(canvas, area, state, profile);
        let h3 = TextStyle::new(16.0, 600.0).tracking(-0.02);
        canvas.text(&text(state.language, "card.account"), h3, area.x + 20.0, area.y + 74.0, Palette::TEXT);
        if let Some(date) = registered_text(state, profile) {
            let p = TextStyle::new(14.0, 400.0);
            canvas.text(&date, p, area.x + 20.0, area.y + 96.0, Palette::TEXT_2);
        }
        self.link(canvas, area, "card.cabinet", state.language, self.cabinet.hover());
    }

    fn link(&self, canvas: &mut Canvas, area: Rect, key: &str, lang: crate::i18n::Language, hover: f32) {
        let label = text(lang, key);
        let style = TextStyle::new(14.0, 500.0);
        let layout = canvas.layout(&label, style);
        let y = area.bottom() - 20.0 - 21.0;
        let baseline = layout.baseline_in(y, 21.0);
        canvas.draw_layout(&layout, area.x + 20.0, baseline, Palette::FOCUS);
        canvas.icon(
            "arrow-right",
            Rect::new(area.x + 20.0 + layout.width + 6.0, baseline - 10.5, 14.0, 14.0),
            Palette::FOCUS,
        );
        if hover > 0.5 {
            canvas.fill(Rect::new(area.x + 20.0, y + 19.0, layout.width, 1.0), 0.0, Palette::FOCUS);
        }
    }

    fn play_bar(&mut self, canvas: &mut Canvas, state: &State) {
        let bar = Rect::new(PAD_X, 484.0, Metrics::WINDOW_W - PAD_X * 2.0, 84.0);
        canvas.fill(bar, CARD_RADIUS, Palette::SURFACE);
        canvas.inset_border(bar, CARD_RADIUS, 1.0, Palette::BORDER);
        if state.play.busy {
            self.busy_bar(canvas, state, bar);
            return;
        }
        let blocked = !state.has_required() || (state.maintenance && !state.is_staff());
        let (dot, line) = if blocked {
            (Palette::AMBER, text(state.language, "play.unavailable"))
        } else if state.pending_bytes() > 0 {
            (
                Palette::TEXT_3,
                format(state.language, "play.idleSize", &[("size", &format_bytes(state.language, state.pending_bytes()))]),
            )
        } else {
            (Palette::GREEN, text(state.language, "play.idleReady"))
        };
        let reason = if blocked {
            Some(if state.maintenance && !state.is_staff() {
                text(state.language, "home.maintenance")
            } else {
                text(state.language, "card.clientUnpublished")
            })
        } else {
            None
        };
        let strong = TextStyle::new(15.0, 600.0);
        let layout = canvas.layout(&line, strong);
        let title_lh = layout.line_height();
        let text_x = bar.x + 38.0;
        const SUB_GAP: f32 = 2.0;
        let sub_layout = reason.as_ref().map(|reason| canvas.layout(reason, TextStyle::new(13.0, 400.0)));
        let title_top = match &sub_layout {
            Some(sub) => bar.center().y - (title_lh + SUB_GAP + sub.line_height()) * 0.5,
            None => bar.center().y - title_lh * 0.5,
        };
        let dot_center = Point::new(bar.x + 26.0, title_top + title_lh * 0.5);
        if dot == Palette::GREEN || dot == Palette::AMBER {
            let soft = if dot == Palette::GREEN { Palette::GREEN_SOFT } else { Palette::AMBER_SOFT };
            canvas.fill(Rect::centered(dot_center, 16.0, 16.0), Metrics::PILL, soft);
        }
        canvas.fill(Rect::centered(dot_center, 8.0, 8.0), Metrics::PILL, dot);
        let baseline = layout.baseline_in(title_top, title_lh);
        canvas.draw_layout(&layout, text_x, baseline, Palette::TEXT);
        if let Some(sub_layout) = &sub_layout {
            let sub_baseline = sub_layout.baseline_in(title_top + title_lh + SUB_GAP, sub_layout.line_height());
            canvas.draw_layout(sub_layout, text_x, sub_baseline, Palette::TEXT_3);
        }
        let (glyph, label_key) = Self::play_glyph(state);
        self.play.icon = Some(glyph);
        self.play.label = text(state.language, label_key);
        self.play.disabled = blocked || state.manifest.is_none();
        self.play.draw(canvas, self.play_area());
    }

    fn busy_bar(&mut self, canvas: &mut Canvas, state: &State, bar: Rect) {
        let status_right = self.play_area().x - 24.0;
        let block_h = 15.0 * 1.5 + 10.0 + 6.0;
        let top = bar.center().y - block_h * 0.5;
        let label = TextStyle::new(15.0, 600.0);
        let layout = canvas.layout(&state.play.label, label);
        let baseline = layout.baseline_in(top, layout.line_height());
        canvas.draw_layout(&layout, bar.x + 22.0, baseline, Palette::TEXT);
        if !state.play.detail.is_empty() {
            let detail = TextStyle::new(13.0, 400.0).tabular();
            let detail_layout = canvas.layout(&state.play.detail, detail);
            let dbaseline = detail_layout.baseline_in(top, layout.line_height());
            canvas.draw_layout(&detail_layout, status_right - detail_layout.width, dbaseline, Palette::TEXT_3);
        }
        let track = Rect::new(bar.x + 22.0, top + layout.line_height() + 10.0, status_right - bar.x - 22.0, 6.0);
        canvas.fill(track, Metrics::PILL, Palette::SURFACE_3);
        match state.play.progress {
            Some(fraction) => {
                let filled = track.w * fraction.clamp(0.0, 1.0);
                canvas.fill(Rect::new(track.x, track.y, filled, track.h), Metrics::PILL, Palette::TEXT.alpha(0.6));
                if filled > 2.0 {
                    let w = (filled * 0.4).clamp(12.0, track.w * 0.3);
                    sweep(canvas, track, filled, w, 1.0, self.pulse.seconds(), Palette::TEXT);
                }
            }
            None => {
                let w = track.w * 0.32;
                sweep(canvas, track, track.w, w, 1.2, self.pulse.seconds(), Palette::TEXT);
            }
        }
        self.cancel.label = text(state.language, "play.cancel");
        self.cancel.disabled = !state.play.cancellable;
        self.cancel.draw(canvas, self.play_area());
    }

    pub fn track(&mut self, canvas: &mut Canvas, state: &State, pointer: &Pointer) {
        if state.play.busy {
            self.cancel.track(self.play_area(), pointer);
            return;
        }
        self.play.track(self.play_area(), pointer);
        let buy = self.buy_link(canvas, state);
        self.buy.track(buy, pointer);
        let cabinet = self.cabinet_link(canvas, state.language);
        self.cabinet.track(cabinet, pointer);
    }

    pub fn press(&mut self, state: &State, pointer: &Pointer) {
        if state.play.busy {
            self.cancel.press(self.play_area(), pointer);
            return;
        }
        self.play.press(self.play_area(), pointer);
    }

    pub fn release(&mut self, canvas: &mut Canvas, state: &State, pointer: &Pointer) -> HomeAction {
        if state.play.busy {
            if self.cancel.release(self.play_area(), pointer) {
                return HomeAction::Cancel;
            }
            return HomeAction::None;
        }
        if self.play.release(self.play_area(), pointer) {
            return HomeAction::Play;
        }
        if self.buy_link(canvas, state).contains(pointer.position) && !pointer.pressed {
            return HomeAction::Buy;
        }
        if self.cabinet_link(canvas, state.language).contains(pointer.position) && !pointer.pressed {
            return HomeAction::Cabinet;
        }
        HomeAction::None
    }

    pub fn animating(&self) -> bool {
        self.play.animating() || self.cancel.animating() || self.buy.animating() || self.cabinet.animating()
    }
}

fn sweep(canvas: &mut Canvas, track: Rect, span: f32, w: f32, period: f32, seconds: f32, color: Color) {
    let t = (seconds / period).fract();
    let head = track.x - w + (span + w) * t;
    let start = head.max(track.x);
    let end = (head + w).min(track.x + span);
    if end > start {
        canvas.fill(Rect::new(start, track.y, end - start, track.h), Metrics::PILL, color);
    }
}

fn link_rect(canvas: &mut Canvas, card: Rect, key: &str, lang: crate::i18n::Language) -> Rect {
    let style = TextStyle::new(14.0, 500.0);
    let label = text(lang, key);
    let width = canvas.layout(&label, style).width + 6.0 + 14.0;
    Rect::new(card.x + 20.0, card.bottom() - 20.0 - 21.0, width, 21.0)
}

fn buy_label(state: &State) -> &'static str {
    let active = state
        .account
        .as_ref()
        .and_then(|a| a.profile.premium.as_ref())
        .map(|p| p.active)
        .unwrap_or(false);
    if active { "card.premiumExtend" } else { "card.premiumBuy" }
}

fn premium_text(state: &State, active: bool) -> String {
    let lang = state.language;
    if !active {
        return text(lang, "card.premiumInactive");
    }
    let expires = state
        .account
        .as_ref()
        .and_then(|a| a.profile.premium.as_ref())
        .and_then(|p| p.expires_at);
    match expires {
        Some(ts) => format(lang, "card.premiumActive", &[("date", &crate::ui::home::format_epoch(lang, ts))]),
        None => text(lang, "card.premiumForever"),
    }
}

fn registered_text(state: &State, profile: &Profile) -> Option<String> {
    let date = profile.registered_at.as_ref()?;
    let formatted = format_iso_date(state.language, date);
    Some(format(state.language, "card.accountSince", &[("date", &formatted)]))
}

pub fn format_epoch(lang: crate::i18n::Language, seconds: i64) -> String {
    let (y, m, d) = civil_from_days(seconds.div_euclid(86400));
    crate::i18n::format_month_date(lang, y, m, d)
}

fn format_iso_date(lang: crate::i18n::Language, iso: &str) -> String {
    let date = iso.split(['T', ' ']).next().unwrap_or(iso);
    let mut parts = date.split('-');
    let y: i32 = parts.next().and_then(|v| v.parse().ok()).unwrap_or(2024);
    let m: u32 = parts.next().and_then(|v| v.parse().ok()).unwrap_or(1);
    let d: u32 = parts.next().and_then(|v| v.parse().ok()).unwrap_or(1);
    crate::i18n::format_month_date(lang, y, m, d)
}

fn civil_from_days(z: i64) -> (i32, u32, u32) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
    ((y + if m <= 2 { 1 } else { 0 }) as i32, m, d)
}

fn home_lead(state: &State) -> String {
    let lang = state.language;
    if state.manifest.is_none() || state.pending_bytes() == 0 {
        text(lang, "home.ready")
    } else {
        text(lang, "home.needDownload")
    }
}

fn dot_badge(canvas: &mut Canvas, card: Rect, label: &str, fill: Color, tint: Color) {
    let style = TextStyle::new(13.0, 600.0).tracking(0.0);
    let layout = canvas.layout(label, style);
    let width = 10.0 + 6.0 + 5.0 + layout.width + 10.0;
    let rect = Rect::new(card.right() - 20.0 - width, card.y + 28.0, width, 24.0);
    canvas.fill(rect, Metrics::PILL, fill);
    canvas.fill(Rect::centered(Point::new(rect.x + 10.0 + 3.0, rect.center().y), 6.0, 6.0), Metrics::PILL, tint);
    let baseline = layout.baseline_in(rect.y + (rect.h - layout.line_height()) * 0.5, layout.line_height());
    canvas.draw_layout(&layout, rect.x + 10.0 + 6.0 + 5.0, baseline, tint);
}

fn role_badge(canvas: &mut Canvas, card: Rect, state: &State, profile: &Profile) {
    let (glyph, fill, tint) = role_style(&profile.role);
    let label = text(state.language, &format!("role.{}", profile.role));
    let style = TextStyle::new(13.0, 600.0).tracking(0.0);
    let layout = canvas.layout(&label, style);
    let width = 10.0 + 13.0 + 5.0 + layout.width + 10.0;
    let rect = Rect::new(card.right() - 20.0 - width, card.y + 28.0, width, 24.0);
    canvas.fill(rect, Metrics::PILL, fill);
    canvas.icon(glyph, Rect::new(rect.x + 10.0, rect.center().y - 6.5, 13.0, 13.0), tint);
    let baseline = layout.baseline_in(rect.y + (rect.h - layout.line_height()) * 0.5, layout.line_height());
    canvas.draw_layout(&layout, rect.x + 10.0 + 13.0 + 5.0, baseline, tint);
}

pub fn role_style(role: &str) -> (&'static str, Color, Color) {
    match role {
        "ADMIN" => ("shield", Palette::RED_SOFT, Palette::RED),
        "MODERATOR" => ("shield-check", Palette::BLUE_SOFT, Palette::BLUE),
        "MEDIA" => ("video", Palette::PINK_SOFT, Palette::PINK),
        _ => ("user", Color::rgba(0xf5f5f7, 0.08), Palette::TEXT_2),
    }
}


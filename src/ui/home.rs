use crate::core::state::{ClientBuild, State};
use crate::geometry::{Point, Rect};
use crate::gfx::canvas::Canvas;
use crate::i18n::{format, format_bytes, text};
use crate::net::api::Profile;
use crate::text::font::TextStyle;
use crate::theme::{Color, Metrics, Palette};
use crate::ui::interaction::{Hover, Pointer};
use crate::ui::widgets::{Button, ButtonSize, ButtonVariant};

const PAD_X: f32 = 40.0;
const TOP: f32 = 76.0;
const BOTTOM: f32 = 568.0;
const GAP: f32 = 14.0;
const SIDE_W: f32 = 294.0;
const INSET: f32 = 32.0;
const CARD_INSET: f32 = 22.0;
const CARD_RADIUS: f32 = Metrics::RADIUS_LG;
const SPIN: f32 = 0.9;
const SWITCH_H: f32 = 30.0;
const SWITCH_PAD: f32 = 14.0;
const SWITCH_ICON: f32 = 14.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HomeAction {
    None,
    Play,
    Cancel,
    Buy,
    Cabinet,
    SelectBuild(ClientBuild),
}

pub struct HomeScreen {
    play: Button,
    cancel: Button,
    buy: Button,
    cabinet: Button,
    switch: [Hover; 2],
    pulse: crate::anim::Clock,
}

impl HomeScreen {
    pub fn new() -> Self {
        Self {
            play: Button::new(ButtonVariant::Primary, ButtonSize::Large, String::new()).radius(Metrics::RADIUS_LG),
            cancel: Button::new(ButtonVariant::Outline, ButtonSize::Large, String::new()).icon(crate::secret!("x")).radius(Metrics::RADIUS_LG),
            buy: Button::new(ButtonVariant::Neutral, ButtonSize::Small, String::new()).radius(Metrics::RADIUS_SM),
            cabinet: Button::new(ButtonVariant::Neutral, ButtonSize::Small, String::new()).icon(crate::secret!("external")).radius(Metrics::RADIUS_SM),
            switch: [Hover::new(), Hover::new()],
            pulse: crate::anim::Clock::new(),
        }
    }

    fn hero(&self) -> Rect {
        Rect::new(PAD_X, TOP, Metrics::WINDOW_W - PAD_X * 2.0 - GAP - SIDE_W, BOTTOM - TOP)
    }

    fn cards(&self) -> [Rect; 2] {
        let x = Metrics::WINDOW_W - PAD_X - SIDE_W;
        let h = (BOTTOM - TOP - GAP) * 0.5;
        [Rect::new(x, TOP, SIDE_W, h), Rect::new(x, TOP + h + GAP, SIDE_W, h)]
    }

    fn play_area(&self) -> Rect {
        let hero = self.hero();
        Rect::new(hero.right() - INSET - 200.0, hero.bottom() - INSET - 52.0, 200.0, 52.0)
    }

    fn card_button(card: Rect) -> Rect {
        Rect::new(card.x + CARD_INSET, card.bottom() - CARD_INSET - 34.0, card.w - CARD_INSET * 2.0, 34.0)
    }

    fn play_glyph(state: &State) -> (String, String) {
        if state.pending_bytes() > 0 {
            (crate::secret!("download").to_owned(), crate::secret!("play.install").to_owned())
        } else {
            (crate::secret!("gamepad").to_owned(), crate::secret!("play.play").to_owned())
        }
    }

    pub fn draw(&mut self, canvas: &mut Canvas, state: &State) {
        let Some(account) = &state.account else { return };
        let profile = account.profile.clone();
        let lang = state.language;
        let hero = self.hero();
        self.hero_frame(canvas, hero);
        self.badge(canvas, state, Point::new(hero.x + INSET, hero.y + INSET));
        self.build_switch(canvas, state);
        let greeting = format(lang, crate::secret!("home.greeting"), &[(crate::secret!("name"), &profile.username)]);
        let h1 = TextStyle::new(44.0, 700.0).tracking(-0.035).line_height(1.1);
        let lead = TextStyle::new(16.0, 400.0).line_height(1.5);
        let lead_text = home_lead(state);
        let lead_w = hero.w - INSET * 2.0 - 60.0;
        let lead_h = canvas.wrap(&lead_text, lead, lead_w).iter().map(|l| l.line_height()).sum::<f32>();
        let lead_top = self.divider() - 36.0 - lead_h;
        let title_top = lead_top - 10.0 - 44.0 * 1.1;
        canvas.text(&greeting, h1, hero.x + INSET, title_top, Palette::TEXT);
        canvas.paragraph_left(&lead_text, lead, hero.x + INSET, lead_top, lead_w, Palette::TEXT_2);
        let cards = self.cards();
        self.premium_card(canvas, state, cards[0]);
        self.account_card(canvas, state, cards[1], &profile);
        self.buy.label = text(lang, &buy_label(state));
        self.buy.icon = Some(buy_icon(state));
        self.buy.draw(canvas, Self::card_button(cards[0]));
        self.cabinet.label = text(lang, crate::secret!("card.cabinet"));
        self.cabinet.draw(canvas, Self::card_button(cards[1]));
        self.play_bar(canvas, state);
    }

    fn hero_frame(&self, canvas: &mut Canvas, hero: Rect) {
        canvas.fill(hero, CARD_RADIUS, Palette::SURFACE);
        canvas.glow(hero, CARD_RADIUS, Point::new(hero.right() - 40.0, hero.y - 20.0), 520.0, 360.0, Palette::BLUE.alpha(0.22));
        canvas.glow(hero, CARD_RADIUS, Point::new(hero.right() + 40.0, hero.bottom() - 120.0), 360.0, 280.0, Palette::VIOLET.alpha(0.10));
        canvas.inset_border(hero, CARD_RADIUS, 1.0, Palette::BORDER);
        canvas.icon(crate::secret!("mark"), Rect::new(hero.right() - INSET - 132.0, hero.y + INSET + 64.0, 132.0, 132.0), Palette::TEXT.alpha(0.06));
        canvas.fill(Rect::new(hero.x + INSET, self.divider(), hero.w - INSET * 2.0, 1.0), 0.0, Palette::BORDER);
    }

    fn switch_rects(&self, canvas: &mut Canvas, state: &State) -> Option<[Rect; 2]> {
        if !state.premium_active() {
            return None;
        }
        let hero = self.hero();
        let style = switch_style();
        let free = canvas.layout(&text(state.language, crate::secret!("build.free")), style).width + SWITCH_PAD * 2.0;
        let premium = canvas.layout(&text(state.language, crate::secret!("build.premium")), style).width + SWITCH_PAD * 2.0 + SWITCH_ICON + 6.0;
        let top = hero.y + INSET + 12.0 - SWITCH_H * 0.5;
        let right = hero.right() - INSET - 3.0;
        let premium_rect = Rect::new(right - premium, top, premium, SWITCH_H);
        Some([Rect::new(premium_rect.x - free, top, free, SWITCH_H), premium_rect])
    }

    fn build_switch(&self, canvas: &mut Canvas, state: &State) {
        let Some(rects) = self.switch_rects(canvas, state) else { return };
        let outer = Rect::new(rects[0].x - 3.0, rects[0].y - 3.0, rects[0].w + rects[1].w + 6.0, SWITCH_H + 6.0);
        canvas.fill(outer, Metrics::PILL, Color::rgba(0xffffff, 0.05));
        canvas.inset_border(outer, Metrics::PILL, 1.0, Palette::BORDER);
        let selected = state.selected_build();
        let style = switch_style();
        for (index, build) in [ClientBuild::Free, ClientBuild::Premium].into_iter().enumerate() {
            let rect = rects[index];
            let active = build == selected;
            let hover = self.switch[index].hover();
            if active {
                canvas.fill(rect, Metrics::PILL, Palette::SURFACE_3);
                canvas.inset_border(rect, Metrics::PILL, 1.0, Palette::BORDER);
            } else if hover > 0.01 {
                canvas.fill(rect, Metrics::PILL, Color::rgba(0xffffff, 0.04 * hover));
            }
            let color = if active { Palette::TEXT } else { Palette::TEXT_2.mix(Palette::TEXT, hover) };
            let mut x = rect.x + SWITCH_PAD;
            if build == ClientBuild::Premium {
                let tint = if active { Palette::AMBER } else { Palette::AMBER.alpha(0.7) };
                canvas.icon(crate::secret!("crown"), Rect::new(x, rect.center().y - SWITCH_ICON * 0.5, SWITCH_ICON, SWITCH_ICON), tint);
                x += SWITCH_ICON + 6.0;
            }
            let key = if build == ClientBuild::Premium { crate::secret!("build.premium").to_owned() } else { crate::secret!("build.free").to_owned() };
            let layout = canvas.layout(&text(state.language, &key), style);
            let baseline = layout.baseline_in(rect.y + (rect.h - layout.line_height()) * 0.5, layout.line_height());
            canvas.draw_layout(&layout, x, baseline, color);
        }
    }

    fn divider(&self) -> f32 {
        self.play_area().y - 28.0
    }

    fn badge(&self, canvas: &mut Canvas, state: &State, origin: Point) {
        let label = format!("{} {}", crate::secret!("Minecraft"), state.game_version().unwrap_or_else(|| crate::secret!("1.21.11").into()));
        let style = TextStyle::new(13.0, 600.0).tracking(0.0);
        let layout = canvas.layout(&label, style);
        let width = 10.0 + 13.0 + 5.0 + layout.width + 10.0;
        let rect = Rect::new(origin.x, origin.y, width, 24.0);
        canvas.fill(rect, Metrics::PILL, Color::rgba(0xf5f5f7, 0.08));
        canvas.icon(crate::secret!("gamepad"), Rect::new(rect.x + 10.0, rect.center().y - 6.5, 13.0, 13.0), Palette::TEXT_2);
        let baseline = layout.baseline_in(rect.y + (rect.h - layout.line_height()) * 0.5, layout.line_height());
        canvas.draw_layout(&layout, rect.x + 10.0 + 13.0 + 5.0, baseline, Palette::TEXT_2);
    }

    fn card_frame(&self, canvas: &mut Canvas, area: Rect, glow: Color) {
        canvas.fill(area, CARD_RADIUS, Palette::SURFACE);
        canvas.glow(area, CARD_RADIUS, Point::new(area.right() - 10.0, area.y - 30.0), 300.0, 220.0, glow);
        canvas.inset_border(area, CARD_RADIUS, 1.0, Palette::BORDER);
    }

    fn icon_tile(&self, canvas: &mut Canvas, origin: Point, icon: &str, fill: Color, tint: Color) {
        let rect = Rect::new(origin.x, origin.y, 40.0, 40.0);
        canvas.fill(rect, 12.0, fill);
        canvas.icon(icon, Rect::centered(rect.center(), 20.0, 20.0), tint);
    }

    fn headline_top(area: Rect) -> f32 {
        area.y + CARD_INSET + 40.0 + 26.0
    }

    fn premium_card(&self, canvas: &mut Canvas, state: &State, area: Rect) {
        let lang = state.language;
        self.card_frame(canvas, area, Palette::AMBER.alpha(0.16));
        let premium = state.account.as_ref().and_then(|a| a.profile.premium.as_ref());
        let active = premium.map(|p| p.active).unwrap_or(false);
        let expires = premium.and_then(|p| p.expires_at).filter(|_| active);
        self.icon_tile(canvas, Point::new(area.x + CARD_INSET, area.y + CARD_INSET), crate::secret!("crown"), Palette::AMBER_SOFT, Palette::AMBER);
        card_label(canvas, area, &text(lang, crate::secret!("card.premium")));
        if active {
            dot_badge(canvas, area, &text(lang, crate::secret!("card.premiumForever")), Palette::GREEN_SOFT, Palette::GREEN);
        }
        let x = area.x + CARD_INSET;
        let top = Self::headline_top(area);
        let sub_style = TextStyle::new(13.5, 400.0);
        match expires {
            Some(ts) => {
                let days = days_left(ts);
                let number = TextStyle::new(38.0, 700.0).tracking(-0.04).line_height(1.0);
                let number_layout = canvas.layout(&days.to_string(), number);
                let baseline = number_layout.baseline_in(top, 38.0);
                canvas.draw_layout(&number_layout, x, baseline, Palette::TEXT);
                let unit = TextStyle::new(17.0, 600.0).tracking(-0.01);
                let unit_layout = canvas.layout(&days_word(lang, days), unit);
                canvas.draw_layout(&unit_layout, x + number_layout.width + 7.0, baseline, Palette::TEXT_2);
                let until = format(lang, crate::secret!("card.premiumUntil"), &[(crate::secret!("date"), &format_epoch(lang, ts))]);
                canvas.text(&until, sub_style, x, top + 38.0 + 8.0, Palette::TEXT_3);
            }
            None => {
                let (title, sub) = if active {
                    (crate::secret!("card.premiumLifetime").to_owned(), crate::secret!("card.premiumLifetimeHint").to_owned())
                } else {
                    (crate::secret!("card.premiumOff").to_owned(), crate::secret!("card.premiumPitch").to_owned())
                };
                headline(canvas, &text(lang, &title), x, top, area.w - CARD_INSET * 2.0);
                canvas.paragraph_left(&text(lang, &sub), sub_style.line_height(1.4), x, top + 38.0 + 8.0, area.w - CARD_INSET * 2.0, Palette::TEXT_3);
            }
        }
    }

    fn account_card(&self, canvas: &mut Canvas, state: &State, area: Rect, profile: &Profile) {
        self.card_frame(canvas, area, Palette::BLUE.alpha(0.14));
        let avatar = Rect::new(area.x + CARD_INSET, area.y + CARD_INSET, 40.0, 40.0);
        canvas.fill(avatar, Metrics::PILL, Palette::SURFACE_3);
        canvas.inset_border(avatar, Metrics::PILL, 1.0, Palette::BORDER_STRONG);
        let initial = profile.username.chars().next().unwrap_or('?').to_uppercase().to_string();
        let style = TextStyle::new(16.0, 600.0);
        canvas.text_centered(&initial, style, avatar.center().x, avatar.center().y - style.size * 0.75, Palette::TEXT);
        card_label(canvas, area, &text(state.language, crate::secret!("card.account")));
        role_badge(canvas, area, state, profile);
        let x = area.x + CARD_INSET;
        let top = Self::headline_top(area);
        headline(canvas, &profile.username, x, top, area.w - CARD_INSET * 2.0);
        if let Some(date) = registered_text(state, profile) {
            canvas.text(&date, TextStyle::new(13.5, 400.0), x, top + 38.0 + 8.0, Palette::TEXT_3);
        }
    }

    fn play_bar(&mut self, canvas: &mut Canvas, state: &State) {
        let hero = self.hero();
        let area = self.play_area();
        let bar = Rect::new(hero.x + INSET, area.y, area.x - hero.x - INSET, area.h);
        if state.play.busy {
            self.busy_bar(canvas, state, bar);
            return;
        }
        let blocked = !state.has_required() || (state.maintenance && !state.bypasses_maintenance());
        let checking = !blocked && state.checking();
        let (dot, line) = if blocked {
            (Palette::AMBER, text(state.language, crate::secret!("play.unavailable")))
        } else if checking {
            (Palette::TEXT_3, text(state.language, crate::secret!("play.checking")))
        } else if state.pending_bytes() > 0 {
            (
                Palette::TEXT_3,
                format(state.language, crate::secret!("play.idleSize"), &[(crate::secret!("size"), &format_bytes(state.language, state.pending_bytes()))]),
            )
        } else {
            (Palette::GREEN, text(state.language, crate::secret!("play.idleReady")))
        };
        let reason = if blocked {
            Some(if state.maintenance && !state.bypasses_maintenance() {
                text(state.language, crate::secret!("home.maintenance"))
            } else {
                text(state.language, crate::secret!("card.clientUnpublished"))
            })
        } else {
            None
        };
        let strong = TextStyle::new(15.0, 600.0);
        let layout = canvas.layout(&line, strong);
        let title_lh = layout.line_height();
        let text_x = bar.x + 22.0;
        const SUB_GAP: f32 = 2.0;
        let sub_layout = reason.as_ref().map(|reason| canvas.layout(reason, TextStyle::new(13.0, 400.0)));
        let title_top = match &sub_layout {
            Some(sub) => bar.center().y - (title_lh + SUB_GAP + sub.line_height()) * 0.5,
            None => bar.center().y - title_lh * 0.5,
        };
        let dot_center = Point::new(bar.x + 8.0, bar.center().y);
        if checking {
            let angle = (self.pulse.seconds() % SPIN) / SPIN * std::f32::consts::TAU;
            canvas.spinner(dot_center, 14.0, 2.0, angle, Palette::TEXT_2, Palette::SURFACE_3);
        } else {
            if dot == Palette::GREEN || dot == Palette::AMBER {
                let soft = if dot == Palette::GREEN { Palette::GREEN_SOFT } else { Palette::AMBER_SOFT };
                canvas.fill(Rect::centered(dot_center, 16.0, 16.0), Metrics::PILL, soft);
            }
            canvas.fill(Rect::centered(dot_center, 8.0, 8.0), Metrics::PILL, dot);
        }
        let baseline = layout.baseline_in(title_top, title_lh);
        canvas.draw_layout(&layout, text_x, baseline, Palette::TEXT);
        if let Some(sub_layout) = &sub_layout {
            let sub_baseline = sub_layout.baseline_in(title_top + title_lh + SUB_GAP, sub_layout.line_height());
            canvas.draw_layout(sub_layout, text_x, sub_baseline, Palette::TEXT_3);
        }
        let (glyph, label_key) = Self::play_glyph(state);
        self.play.icon = Some(glyph);
        self.play.label = text(state.language, &label_key);
        self.play.disabled = blocked || state.manifest.is_none();
        self.play.draw(canvas, self.play_area());
    }

    fn busy_bar(&mut self, canvas: &mut Canvas, state: &State, bar: Rect) {
        let status_right = self.play_area().x - 24.0;
        let bar = Rect::new(bar.x - 22.0, bar.y, bar.w, bar.h);
        let block_h = 15.0 * 1.5 + 10.0 + 6.0;
        let top = bar.center().y - block_h * 0.5;
        let label = TextStyle::new(15.0, 600.0);
        let layout = canvas.layout(&state.play.label, label);
        let baseline = layout.baseline_in(top, layout.line_height());
        canvas.draw_layout(&layout, bar.x + 22.0, baseline, Palette::TEXT);
        let detail_text = match state.play.since {
            Some(since) => {
                let seconds = since.elapsed().as_secs();
                format!("{}:{:02}", seconds / 60, seconds % 60)
            }
            None => state.play.detail.clone(),
        };
        if !detail_text.is_empty() {
            let detail = TextStyle::new(13.0, 400.0).tabular();
            let detail_layout = canvas.layout(&detail_text, detail);
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
        self.cancel.label = text(state.language, crate::secret!("play.cancel"));
        self.cancel.disabled = !state.play.cancellable;
        self.cancel.draw(canvas, self.play_area());
    }

    pub fn track(&mut self, canvas: &mut Canvas, state: &State, pointer: &Pointer) {
        if state.play.busy {
            self.cancel.track(self.play_area(), pointer);
            return;
        }
        if let Some(rects) = self.switch_rects(canvas, state) {
            self.switch[0].track(rects[0], pointer);
            self.switch[1].track(rects[1], pointer);
        }
        self.play.track(self.play_area(), pointer);
        let cards = self.cards();
        self.buy.track(Self::card_button(cards[0]), pointer);
        self.cabinet.track(Self::card_button(cards[1]), pointer);
    }

    pub fn press(&mut self, state: &State, pointer: &Pointer) {
        if state.play.busy {
            self.cancel.press(self.play_area(), pointer);
            return;
        }
        self.play.press(self.play_area(), pointer);
        let cards = self.cards();
        self.buy.press(Self::card_button(cards[0]), pointer);
        self.cabinet.press(Self::card_button(cards[1]), pointer);
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
        if let Some(rects) = self.switch_rects(canvas, state) {
            if rects[0].contains(pointer.position) && !pointer.pressed {
                return HomeAction::SelectBuild(ClientBuild::Free);
            }
            if rects[1].contains(pointer.position) && !pointer.pressed {
                return HomeAction::SelectBuild(ClientBuild::Premium);
            }
        }
        let cards = self.cards();
        if self.buy.release(Self::card_button(cards[0]), pointer) {
            return HomeAction::Buy;
        }
        if self.cabinet.release(Self::card_button(cards[1]), pointer) {
            return HomeAction::Cabinet;
        }
        HomeAction::None
    }

    pub fn animating(&self) -> bool {
        self.play.animating() || self.cancel.animating() || self.buy.animating() || self.cabinet.animating() || self.switch.iter().any(Hover::animating)
    }
}

fn switch_style() -> TextStyle {
    TextStyle::new(13.5, 600.0).tracking(-0.005)
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

fn card_label(canvas: &mut Canvas, card: Rect, label: &str) {
    let style = TextStyle::new(15.0, 600.0).tracking(-0.01);
    let layout = canvas.layout(label, style);
    let tile_center = card.y + CARD_INSET + 20.0;
    let baseline = layout.baseline_in(tile_center - layout.line_height() * 0.5, layout.line_height());
    canvas.draw_layout(&layout, card.x + CARD_INSET + 40.0 + 12.0, baseline, Palette::TEXT);
}

fn headline(canvas: &mut Canvas, value: &str, x: f32, top: f32, max_width: f32) {
    let mut size = 28.0;
    let mut layout = canvas.layout(value, TextStyle::new(size, 700.0).tracking(-0.03));
    while layout.width > max_width && size > 16.0 {
        size -= 1.0;
        layout = canvas.layout(value, TextStyle::new(size, 700.0).tracking(-0.03));
    }
    let baseline = layout.baseline_in(top, 38.0);
    canvas.draw_layout(&layout, x, baseline, Palette::TEXT);
}

fn days_left(expires: i64) -> i64 {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    ((expires - now).max(0) + 86399) / 86400
}

fn days_word(lang: crate::i18n::Language, days: i64) -> String {
    let tail = days % 100;
    let last = days % 10;
    let key = if last == 1 && tail != 11 {
        crate::secret!("card.days.one").to_owned()
    } else if (2..=4).contains(&last) && !(12..=14).contains(&tail) {
        crate::secret!("card.days.few").to_owned()
    } else {
        crate::secret!("card.days.many").to_owned()
    };
    text(lang, &key)
}

fn buy_icon(state: &State) -> String {
    match premium_kind(state) {
        PremiumKind::Lifetime => crate::secret!("external").to_owned(),
        PremiumKind::Timed => crate::secret!("sparkles").to_owned(),
        PremiumKind::Off => crate::secret!("cart").to_owned(),
    }
}

enum PremiumKind {
    Off,
    Timed,
    Lifetime,
}

fn premium_kind(state: &State) -> PremiumKind {
    let premium = state.account.as_ref().and_then(|a| a.profile.premium.as_ref());
    match premium {
        Some(p) if p.active && p.expires_at.is_none() => PremiumKind::Lifetime,
        Some(p) if p.active => PremiumKind::Timed,
        _ => PremiumKind::Off,
    }
}

fn buy_label(state: &State) -> String {
    match premium_kind(state) {
        PremiumKind::Lifetime => crate::secret!("card.premiumMore").to_owned(),
        PremiumKind::Timed => crate::secret!("card.premiumExtend").to_owned(),
        PremiumKind::Off => crate::secret!("card.premiumBuy").to_owned(),
    }
}

fn registered_text(state: &State, profile: &Profile) -> Option<String> {
    let date = profile.registered_at.as_ref()?;
    let formatted = format_iso_date(state.language, date);
    Some(format(state.language, crate::secret!("card.accountSince"), &[(crate::secret!("date"), &formatted)]))
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
    if state.manifest.is_some() && !state.build_available(state.selected_build()) {
        text(lang, crate::secret!("home.buildSoon"))
    } else if state.checking() {
        text(lang, crate::secret!("home.checking"))
    } else if state.manifest.is_none() || state.pending_bytes() == 0 {
        text(lang, crate::secret!("home.ready"))
    } else {
        text(lang, crate::secret!("home.needDownload"))
    }
}

fn dot_badge(canvas: &mut Canvas, card: Rect, label: &str, fill: Color, tint: Color) {
    let style = TextStyle::new(13.0, 600.0).tracking(0.0);
    let layout = canvas.layout(label, style);
    let width = 10.0 + 6.0 + 5.0 + layout.width + 10.0;
    let rect = Rect::new(card.right() - CARD_INSET - width, card.y + CARD_INSET + 8.0, width, 24.0);
    canvas.fill(rect, Metrics::PILL, fill);
    canvas.fill(Rect::centered(Point::new(rect.x + 10.0 + 3.0, rect.center().y), 6.0, 6.0), Metrics::PILL, tint);
    let baseline = layout.baseline_in(rect.y + (rect.h - layout.line_height()) * 0.5, layout.line_height());
    canvas.draw_layout(&layout, rect.x + 10.0 + 6.0 + 5.0, baseline, tint);
}

fn role_badge(canvas: &mut Canvas, card: Rect, state: &State, profile: &Profile) {
    let (glyph, fill, tint) = role_style(&profile.role);
    let label = text(state.language, &format!("{}{}", crate::secret!("role."), profile.role));
    let style = TextStyle::new(13.0, 600.0).tracking(0.0);
    let layout = canvas.layout(&label, style);
    let width = 10.0 + 13.0 + 5.0 + layout.width + 10.0;
    let rect = Rect::new(card.right() - CARD_INSET - width, card.y + CARD_INSET + 8.0, width, 24.0);
    canvas.fill(rect, Metrics::PILL, fill);
    canvas.icon(&glyph, Rect::new(rect.x + 10.0, rect.center().y - 6.5, 13.0, 13.0), tint);
    let baseline = layout.baseline_in(rect.y + (rect.h - layout.line_height()) * 0.5, layout.line_height());
    canvas.draw_layout(&layout, rect.x + 10.0 + 13.0 + 5.0, baseline, tint);
}

pub fn role_style(role: &str) -> (String, Color, Color) {
    let (icon, fill, tint) = if role == crate::secret!("ADMIN") {
        (crate::secret!("shield").to_owned(), Palette::RED_SOFT, Palette::RED)
    } else if role == crate::secret!("MODERATOR") {
        (crate::secret!("shield-check").to_owned(), Palette::BLUE_SOFT, Palette::BLUE)
    } else if role == crate::secret!("SUPPORT") {
        (crate::secret!("headset").to_owned(), Palette::GREEN_SOFT, Palette::GREEN)
    } else if role == crate::secret!("MEDIA") {
        (crate::secret!("video").to_owned(), Palette::PINK_SOFT, Palette::PINK)
    } else {
        (crate::secret!("user").to_owned(), Color::rgba(0xf5f5f7, 0.08), Palette::TEXT_2)
    };
    (icon, fill, tint)
}


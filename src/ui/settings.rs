use crate::config::Config;
use crate::core::paths::Paths;
use crate::core::state::State;
use crate::geometry::{Point, Rect};
use crate::gfx::canvas::{Canvas, Layer};
use crate::i18n::{Language, format, text};
use crate::text::font::TextStyle;
use crate::theme::{Color, Metrics, Palette};
use crate::ui::interaction::{Hover, Pointer};
use crate::ui::widgets::{Button, ButtonSize, ButtonVariant};

const CARD_X: f32 = 160.0;
const CARD_W: f32 = 640.0;
const VIEW_TOP: f32 = Metrics::BAR_H;
const VIEW_BOTTOM: f32 = Metrics::WINDOW_H;

#[derive(Clone, Debug, PartialEq)]
pub enum SettingsAction {
    None,
    Memory(u32),
    Language(Language),
    OpenFolder,
    Verify,
    OpenUrl(String),
}

struct Layout {
    h1: f32,
    group1_title: f32,
    group1: Rect,
    memory_row: Rect,
    folder_row: Rect,
    group2_title: f32,
    group2: Rect,
    language_row: Rect,
    group3_title: f32,
    group3: Rect,
    verify_row: Rect,
    footer: f32,
    content_height: f32,
}

pub struct SettingsScreen {
    scroll: f32,
    open: Button,
    verify: Button,
    lang_ru: Hover,
    lang_en: Hover,
    site: Hover,
    support: Hover,
    dragging_memory: bool,
}

impl SettingsScreen {
    pub fn new() -> Self {
        Self {
            scroll: 0.0,
            open: Button::new(ButtonVariant::Neutral, ButtonSize::Small, String::new()).icon("external"),
            verify: Button::new(ButtonVariant::Neutral, ButtonSize::Small, String::new()).icon("refresh"),
            lang_ru: Hover::new(),
            lang_en: Hover::new(),
            site: Hover::new(),
            support: Hover::new(),
            dragging_memory: false,
        }
    }

    fn layout(&self) -> Layout {
        let h1 = 80.0;
        let group1_title = h1 + 35.2 + 24.0;
        let group1_top = group1_title + 19.5 + 10.0;
        let memory_row = Rect::new(CARD_X, group1_top, CARD_W, 95.5);
        let folder_row = Rect::new(CARD_X, memory_row.bottom(), CARD_W, 77.3);
        let group1 = Rect::new(CARD_X, group1_top, CARD_W, memory_row.h + folder_row.h);
        let group2_title = group1.bottom() + 24.0;
        let group2_top = group2_title + 19.5 + 10.0;
        let language_row = Rect::new(CARD_X, group2_top, CARD_W, 72.0);
        let group2 = Rect::new(CARD_X, group2_top, CARD_W, 72.0);
        let group3_title = group2.bottom() + 24.0;
        let group3_top = group3_title + 19.5 + 10.0;
        let verify_row = Rect::new(CARD_X, group3_top, CARD_W, 72.0);
        let group3 = Rect::new(CARD_X, group3_top, CARD_W, 72.0);
        let footer = group3.bottom() + 24.0;
        let content_height = footer + 19.5 + 40.0 - VIEW_TOP;
        Layout {
            h1,
            group1_title,
            group1,
            memory_row,
            folder_row,
            group2_title,
            group2,
            language_row,
            group3_title,
            group3,
            verify_row,
            footer,
            content_height,
        }
    }

    fn max_scroll(&self) -> f32 {
        let viewport = VIEW_BOTTOM - VIEW_TOP;
        (self.layout().content_height - viewport).max(0.0)
    }

    pub fn scroll_by(&mut self, delta: f32) {
        self.scroll = (self.scroll - delta).clamp(0.0, self.max_scroll());
    }

    fn total_memory(_state: &State) -> u32 {
        Config::system_memory_gb()
    }

    fn memory_track(&self, row: Rect) -> Rect {
        Rect::new(row.right() - 20.0 - 260.0, row.center().y - 3.0, 204.0, 6.0)
    }

    pub fn draw(&mut self, canvas: &mut Canvas, state: &State) {
        let layout = self.layout();
        canvas.clip(Some(Rect::new(0.0, VIEW_TOP, Metrics::WINDOW_W, VIEW_BOTTOM - VIEW_TOP)));
        let offset = -self.scroll;
        canvas.with(Layer::new().translate(0.0, offset), |c| self.draw_content(c, state, &layout));
        canvas.clip(None);
    }

    fn draw_content(&mut self, canvas: &mut Canvas, state: &State, layout: &Layout) {
        let lang = state.language;
        canvas.text(&text(lang, "settings.title"), TextStyle::new(32.0, 700.0).tracking(-0.028).line_height(1.1), CARD_X, layout.h1, Palette::TEXT);
        self.group_title(canvas, &text(lang, "settings.game"), layout.group1_title);
        self.card(canvas, layout.group1);
        self.row_divider(canvas, layout.folder_row);
        self.memory_row(canvas, state, layout.memory_row);
        self.folder_row(canvas, state, layout.folder_row);
        self.group_title(canvas, &text(lang, "settings.launcher"), layout.group2_title);
        self.card(canvas, layout.group2);
        self.language_row(canvas, state, layout.language_row);
        self.group_title(canvas, &text(lang, "settings.files"), layout.group3_title);
        self.card(canvas, layout.group3);
        self.verify_row(canvas, state, layout.verify_row);
        self.footer(canvas, state, layout.footer);
    }

    fn group_title(&self, canvas: &mut Canvas, label: &str, y: f32) {
        canvas.text(label, TextStyle::new(13.0, 600.0).tracking(0.0), CARD_X + 4.0, y, Palette::TEXT_3);
    }

    fn card(&self, canvas: &mut Canvas, area: Rect) {
        canvas.fill(area, Metrics::RADIUS_LG, Palette::SURFACE);
        canvas.inset_border(area, Metrics::RADIUS_LG, 1.0, Palette::BORDER);
    }

    fn row_divider(&self, canvas: &mut Canvas, row: Rect) {
        canvas.fill(Rect::new(row.x, row.y, row.w, 1.0), 0.0, Palette::BORDER);
    }

    fn icon_tile(&self, canvas: &mut Canvas, row: Rect, icon: &'static str) {
        let rect = Rect::new(row.x + 20.0, row.center().y - 20.0, 40.0, 40.0);
        canvas.fill(rect, 12.0, Color::rgba(0xf5f5f7, 0.07));
        canvas.icon(icon, Rect::centered(rect.center(), 18.0, 18.0), Palette::TEXT);
    }

    fn row_text(&self, canvas: &mut Canvas, row: Rect, title: &str, subtitle: Option<(&str, f32)>) {
        let x = row.x + 20.0 + 40.0 + 16.0;
        let title_style = TextStyle::new(15.0, 600.0);
        let title_lh = title_style.size * 1.5;
        if let Some((sub, sub_width)) = subtitle {
            let sub_style = TextStyle::new(13.0, 400.0).line_height(1.25);
            let lines = canvas.wrap(sub, sub_style, sub_width).len() as f32;
            let title_h = 20.0;
            let sub_lh = 16.0;
            let gap = 1.0;
            let block_h = title_h + gap + lines * sub_lh;
            let block_top = row.center().y - block_h * 0.5;
            let baseline = canvas.layout(title, title_style).baseline_in(block_top, title_h);
            let title_layout = canvas.layout(title, title_style);
            canvas.draw_layout(&title_layout, x, baseline, Palette::TEXT);
            canvas.paragraph_left(sub, sub_style, x, block_top + title_h + gap, sub_width, Palette::TEXT_3);
        } else {
            let layout = canvas.layout(title, title_style);
            let baseline = layout.baseline_in(row.center().y - title_lh * 0.5, title_lh);
            canvas.draw_layout(&layout, x, baseline, Palette::TEXT);
        }
    }

    fn memory_row(&mut self, canvas: &mut Canvas, state: &State, row: Rect) {
        self.icon_tile(canvas, row, "cpu");
        let total = Self::total_memory(state);
        let hint = format(state.language, "settings.memoryHint", &[("total", &format(state.language, "gb", &[("n", &total.to_string())]))]);
        self.row_text(canvas, row, &text(state.language, "settings.memory"), Some((&hint, 284.0)));
        let track = self.memory_track(row);
        let fraction = ((state.memory - Config::MEMORY_MIN) as f32 / (total - Config::MEMORY_MIN) as f32).clamp(0.0, 1.0);
        canvas.fill(track, Metrics::PILL, Palette::SURFACE_3);
        canvas.fill(Rect::new(track.x, track.y, track.w * fraction, track.h), Metrics::PILL, Palette::TEXT);
        let thumb = Point::new(track.x + track.w * fraction, track.center().y);
        canvas.fill(Rect::centered(thumb, 22.0, 22.0), Metrics::PILL, Color::rgb(0xffffff));
        let value = format(state.language, "gb", &[("n", &state.memory.to_string())]);
        let style = TextStyle::new(15.0, 600.0).tabular();
        let layout = canvas.layout(&value, style);
        let baseline = layout.baseline_in(row.center().y - layout.line_height() * 0.5, layout.line_height());
        canvas.draw_layout(&layout, row.right() - 20.0 - layout.width, baseline, Palette::TEXT);
    }

    fn folder_row(&mut self, canvas: &mut Canvas, state: &State, row: Rect) {
        self.icon_tile(canvas, row, "folder");
        let x = row.x + 20.0 + 40.0 + 16.0;
        let title_style = TextStyle::new(15.0, 600.0);
        let path_style = TextStyle::new(12.5, 400.0);
        let title_lh = 20.0;
        let path_lh = 16.0;
        let gap = 1.0;
        let block_top = row.center().y - (title_lh + gap + path_lh) * 0.5;
        let title_layout = canvas.layout(&text(state.language, "settings.folder"), title_style);
        let baseline = title_layout.baseline_in(block_top, title_lh);
        canvas.draw_layout(&title_layout, x, baseline, Palette::TEXT);
        canvas.text(&Paths::game_display(), path_style, x, block_top + title_lh + gap, Palette::TEXT_2);
        self.open.label = text(state.language, "settings.open");
        let width = 108.5;
        self.open.draw(canvas, Rect::new(row.right() - 20.0 - width, row.center().y - 17.0, width, 34.0));
    }

    fn language_row(&mut self, canvas: &mut Canvas, state: &State, row: Rect) {
        self.icon_tile(canvas, row, "globe");
        self.row_text(canvas, row, &text(state.language, "settings.language"), None);
        let (ru, en) = self.segmented_rects(row);
        let outer = Rect::new(ru.x - 3.0, ru.y - 3.0, ru.w + en.w + 6.0, ru.h + 6.0);
        canvas.fill(outer, Metrics::PILL, Palette::SURFACE_2);
        canvas.inset_border(outer, Metrics::PILL, 1.0, Palette::BORDER);
        self.segment(canvas, ru, "ru", "Русский", state.language == Language::Ru, self.lang_ru.hover());
        self.segment(canvas, en, "gb", "English", state.language == Language::En, self.lang_en.hover());
    }

    fn segmented_rects(&self, row: Rect) -> (Rect, Rect) {
        let ru_w = 110.4;
        let en_w = 101.4;
        let x = row.right() - 20.0 - 3.0 - ru_w - en_w - 3.0;
        let y = row.center().y - 16.0;
        (Rect::new(x, y, ru_w, 32.0), Rect::new(x + ru_w, y, en_w, 32.0))
    }

    fn segment(&self, canvas: &mut Canvas, area: Rect, flag: &'static str, label: &str, active: bool, hover: f32) {
        if active {
            canvas.fill(area, Metrics::PILL, Palette::SURFACE_3);
        }
        let color = if active { Palette::TEXT } else { Palette::TEXT_2.mix(Palette::TEXT, hover) };
        let style = TextStyle::new(14.0, 500.0);
        let layout = canvas.layout(label, style);
        let content = 18.0 + 8.0 + layout.width;
        let x = area.center().x - content * 0.5;
        draw_flag(canvas, flag, Rect::new(x, area.center().y - 6.5, 18.0, 13.0));
        let baseline = layout.baseline_in(area.y + (area.h - layout.line_height()) * 0.5, layout.line_height());
        canvas.draw_layout(&layout, x + 18.0 + 8.0, baseline, color);
    }

    fn verify_row(&mut self, canvas: &mut Canvas, state: &State, row: Rect) {
        self.icon_tile(canvas, row, "shield-check");
        self.row_text(canvas, row, &text(state.language, "settings.verify"), Some((&text(state.language, "settings.verifyHint"), 420.0)));
        self.verify.label = text(state.language, "settings.verifyBtn");
        let width = 124.0;
        self.verify.draw(canvas, Rect::new(row.right() - 20.0 - width, row.center().y - 17.0, width, 34.0));
    }

    fn footer(&mut self, canvas: &mut Canvas, state: &State, y: f32) {
        let version = format(state.language, "settings.version", &[("version", Config::VERSION)]);
        canvas.text(&version, TextStyle::new(13.0, 400.0), CARD_X + 4.0, y, Palette::TEXT_3);
        let (site, support) = self.footer_rects(canvas, state);
        self.footer_link(canvas, site, &text(state.language, "settings.site"), self.site.hover());
        self.footer_link(canvas, support, &text(state.language, "settings.support"), self.support.hover());
    }

    fn footer_rects(&self, canvas: &mut Canvas, state: &State) -> (Rect, Rect) {
        let style = TextStyle::new(13.0, 400.0);
        let layout = self.layout();
        let support_label = text(state.language, "settings.support");
        let site_label = text(state.language, "settings.site");
        let support_w = canvas.layout(&support_label, style).width;
        let site_w = canvas.layout(&site_label, style).width;
        let right = CARD_X + CARD_W - 4.0;
        let support = Rect::new(right - support_w, layout.footer, support_w, 19.5);
        let site = Rect::new(support.x - 16.0 - site_w, layout.footer, site_w, 19.5);
        (site, support)
    }

    fn footer_link(&self, canvas: &mut Canvas, area: Rect, label: &str, hover: f32) {
        let style = TextStyle::new(13.0, 400.0);
        let layout = canvas.layout(label, style);
        let baseline = layout.baseline_in(area.y, 19.5);
        canvas.draw_layout(&layout, area.x, baseline, Palette::FOCUS);
        if hover > 0.5 {
            canvas.fill(Rect::new(area.x, area.y + 17.0, layout.width, 1.0), 0.0, Palette::FOCUS);
        }
    }

    fn screen_point(&self, pointer: &Pointer) -> Point {
        Point::new(pointer.position.x, pointer.position.y + self.scroll)
    }

    pub fn track(&mut self, canvas: &mut Canvas, state: &State, pointer: &Pointer) {
        let layout = self.layout();
        let p = Pointer { position: self.screen_point(pointer), pressed: pointer.pressed };
        self.open.track(self.button_rect(layout.folder_row, 108.5), &p);
        self.verify.track(self.button_rect(layout.verify_row, 124.0), &p);
        let (ru, en) = self.segmented_rects(layout.language_row);
        self.lang_ru.track(ru, &p);
        self.lang_en.track(en, &p);
        let (site, support) = self.footer_rects(canvas, state);
        self.site.track(site, &p);
        self.support.track(support, &p);
    }

    fn button_rect(&self, row: Rect, width: f32) -> Rect {
        Rect::new(row.right() - 20.0 - width, row.center().y - 17.0, width, 34.0)
    }

    pub fn press(&mut self, state: &State, pointer: &Pointer) -> SettingsAction {
        let layout = self.layout();
        let p = Pointer { position: self.screen_point(pointer), pressed: pointer.pressed };
        self.open.press(self.button_rect(layout.folder_row, 108.5), &p);
        self.verify.press(self.button_rect(layout.verify_row, 124.0), &p);
        let track = self.memory_track(layout.memory_row);
        if Rect::new(track.x - 11.0, track.y - 11.0, track.w + 22.0, track.h + 22.0).contains(p.position) {
            self.dragging_memory = true;
            return self.memory_from(&p, layout.memory_row, state);
        }
        SettingsAction::None
    }

    fn memory_from(&self, pointer: &Pointer, row: Rect, state: &State) -> SettingsAction {
        let track = self.memory_track(row);
        let total = Self::total_memory(state);
        let fraction = ((pointer.position.x - track.x) / track.w).clamp(0.0, 1.0);
        let value = Config::MEMORY_MIN + (fraction * (total - Config::MEMORY_MIN) as f32).round() as u32;
        SettingsAction::Memory(value.clamp(Config::MEMORY_MIN, total))
    }

    pub fn dragging(&self) -> bool {
        self.dragging_memory
    }

    pub fn drag(&mut self, state: &State, pointer: &Pointer) -> SettingsAction {
        if !self.dragging_memory {
            return SettingsAction::None;
        }
        let layout = self.layout();
        let p = Pointer { position: self.screen_point(pointer), pressed: pointer.pressed };
        self.memory_from(&p, layout.memory_row, state)
    }

    pub fn release(&mut self, canvas: &mut Canvas, state: &State, pointer: &Pointer) -> SettingsAction {
        self.dragging_memory = false;
        let layout = self.layout();
        let p = Pointer { position: self.screen_point(pointer), pressed: false };
        if self.open.release(self.button_rect(layout.folder_row, 108.5), &p) {
            return SettingsAction::OpenFolder;
        }
        if self.verify.release(self.button_rect(layout.verify_row, 124.0), &p) {
            return SettingsAction::Verify;
        }
        let (ru, en) = self.segmented_rects(layout.language_row);
        if ru.contains(p.position) {
            return SettingsAction::Language(Language::Ru);
        }
        if en.contains(p.position) {
            return SettingsAction::Language(Language::En);
        }
        let (site, support) = self.footer_rects(canvas, state);
        if site.contains(p.position) {
            return SettingsAction::OpenUrl("/".into());
        }
        if support.contains(p.position) {
            return SettingsAction::OpenUrl("/contacts".into());
        }
        SettingsAction::None
    }

    pub fn animating(&self) -> bool {
        self.open.animating()
            || self.verify.animating()
            || self.lang_ru.animating()
            || self.lang_en.animating()
            || self.site.animating()
            || self.support.animating()
            || self.dragging_memory
    }
}

fn draw_flag(canvas: &mut Canvas, flag: &str, rect: Rect) {
    canvas.image(flag, rect, 3.0);
}

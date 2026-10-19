use crate::core::state::{Screen, State};
use crate::geometry::{Point, Rect};
use crate::gfx::canvas::{Canvas, Layer};
use crate::i18n::text;
use crate::text::font::TextStyle;
use crate::theme::{Color, Metrics, Palette};
use crate::ui::home::role_style;
use crate::ui::interaction::{Command, Hover, Pointer};

const BUTTON: f32 = 34.0;
const ICON: f32 = 16.0;
const PAD_RIGHT: f32 = 10.0;
const PAD_LEFT: f32 = 18.0;
const BRAND_TOP: f32 = 11.75;
const BRAND_MARK: f32 = 26.0;
const TAB_H: f32 = 34.0;
const TAB_Y: f32 = 11.0;
const AVATAR: f32 = 32.0;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BarIntent {
    None,
    Navigate(Screen),
    ToggleMenu,
    CloseMenu,
    Cabinet,
    Logout,
}

struct NavTab {
    screen: Screen,
    key: &'static str,
    icon: &'static str,
    hover: Hover,
}

struct WindowButton {
    icon: &'static str,
    command: Command,
    danger: bool,
    area: Rect,
    state: Hover,
}

impl WindowButton {
    fn new(icon: &'static str, command: Command, danger: bool, area: Rect) -> Self {
        Self {
            icon,
            command,
            danger,
            area,
            state: Hover::new(),
        }
    }

    fn draw(&self, canvas: &mut Canvas) {
        let h = self.state.hover();
        let (background, foreground) = if self.danger {
            (Palette::RED.alpha(h), Palette::TEXT_2.mix(Color::rgb(0xffffff), h))
        } else {
            (Color::rgba(0xf5f5f7, 0.08 * h), Palette::TEXT_2.mix(Palette::TEXT, h))
        };
        let scale = 1.0 - 0.06 * self.state.pressed();
        let area = self.area;
        canvas.with(Layer::new().scale(scale, area.center()), |c| {
            c.fill(area, Metrics::RADIUS_SM, background);
            c.icon(self.icon, Rect::centered(area.center(), ICON, ICON), foreground);
        });
    }
}

pub struct Titlebar {
    buttons: [WindowButton; 2],
    tabs: [NavTab; 2],
    avatar: Hover,
    cabinet: Hover,
    logout: Hover,
}

impl Titlebar {
    pub fn new() -> Self {
        let y = (Metrics::BAR_H - BUTTON) * 0.5;
        let close = Rect::new(Metrics::WINDOW_W - PAD_RIGHT - BUTTON, y, BUTTON, BUTTON);
        let minimize = close.offset(-(BUTTON + 2.0), 0.0);
        Self {
            buttons: [
                WindowButton::new("minus", Command::Minimize, false, minimize),
                WindowButton::new("x", Command::Close, true, close),
            ],
            tabs: [
                NavTab { screen: Screen::Home, key: "nav.home", icon: "home", hover: Hover::new() },
                NavTab { screen: Screen::Settings, key: "nav.settings", icon: "settings", hover: Hover::new() },
            ],
            avatar: Hover::new(),
            cabinet: Hover::new(),
            logout: Hover::new(),
        }
    }

    fn tab_layout(&self, canvas: &mut Canvas, lang: crate::i18n::Language) -> [Rect; 2] {
        let style = TextStyle::new(14.0, 500.0);
        let widths: Vec<f32> = self
            .tabs
            .iter()
            .map(|tab| 14.0 + 16.0 + 7.0 + canvas.layout(&text(lang, tab.key), style).width + 14.0)
            .collect();
        let total = widths[0] + 2.0 + widths[1];
        let mut x = Metrics::WINDOW_W * 0.5 - total * 0.5;
        let mut rects = [Rect::default(); 2];
        for (index, width) in widths.iter().enumerate() {
            rects[index] = Rect::new(x, TAB_Y, *width, TAB_H);
            x += width + 2.0;
        }
        rects
    }

    fn avatar_area(&self) -> Rect {
        Rect::new(self.controls_left() - 12.0 - AVATAR, 12.0, AVATAR, AVATAR)
    }

    fn menu_rect(&self) -> Rect {
        let avatar = self.avatar_area();
        Rect::new(avatar.right() - 240.0, avatar.bottom() + 10.0, 240.0, 168.0)
    }

    fn menu_items(&self) -> [Rect; 2] {
        let menu = self.menu_rect();
        let first = Rect::new(menu.x + 6.0, menu.y + 86.0, 228.0, 38.0);
        [first, first.offset(0.0, 38.0)]
    }

    fn controls_left(&self) -> f32 {
        self.buttons[0].area.x - 8.0 - 1.0
    }

    pub fn area() -> Rect {
        Rect::new(0.0, 0.0, Metrics::WINDOW_W, Metrics::BAR_H)
    }

    pub fn draw(&self, canvas: &mut Canvas) {
        canvas.icon("mark", Rect::new(PAD_LEFT, BRAND_TOP, BRAND_MARK, BRAND_MARK), Palette::TEXT);
        let style = TextStyle::new(16.0, 650.0).tracking(-0.02);
        let layout = canvas.layout("AlekDLC", style);
        let line = layout.line_height();
        let top = BRAND_TOP + (BRAND_MARK - line) * 0.5;
        let baseline = layout.baseline_in(top, line);
        canvas.draw_layout(&layout, PAD_LEFT + BRAND_MARK + 9.0, baseline, Palette::TEXT);
        let divider = self.controls_left();
        canvas.fill(
            Rect::new(divider, (Metrics::BAR_H - BUTTON) * 0.5, 1.0, BUTTON),
            0.0,
            Palette::BORDER,
        );
        for button in &self.buttons {
            button.draw(canvas);
        }
    }

    pub fn draw_nav(&self, canvas: &mut Canvas, state: &State) {
        let Some(account) = &state.account else { return };
        let lang = state.language;
        let rects = self.tab_layout(canvas, lang);
        let style = TextStyle::new(14.0, 500.0);
        for (index, tab) in self.tabs.iter().enumerate() {
            let area = rects[index];
            let active = state.screen == tab.screen;
            let hover = tab.hover.hover();
            let bg = if active {
                Color::rgba(0xf5f5f7, 0.09)
            } else {
                Color::rgba(0xf5f5f7, 0.06 * hover)
            };
            canvas.fill(area, Metrics::PILL, bg);
            let color = if active { Palette::TEXT } else { Palette::TEXT_2.mix(Palette::TEXT, hover) };
            let label = text(lang, tab.key);
            let layout = canvas.layout(&label, style);
            canvas.icon(tab.icon, Rect::new(area.x + 14.0, area.center().y - 8.0, 16.0, 16.0), color.alpha(0.9));
            let baseline = layout.baseline_in(area.y + (area.h - layout.line_height()) * 0.5, layout.line_height());
            canvas.draw_layout(&layout, area.x + 14.0 + 16.0 + 7.0, baseline, color);
        }
        self.avatar(canvas, &account.profile, self.avatar.hover() + if state.menu_open { 1.0 } else { 0.0 });
        if state.menu_open {
            self.menu(canvas, state, &account.profile);
        }
    }

    fn avatar(&self, canvas: &mut Canvas, profile: &crate::net::api::Profile, emphasis: f32) {
        let area = self.avatar_area();
        if emphasis > 0.01 {
            canvas.outline(area, Metrics::PILL, 3.0, Color::rgba(0xf5f5f7, 0.12 * emphasis.min(1.0)));
        }
        canvas.fill(area, Metrics::PILL, Palette::SURFACE_3);
        canvas.inset_border(area, Metrics::PILL, 1.0, Palette::BORDER_STRONG);
        let initial = profile.username.chars().next().unwrap_or('?').to_uppercase().to_string();
        let style = TextStyle::new(14.0, 600.0);
        canvas.text_centered(&initial, style, area.center().x, area.center().y - style.size * 0.75, Palette::TEXT);
    }

    fn menu(&self, canvas: &mut Canvas, state: &State, profile: &crate::net::api::Profile) {
        let menu = self.menu_rect();
        canvas.shadow(menu, Metrics::RADIUS, crate::gfx::canvas::Shadow::new(0.0, 24.0, 64.0, -24.0, Color::rgba(0, 0.9)));
        canvas.fill(menu, Metrics::RADIUS, Palette::SURFACE_2);
        canvas.inset_border(menu, Metrics::RADIUS, 1.0, Color::rgba(0xffffff, 0.06));
        let avatar = Rect::new(menu.x + 16.0, menu.y + 16.0, 40.0, 40.0);
        canvas.fill(avatar, Metrics::PILL, Palette::SURFACE_3);
        canvas.inset_border(avatar, Metrics::PILL, 1.0, Palette::BORDER_STRONG);
        let initial = profile.username.chars().next().unwrap_or('?').to_uppercase().to_string();
        canvas.text_centered(&initial, TextStyle::new(16.0, 600.0), avatar.center().x, avatar.center().y - 12.0, Palette::TEXT);
        let name_x = avatar.right() + 12.0;
        let name_style = TextStyle::new(15.0, 600.0);
        let name = canvas.layout(&profile.username, name_style);
        let badge_h = 24.0;
        let block_h = name.line_height() + 4.0 + badge_h;
        let block_top = avatar.center().y - block_h * 0.5;
        let baseline = name.baseline_in(block_top, name.line_height());
        canvas.draw_layout(&name, name_x, baseline, Palette::TEXT);
        self.role_chip(canvas, state, profile, name_x, block_top + name.line_height() + 4.0);
        canvas.fill(Rect::new(menu.x + 16.0, menu.y + 72.0, menu.w - 32.0, 1.0), 0.0, Palette::BORDER);
        let items = self.menu_items();
        self.menu_item(canvas, state, items[0], "user", "menu.cabinet", self.cabinet.hover(), false);
        self.menu_item(canvas, state, items[1], "log-out", "menu.logout", self.logout.hover(), true);
    }

    fn role_chip(&self, canvas: &mut Canvas, state: &State, profile: &crate::net::api::Profile, x: f32, y: f32) {
        let (glyph, fill, tint) = role_style(&profile.role);
        let label = text(state.language, &format!("role.{}", profile.role));
        let style = TextStyle::new(13.0, 600.0).tracking(0.0);
        let layout = canvas.layout(&label, style);
        let width = 10.0 + 13.0 + 5.0 + layout.width + 10.0;
        let rect = Rect::new(x, y, width, 24.0);
        canvas.fill(rect, Metrics::PILL, fill);
        canvas.icon(glyph, Rect::new(rect.x + 10.0, rect.center().y - 6.5, 13.0, 13.0), tint);
        let baseline = layout.baseline_in(rect.y + (rect.h - layout.line_height()) * 0.5, layout.line_height());
        canvas.draw_layout(&layout, rect.x + 10.0 + 13.0 + 5.0, baseline, tint);
    }

    fn menu_item(&self, canvas: &mut Canvas, state: &State, area: Rect, icon: &'static str, key: &str, hover: f32, danger: bool) {
        let base = if danger { Palette::RED } else { Palette::TEXT };
        let icon_color = if danger { Palette::RED.mix(Palette::TEXT_2, 1.0 - hover) } else { Palette::TEXT_2 };
        canvas.fill(area, 10.0, Color::rgba(0xf5f5f7, 0.07 * hover));
        canvas.icon(icon, Rect::new(area.x + 10.0, area.center().y - 8.0, 16.0, 16.0), if danger { base } else { icon_color });
        let style = TextStyle::new(14.0, 500.0);
        let layout = canvas.layout(&text(state.language, key), style);
        let baseline = layout.baseline_in(area.y + (area.h - layout.line_height()) * 0.5, layout.line_height());
        canvas.draw_layout(&layout, area.x + 10.0 + 16.0 + 10.0, baseline, base);
    }

    pub fn pointer_moved(&mut self, pointer: &Pointer) {
        for button in &mut self.buttons {
            button.state.track(button.area, pointer);
        }
    }

    pub fn track_nav(&mut self, canvas: &mut Canvas, state: &State, pointer: &Pointer) {
        if state.account.is_none() {
            return;
        }
        let rects = self.tab_layout(canvas, state.language);
        for (index, rect) in rects.iter().enumerate() {
            self.tabs[index].hover.track(*rect, pointer);
        }
        self.avatar.track(self.avatar_area(), pointer);
        if state.menu_open {
            let items = self.menu_items();
            self.cabinet.track(items[0], pointer);
            self.logout.track(items[1], pointer);
        }
    }

    pub fn nav_intent(&mut self, canvas: &mut Canvas, state: &State, pointer: &Pointer, release: bool) -> BarIntent {
        if state.account.is_none() {
            return BarIntent::None;
        }
        if state.menu_open {
            let items = self.menu_items();
            if release {
                if items[0].contains(pointer.position) {
                    return BarIntent::Cabinet;
                }
                if items[1].contains(pointer.position) {
                    return BarIntent::Logout;
                }
                let menu = self.menu_rect();
                if !menu.contains(pointer.position) && !self.avatar_area().contains(pointer.position) {
                    return BarIntent::CloseMenu;
                }
            }
        }
        if self.avatar_area().contains(pointer.position) {
            return if release { BarIntent::ToggleMenu } else { BarIntent::None };
        }
        if release {
            let rects = self.tab_layout(canvas, state.language);
            for (index, rect) in rects.iter().enumerate() {
                if rect.contains(pointer.position) {
                    return BarIntent::Navigate(self.tabs[index].screen);
                }
            }
        }
        BarIntent::None
    }

    pub fn nav_hit(&self, canvas: &mut Canvas, state: &State, pointer: Point) -> bool {
        if state.account.is_none() {
            return false;
        }
        if self.avatar_area().contains(pointer) {
            return true;
        }
        if state.menu_open {
            let menu = self.menu_rect();
            if menu.contains(pointer) {
                return true;
            }
        }
        self.tab_layout(canvas, state.language).iter().any(|r| r.contains(pointer))
    }

    pub fn press_nav(&mut self, canvas: &mut Canvas, state: &State, pointer: &Pointer) {
        if state.account.is_none() {
            return;
        }
        let rects = self.tab_layout(canvas, state.language);
        for (index, rect) in rects.iter().enumerate() {
            self.tabs[index].hover.press(*rect, pointer);
        }
        self.avatar.press(self.avatar_area(), pointer);
    }

    pub fn nav_animating(&self, menu_open: bool) -> bool {
        self.tabs.iter().any(|t| t.hover.animating())
            || self.avatar.animating()
            || (menu_open && (self.cabinet.animating() || self.logout.animating()))
    }

    pub fn pressed(&mut self, pointer: &Pointer) -> Command {
        let mut hit = false;
        for button in &mut self.buttons {
            hit |= button.state.press(button.area, pointer);
        }
        if !hit && Self::area().contains(pointer.position) {
            return Command::DragWindow;
        }
        Command::None
    }

    pub fn released(&mut self, pointer: &Pointer) -> Command {
        let mut command = Command::None;
        for button in &mut self.buttons {
            if button.state.release(button.area, pointer) {
                command = button.command;
            }
        }
        command
    }

    pub fn animating(&self) -> bool {
        self.buttons.iter().any(|b| b.state.animating())
    }

    pub fn contains(&self, p: Point) -> bool {
        Self::area().contains(p)
    }
}

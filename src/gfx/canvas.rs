use std::sync::Arc;

use glow::HasContext;

use crate::geometry::{Point, Rect};
use crate::gfx::atlas::Atlas;
use crate::gfx::batch::{Batch, Extra, Quad};
use crate::icons::IconSet;
use crate::text::engine::{Layout, TextEngine};
use crate::text::font::TextStyle;
use crate::theme::Color;

#[derive(Clone, Copy, Debug)]
pub struct Shadow {
    pub dx: f32,
    pub dy: f32,
    pub blur: f32,
    pub spread: f32,
    pub color: Color,
}

impl Shadow {
    pub const fn new(dx: f32, dy: f32, blur: f32, spread: f32, color: Color) -> Self {
        Self { dx, dy, blur, spread, color }
    }
}

#[derive(Clone, Copy, Debug)]
struct Frame {
    scale: f32,
    tx: f32,
    ty: f32,
    alpha: f32,
}

impl Frame {
    const IDENTITY: Self = Self { scale: 1.0, tx: 0.0, ty: 0.0, alpha: 1.0 };

    fn apply(&self, p: Point) -> Point {
        Point::new(p.x * self.scale + self.tx, p.y * self.scale + self.ty)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Layer {
    pub translate: Point,
    pub scale: f32,
    pub origin: Point,
    pub opacity: f32,
}

impl Layer {
    pub const fn new() -> Self {
        Self {
            translate: Point::new(0.0, 0.0),
            scale: 1.0,
            origin: Point::new(0.0, 0.0),
            opacity: 1.0,
        }
    }

    pub const fn opacity(mut self, value: f32) -> Self {
        self.opacity = value;
        self
    }

    pub const fn translate(mut self, x: f32, y: f32) -> Self {
        self.translate = Point::new(x, y);
        self
    }

    pub const fn scale(mut self, value: f32, origin: Point) -> Self {
        self.scale = value;
        self.origin = origin;
        self
    }
}

pub struct Canvas {
    gl: Arc<glow::Context>,
    batch: Batch,
    atlas: Atlas,
    images: Atlas,
    flags: crate::flags::FlagSet,
    text: TextEngine,
    icons: IconSet,
    dpi: f32,
    width: u32,
    height: u32,
    corner: f32,
    frames: Vec<Frame>,
}

impl Canvas {
    pub fn new(gl: Arc<glow::Context>) -> Result<Self, String> {
        Ok(Self {
            batch: Batch::new(gl.clone())?,
            atlas: Atlas::new(gl.clone(), 2048),
            images: Atlas::with_format(gl.clone(), 1024, 4, glow::TEXTURE1),
            flags: crate::flags::FlagSet::load(),
            text: TextEngine::new(),
            icons: IconSet::load(),
            gl,
            dpi: 1.0,
            width: 1,
            height: 1,
            corner: 0.0,
            frames: vec![Frame::IDENTITY],
        })
    }

    pub fn begin(&mut self, width: u32, height: u32, dpi: f32, corner: f32, clear: Color) {
        self.width = width.max(1);
        self.height = height.max(1);
        self.dpi = dpi;
        self.corner = corner * dpi;
        self.frames.truncate(1);
        unsafe {
            self.gl.viewport(0, 0, self.width as i32, self.height as i32);
            self.gl.disable(glow::DEPTH_TEST);
            self.gl.enable(glow::BLEND);
            self.gl.blend_func(glow::ONE, glow::ONE_MINUS_SRC_ALPHA);
            self.gl.clear_color(0.0, 0.0, 0.0, 0.0);
            self.gl.clear(glow::COLOR_BUFFER_BIT);
        }
        let logical = Rect::new(0.0, 0.0, self.width as f32 / dpi, self.height as f32 / dpi);
        self.fill(logical, 0.0, clear);
    }

    pub fn finish(&mut self) {
        self.atlas.bind();
        self.batch
            .flush(self.width as f32, self.height as f32, self.corner);
    }

    pub fn clip(&mut self, rect: Option<Rect>) {
        self.atlas.bind();
        self.batch.flush(self.width as f32, self.height as f32, self.corner);
        unsafe {
            match rect {
                Some(r) => {
                    let x = (r.x * self.dpi).round() as i32;
                    let w = (r.w * self.dpi).round() as i32;
                    let bottom = ((r.y + r.h) * self.dpi).round() as i32;
                    self.gl.enable(glow::SCISSOR_TEST);
                    self.gl.scissor(x, self.height as i32 - bottom, w.max(0), ((r.h * self.dpi).round() as i32).max(0));
                }
                None => self.gl.disable(glow::SCISSOR_TEST),
            }
        }
    }

    fn frame(&self) -> Frame {
        *self.frames.last().unwrap_or(&Frame::IDENTITY)
    }

    pub fn push(&mut self, layer: Layer) {
        let current = self.frame();
        let s = layer.scale;
        let lx = layer.origin.x * (1.0 - s) + layer.translate.x;
        let ly = layer.origin.y * (1.0 - s) + layer.translate.y;
        self.frames.push(Frame {
            scale: current.scale * s,
            tx: current.scale * lx + current.tx,
            ty: current.scale * ly + current.ty,
            alpha: current.alpha * layer.opacity,
        });
    }

    pub fn pop(&mut self) {
        if self.frames.len() > 1 {
            self.frames.pop();
        }
    }

    pub fn with<F: FnOnce(&mut Self)>(&mut self, layer: Layer, draw: F) {
        self.push(layer);
        draw(self);
        self.pop();
    }

    fn physical(&self, rect: Rect) -> Rect {
        let f = self.frame();
        let p = f.apply(Point::new(rect.x, rect.y));
        let k = f.scale * self.dpi;
        Rect::new(p.x * self.dpi, p.y * self.dpi, rect.w * k, rect.h * k)
    }

    fn unit(&self) -> f32 {
        self.frame().scale * self.dpi
    }

    fn tint(&self, color: Color) -> [f32; 4] {
        color.alpha(self.frame().alpha).array()
    }

    pub fn fill(&mut self, rect: Rect, radius: f32, color: Color) {
        if color.a <= 0.0 {
            return;
        }
        let r = self.physical(rect);
        let k = self.unit();
        let bounds = r.inset(-1.0, -1.0);
        self.batch.push(
            Quad {
                bounds,
                center: r.center(),
                shape: [r.w * 0.5, r.h * 0.5, radius * k, 0.0],
                color: self.tint(color),
                params: [0.0, 0.0, 0.0, 0.0],
                extra: Extra::Constant([0.0; 4]),
            },
            self.atlas.size() as f32,
        );
    }

    pub fn ring(&mut self, rect: Rect, radius: f32, inner: f32, outer: f32, color: Color) {
        if color.a <= 0.0 {
            return;
        }
        let r = self.physical(rect);
        let k = self.unit();
        let grow = outer.max(0.0) * k + 1.0;
        self.batch.push(
            Quad {
                bounds: r.inset(-grow, -grow),
                center: r.center(),
                shape: [r.w * 0.5, r.h * 0.5, radius * k, 0.0],
                color: self.tint(color),
                params: [1.0, outer * k, inner * k, 0.0],
                extra: Extra::Constant([0.0; 4]),
            },
            self.atlas.size() as f32,
        );
    }

    pub fn inset_border(&mut self, rect: Rect, radius: f32, width: f32, color: Color) {
        self.ring(rect, radius, -width, 0.0, color);
    }

    pub fn outline(&mut self, rect: Rect, radius: f32, width: f32, color: Color) {
        self.ring(rect, radius, 0.0, width, color);
    }

    pub fn shadow(&mut self, rect: Rect, radius: f32, shadow: Shadow) {
        if shadow.color.a <= 0.0 {
            return;
        }
        let element = self.physical(rect);
        let k = self.unit();
        let spread = shadow.spread * k;
        let body = Rect::new(
            element.x + shadow.dx * k - spread,
            element.y + shadow.dy * k - spread,
            element.w + spread * 2.0,
            element.h + spread * 2.0,
        );
        let sigma = shadow.blur * 0.5 * k;
        let reach = sigma * 3.0 + 1.0;
        let center = body.center();
        let element_center = element.center();
        self.batch.push(
            Quad {
                bounds: body.inset(-reach, -reach),
                center,
                shape: [
                    body.w * 0.5,
                    body.h * 0.5,
                    ((radius + shadow.spread) * k).max(0.0),
                    radius * k,
                ],
                color: self.tint(shadow.color),
                params: [2.0, sigma, 1.0, 0.0],
                extra: Extra::Constant([
                    element.w * 0.5,
                    element.h * 0.5,
                    element_center.x - center.x,
                    element_center.y - center.y,
                ]),
            },
            self.atlas.size() as f32,
        );
    }

    pub fn radial(&mut self, area: Rect, center: Point, rx: f32, ry: f32, stop: f32, color: Color) {
        let r = self.physical(area);
        let k = self.unit();
        let f = self.frame();
        let c = f.apply(center);
        self.batch.push(
            Quad {
                bounds: r,
                center: Point::new(c.x * self.dpi, c.y * self.dpi),
                shape: [r.w * 0.5, r.h * 0.5, 0.0, 0.0],
                color: self.tint(color),
                params: [4.0, rx * k, ry * k, stop],
                extra: Extra::Constant([0.0; 4]),
            },
            self.atlas.size() as f32,
        );
    }

    pub fn spinner(&mut self, center: Point, diameter: f32, width: f32, angle: f32, head: Color, track: Color) {
        let rect = Rect::centered(center, diameter, diameter);
        let r = self.physical(rect);
        let k = self.unit();
        self.batch.push(
            Quad {
                bounds: r.inset(-1.0, -1.0),
                center: r.center(),
                shape: [r.w * 0.5, r.h * 0.5, 0.0, 0.0],
                color: self.tint(head),
                params: [5.0, width * k, angle, 0.0],
                extra: Extra::Constant(self.tint(track)),
            },
            self.atlas.size() as f32,
        );
    }

    pub fn image(&mut self, name: &str, rect: Rect, radius: f32) {
        let w = (rect.w * self.dpi).round().max(1.0) as u32;
        let h = (rect.h * self.dpi).round().max(1.0) as u32;
        let Some(region) = self.flags.image(&mut self.images, name, w, h) else {
            return;
        };
        let r = self.physical(rect);
        self.batch.push(
            Quad {
                bounds: r,
                center: r.center(),
                shape: [r.w * 0.5, r.h * 0.5, radius * self.dpi, 0.0],
                color: self.tint(Color::rgb(0xffffff)),
                params: [6.0, 0.0, 0.0, 0.0],
                extra: Extra::Uv(Rect::new(region.x as f32, region.y as f32, region.w as f32, region.h as f32)),
            },
            self.images.size() as f32,
        );
    }

    pub fn icon(&mut self, name: &str, rect: Rect, color: Color) {
        let r = self.physical(rect);
        let scale = self.frame().scale;
        let pixels = rect.w * self.dpi;
        let exact = (scale - 1.0).abs() < 1e-4;
        let (ix, bx) = Self::bin(r.x, exact);
        let (iy, by) = Self::bin(r.y, exact);
        let Some(region) = self.icons.image(&mut self.atlas, name, pixels, bx, by) else {
            return;
        };
        let (x, y) = if exact { (ix, iy) } else { (r.x, r.y) };
        let bounds = Rect::new(x, y, region.w as f32 * scale, region.h as f32 * scale);
        self.mask(bounds, region, color, (1.0, 0.0));
    }

    fn bin(value: f32, exact: bool) -> (f32, u8) {
        if !exact {
            return (value, 0);
        }
        let floor = value.floor();
        let bin = ((value - floor) * IconSet::BINS).round();
        if bin >= IconSet::BINS {
            (floor + 1.0, 0)
        } else {
            (floor, bin as u8)
        }
    }

    fn mask(&mut self, bounds: Rect, region: crate::gfx::atlas::Region, color: Color, gamma: (f32, f32)) {
        self.batch.push(
            Quad {
                bounds,
                center: bounds.center(),
                shape: [bounds.w * 0.5, bounds.h * 0.5, 0.0, 0.0],
                color: self.tint(color),
                params: [3.0, gamma.0, gamma.1, 0.0],
                extra: Extra::Uv(Rect::new(
                    region.x as f32,
                    region.y as f32,
                    region.w as f32,
                    region.h as f32,
                )),
            },
            self.atlas.size() as f32,
        );
    }

    pub fn layout(&mut self, text: &str, style: TextStyle) -> Layout {
        self.text.layout(text, style)
    }

    pub fn wrap(&mut self, text: &str, style: TextStyle, max_width: f32) -> Vec<Layout> {
        self.text.wrap(text, style, max_width)
    }

    pub fn paragraph_left(&mut self, text: &str, style: TextStyle, x: f32, top: f32, max_width: f32, color: Color) -> f32 {
        let lines = self.wrap(text, style, max_width);
        let line_height = lines.first().map(|l| l.line_height()).unwrap_or(0.0);
        let mut y = top;
        for layout in &lines {
            let baseline = layout.baseline_in(y, line_height);
            self.draw_layout(layout, x, baseline, color);
            y += line_height;
        }
        y - top
    }

    pub fn text_centered(&mut self, text: &str, style: TextStyle, center_x: f32, top: f32, color: Color) -> f32 {
        let layout = self.layout(text, style);
        let baseline = layout.baseline_in(top, layout.line_height());
        self.draw_layout(&layout, center_x - layout.width * 0.5, baseline, color);
        layout.line_height()
    }

    pub fn paragraph_centered(
        &mut self,
        text: &str,
        style: TextStyle,
        center_x: f32,
        top: f32,
        max_width: f32,
        color: Color,
    ) -> f32 {
        let lines = self.wrap(text, style, max_width);
        let line_height = lines.first().map(|l| l.line_height()).unwrap_or(0.0);
        let mut y = top;
        for layout in &lines {
            let baseline = layout.baseline_in(y, line_height);
            self.draw_layout(layout, center_x - layout.width * 0.5, baseline, color);
            y += line_height;
        }
        y - top
    }

    pub fn text_gamma(color: Color) -> (f32, f32) {
        let luminance = color.luminance();
        if luminance >= 0.5 {
            (0.81 - 0.30 * luminance, 0.0)
        } else {
            (1.0 + 0.6 * (0.5 - luminance), 1.0)
        }
    }

    pub fn draw_layout(&mut self, layout: &Layout, x: f32, baseline: f32, color: Color) {
        let f = self.frame();
        let anchor = f.apply(Point::new(x, baseline));
        let ax = anchor.x * self.dpi;
        let ay = anchor.y * self.dpi;
        let exact = (f.scale - 1.0).abs() < 1e-4;
        let pixel_size = layout.style.size * self.dpi;
        let gamma = Self::text_gamma(color);
        for glyph in &layout.glyphs {
            let gx = ax + glyph.x * self.dpi * f.scale;
            let (ix, bin) = if exact { TextEngine::subpixel(gx) } else { (gx.floor() as i32, 0) };
            let by = if exact { ay.round() } else { ay };
            let Some(image) = self
                .text
                .glyph(&mut self.atlas, glyph.id, &layout.style, pixel_size, bin)
            else {
                continue;
            };
            if image.region.w == 0 {
                continue;
            }
            let bounds = if exact {
                Rect::new(
                    (ix + image.left) as f32,
                    by - image.top as f32,
                    image.region.w as f32,
                    image.region.h as f32,
                )
            } else {
                let fx = gx.fract();
                Rect::new(
                    ix as f32 + fx + image.left as f32 * f.scale,
                    by - image.top as f32 * f.scale,
                    image.region.w as f32 * f.scale,
                    image.region.h as f32 * f.scale,
                )
            };
            self.mask(bounds, image.region, color, gamma);
        }
    }

    pub fn text(&mut self, text: &str, style: TextStyle, x: f32, top: f32, color: Color) -> Layout {
        let layout = self.layout(text, style);
        let baseline = layout.baseline_in(top, layout.line_height());
        self.draw_layout(&layout, x, baseline, color);
        layout
    }
}

use std::collections::HashMap;

use swash::scale::{Render, ScaleContext, Source};
use swash::shape::ShapeContext;
use swash::zeno::{Format, Vector};
use swash::FontRef;

use crate::gfx::atlas::{Atlas, Region};
use crate::text::font::{Font, TextStyle};

const SUBPIXEL_BINS: f32 = 4.0;

#[derive(Clone, Copy, Debug)]
pub struct PlacedGlyph {
    pub id: u16,
    pub x: f32,
}

#[derive(Clone, Debug)]
pub struct Layout {
    pub glyphs: Vec<PlacedGlyph>,
    pub width: f32,
    pub ascent: f32,
    pub descent: f32,
    pub style: TextStyle,
}

impl Layout {
    pub fn line_height(&self) -> f32 {
        self.style.size * self.style.line_height
    }

    pub fn baseline_in(&self, top: f32, box_height: f32) -> f32 {
        top + (box_height - (self.ascent + self.descent)) * 0.5 + self.ascent
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
struct GlyphKey {
    id: u16,
    size: u32,
    weight: u16,
    optical: u16,
    bin: u8,
}

#[derive(Clone, Copy, Debug)]
pub struct GlyphImage {
    pub region: Region,
    pub left: i32,
    pub top: i32,
}

pub struct TextEngine {
    font: FontRef<'static>,
    shaper: ShapeContext,
    scaler: ScaleContext,
    glyphs: HashMap<GlyphKey, Option<GlyphImage>>,
    layouts: HashMap<(String, u64), Layout>,
}

impl TextEngine {
    pub fn new() -> Self {
        Self {
            font: Font::inter(),
            shaper: ShapeContext::new(),
            scaler: ScaleContext::new(),
            glyphs: HashMap::new(),
            layouts: HashMap::new(),
        }
    }

    fn style_key(style: &TextStyle) -> u64 {
        let mut key = style.size.to_bits() as u64;
        key = key.wrapping_mul(31).wrapping_add(style.weight.to_bits() as u64);
        key = key.wrapping_mul(31).wrapping_add(style.tracking.to_bits() as u64);
        key = key.wrapping_mul(31).wrapping_add(style.line_height.to_bits() as u64);
        key.wrapping_mul(31).wrapping_add(style.tabular as u64)
    }

    pub fn layout(&mut self, text: &str, style: TextStyle) -> Layout {
        let key = (text.to_owned(), Self::style_key(&style));
        if let Some(layout) = self.layouts.get(&key) {
            return layout.clone();
        }
        let layout = self.shape(text, style);
        if self.layouts.len() > 4096 {
            self.layouts.clear();
        }
        self.layouts.insert(key, layout.clone());
        layout
    }

    fn shape(&mut self, text: &str, style: TextStyle) -> Layout {
        let tracking = style.tracking * style.size;
        let tnum = if style.tabular { 1u16 } else { 0u16 };
        let mut shaper = self
            .shaper
            .builder(self.font)
            .size(style.size)
            .variations([("wght", style.weight), ("opsz", style.optical_size())])
            .features([("tnum", tnum)])
            .build();
        shaper.add_str(text);
        let mut glyphs = Vec::with_capacity(text.len());
        let mut pen = 0.0f32;
        shaper.shape_with(|cluster| {
            for glyph in cluster.glyphs {
                glyphs.push(PlacedGlyph {
                    id: glyph.id,
                    x: pen + glyph.x,
                });
                pen += glyph.advance;
            }
            pen += tracking;
        });
        let metrics = self.font.metrics(&[]).scale(style.size);
        Layout {
            glyphs,
            width: pen,
            ascent: metrics.ascent.round(),
            descent: metrics.descent.round(),
            style,
        }
    }

    pub fn wrap(&mut self, text: &str, style: TextStyle, max_width: f32) -> Vec<Layout> {
        let mut lines = Vec::new();
        for paragraph in text.split('\n') {
            self.wrap_paragraph(paragraph, style, max_width, &mut lines);
        }
        lines
    }

    fn wrap_paragraph(&mut self, text: &str, style: TextStyle, max_width: f32, out: &mut Vec<Layout>) {
        let words: Vec<&str> = text.split(' ').collect();
        let mut current = String::new();
        for (index, word) in words.iter().enumerate() {
            let candidate = if current.is_empty() {
                (*word).to_owned()
            } else {
                format!("{current} {word}")
            };
            if index > 0 && self.layout(&candidate, style).width > max_width {
                out.push(self.layout(&current, style));
                current = (*word).to_owned();
            } else {
                current = candidate;
            }
        }
        out.push(self.layout(&current, style));
    }

    pub fn subpixel(x: f32) -> (i32, u8) {
        let floor = x.floor();
        let bin = ((x - floor) * SUBPIXEL_BINS).round();
        if bin >= SUBPIXEL_BINS {
            (floor as i32 + 1, 0)
        } else {
            (floor as i32, bin as u8)
        }
    }

    pub fn glyph(
        &mut self,
        atlas: &mut Atlas,
        id: u16,
        style: &TextStyle,
        pixel_size: f32,
        bin: u8,
    ) -> Option<GlyphImage> {
        let key = GlyphKey {
            id,
            size: (pixel_size * 64.0).round() as u32,
            weight: style.weight.round() as u16,
            optical: (style.optical_size() * 4.0).round() as u16,
            bin,
        };
        if let Some(cached) = self.glyphs.get(&key) {
            return *cached;
        }
        let mut scaler = self
            .scaler
            .builder(self.font)
            .size(pixel_size)
            .hint(false)
            .variations([("wght", style.weight), ("opsz", style.optical_size())])
            .build();
        let image = Render::new(&[Source::Outline])
            .format(Format::Alpha)
            .offset(Vector::new(bin as f32 / SUBPIXEL_BINS, 0.0))
            .render(&mut scaler, id);
        let result = image.and_then(|image| {
            let p = image.placement;
            atlas.insert(p.width, p.height, &image.data).map(|region| GlyphImage {
                region,
                left: p.left,
                top: p.top,
            })
        });
        self.glyphs.insert(key, result);
        result
    }
}

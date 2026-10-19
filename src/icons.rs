use std::collections::HashMap;

use resvg::tiny_skia::{Pixmap, Transform};
use resvg::usvg::{Options, Tree};

use crate::gfx::atlas::{Atlas, Region};

static SPRITE: &str = include_str!("../assets/icons.svg");

pub struct IconSet {
    symbols: HashMap<String, String>,
    images: HashMap<(String, u32, u8, u8), Option<Region>>,
}

impl IconSet {
    pub fn load() -> Self {
        Self {
            symbols: Self::parse(SPRITE),
            images: HashMap::new(),
        }
    }

    fn parse(sprite: &str) -> HashMap<String, String> {
        let mut symbols = HashMap::new();
        let mut rest = sprite;
        while let Some(start) = rest.find("<symbol") {
            let tail = &rest[start + "<symbol".len()..];
            let Some(open_end) = tail.find('>') else { break };
            let Some(close) = tail.find("</symbol>") else { break };
            let attributes = &tail[..open_end];
            let body = &tail[open_end + 1..close];
            if let Some(id) = Self::attribute(attributes, "id") {
                let attributes = attributes.replace(&format!("id=\"{id}\""), "");
                let svg = format!(
                    "<svg xmlns=\"http://www.w3.org/2000/svg\" {attributes}>{body}</svg>"
                )
                .replace("currentColor", "#ffffff");
                symbols.insert(id.to_owned(), svg);
            }
            rest = &tail[close + "</symbol>".len()..];
        }
        symbols
    }

    fn attribute<'a>(attributes: &'a str, name: &str) -> Option<&'a str> {
        let marker = format!("{name}=\"");
        let start = attributes.find(&marker)? + marker.len();
        let end = attributes[start..].find('"')? + start;
        Some(&attributes[start..end])
    }

    pub const BINS: f32 = 4.0;

    pub fn image(&mut self, atlas: &mut Atlas, name: &str, pixels: f32, bx: u8, by: u8) -> Option<Region> {
        let key = (name.to_owned(), (pixels * 64.0).round() as u32, bx, by);
        if let Some(cached) = self.images.get(&key) {
            return *cached;
        }
        let side = pixels.ceil() as u32 + 1;
        let region = self
            .rasterize(name, pixels, side, bx as f32 / Self::BINS, by as f32 / Self::BINS)
            .and_then(|mask| atlas.insert(side, side, &mask));
        self.images.insert(key, region);
        region
    }

    fn rasterize(&self, name: &str, pixels: f32, side: u32, dx: f32, dy: f32) -> Option<Vec<u8>> {
        let source = self.symbols.get(name)?;
        let tree = Tree::from_str(source, &Options::default()).ok()?;
        let mut pixmap = Pixmap::new(side, side)?;
        let size = tree.size();
        let transform = Transform::from_row(pixels / size.width(), 0.0, 0.0, pixels / size.height(), dx, dy);
        resvg::render(&tree, transform, &mut pixmap.as_mut());
        Some(pixmap.pixels().iter().map(|p| p.alpha()).collect())
    }
}

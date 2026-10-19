use std::collections::HashMap;

use resvg::tiny_skia::{Pixmap, Transform};
use resvg::usvg::{Options, Tree};

use crate::gfx::atlas::{Atlas, Region};

pub struct FlagSet {
    sources: HashMap<&'static str, &'static str>,
    cache: HashMap<(String, u32, u32), Region>,
}

impl FlagSet {
    pub fn load() -> Self {
        let mut sources = HashMap::new();
        sources.insert("ru", include_str!("../assets/flags/ru.svg"));
        sources.insert("gb", include_str!("../assets/flags/gb.svg"));
        Self {
            sources,
            cache: HashMap::new(),
        }
    }

    pub fn image(&mut self, atlas: &mut Atlas, name: &str, w: u32, h: u32) -> Option<Region> {
        if w == 0 || h == 0 {
            return None;
        }
        let key = (name.to_owned(), w, h);
        if let Some(region) = self.cache.get(&key) {
            return Some(*region);
        }
        let pixels = self.rasterize(name, w, h)?;
        let region = atlas.insert(w, h, &pixels)?;
        self.cache.insert(key, region);
        Some(region)
    }

    fn rasterize(&self, name: &str, w: u32, h: u32) -> Option<Vec<u8>> {
        let source = self.sources.get(name)?;
        let tree = Tree::from_str(source, &Options::default()).ok()?;
        let mut pixmap = Pixmap::new(w, h)?;
        let size = tree.size();
        let transform = Transform::from_scale(w as f32 / size.width(), h as f32 / size.height());
        resvg::render(&tree, transform, &mut pixmap.as_mut());
        Some(pixmap.data().to_vec())
    }
}

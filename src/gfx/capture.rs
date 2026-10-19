use std::path::PathBuf;
use std::sync::Arc;

use glow::HasContext;
use resvg::tiny_skia::{IntSize, Pixmap};

pub struct Capture {
    path: PathBuf,
    frames: u32,
}

impl Capture {
    pub fn from_env() -> Option<Self> {
        if !cfg!(debug_assertions) {
            return None;
        }
        let path = std::env::var_os("ALEK_CAPTURE")?;
        let frames = std::env::var("ALEK_CAPTURE_FRAMES")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(60);
        crate::anim::freeze();
        Some(Self {
            path: path.into(),
            frames,
        })
    }

    pub fn tick(&mut self, gl: &Arc<glow::Context>, width: u32, height: u32) -> bool {
        if self.frames > 0 {
            self.frames -= 1;
            return false;
        }
        let mut data = vec![0u8; (width * height * 4) as usize];
        unsafe {
            gl.pixel_store_i32(glow::PACK_ALIGNMENT, 1);
            gl.read_pixels(
                0,
                0,
                width as i32,
                height as i32,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelPackData::Slice(Some(&mut data)),
            );
        }
        let row = (width * 4) as usize;
        let flipped: Vec<u8> = data.chunks(row).rev().flatten().copied().collect();
        if let Some(pixmap) = IntSize::from_wh(width, height).and_then(|s| Pixmap::from_vec(flipped, s)) {
            let _ = pixmap.save_png(&self.path);
        }
        true
    }
}

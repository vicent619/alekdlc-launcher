use std::sync::Arc;

use glow::HasContext;

#[derive(Clone, Copy, Debug, Default)]
pub struct Region {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

pub struct Atlas {
    gl: Arc<glow::Context>,
    texture: glow::Texture,
    size: u32,
    channels: u32,
    unit: u32,
    cursor_x: u32,
    cursor_y: u32,
    shelf: u32,
}

impl Atlas {
    const PAD: u32 = 1;

    pub fn new(gl: Arc<glow::Context>, size: u32) -> Self {
        Self::with_format(gl, size, 1, glow::TEXTURE0)
    }

    pub fn with_format(gl: Arc<glow::Context>, size: u32, channels: u32, unit: u32) -> Self {
        let (internal, format) = if channels == 4 { (glow::RGBA8, glow::RGBA) } else { (glow::R8, glow::RED) };
        unsafe {
            let texture = gl.create_texture().expect("atlas texture");
            gl.active_texture(unit);
            gl.bind_texture(glow::TEXTURE_2D, Some(texture));
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::LINEAR as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::LINEAR as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE as i32);
            gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
            let zeros = vec![0u8; (size * size * channels) as usize];
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                internal as i32,
                size as i32,
                size as i32,
                0,
                format,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(Some(&zeros)),
            );
            if unit != glow::TEXTURE0 {
                gl.active_texture(glow::TEXTURE0);
            }
            Self {
                gl,
                texture,
                size,
                channels,
                unit,
                cursor_x: Self::PAD,
                cursor_y: Self::PAD,
                shelf: 0,
            }
        }
    }

    pub fn size(&self) -> u32 {
        self.size
    }

    pub fn bind(&self) {
        unsafe {
            self.gl.active_texture(self.unit);
            self.gl.bind_texture(glow::TEXTURE_2D, Some(self.texture));
            if self.unit != glow::TEXTURE0 {
                self.gl.active_texture(glow::TEXTURE0);
            }
        }
    }

    pub fn insert(&mut self, w: u32, h: u32, pixels: &[u8]) -> Option<Region> {
        if w == 0 || h == 0 {
            return Some(Region::default());
        }
        if self.cursor_x + w + Self::PAD > self.size {
            self.cursor_x = Self::PAD;
            self.cursor_y += self.shelf + Self::PAD;
            self.shelf = 0;
        }
        if self.cursor_y + h + Self::PAD > self.size {
            return None;
        }
        let region = Region {
            x: self.cursor_x,
            y: self.cursor_y,
            w,
            h,
        };
        self.cursor_x += w + Self::PAD;
        self.shelf = self.shelf.max(h);
        let format = if self.channels == 4 { glow::RGBA } else { glow::RED };
        unsafe {
            self.gl.active_texture(self.unit);
            self.gl.bind_texture(glow::TEXTURE_2D, Some(self.texture));
            self.gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
            self.gl.tex_sub_image_2d(
                glow::TEXTURE_2D,
                0,
                region.x as i32,
                region.y as i32,
                w as i32,
                h as i32,
                format,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(Some(pixels)),
            );
            if self.unit != glow::TEXTURE0 {
                self.gl.active_texture(glow::TEXTURE0);
            }
        }
        Some(region)
    }
}

impl Drop for Atlas {
    fn drop(&mut self) {
        unsafe { self.gl.delete_texture(self.texture) };
    }
}

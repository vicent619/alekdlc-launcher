use std::sync::Arc;

use glow::HasContext;

use crate::geometry::{Point, Rect};
use crate::gfx::program::Program;
use crate::gfx::shaders;

const FLOATS: usize = 20;

#[derive(Clone, Copy)]
pub enum Extra {
    Constant([f32; 4]),
    Uv(Rect),
}

#[derive(Clone, Copy)]
pub struct Quad {
    pub bounds: Rect,
    pub center: Point,
    pub shape: [f32; 4],
    pub color: [f32; 4],
    pub params: [f32; 4],
    pub extra: Extra,
}

pub struct Batch {
    gl: Arc<glow::Context>,
    program: Program,
    vao: glow::VertexArray,
    vbo: glow::Buffer,
    vertices: Vec<f32>,
}

impl Batch {
    pub fn new(gl: Arc<glow::Context>) -> Result<Self, String> {
        let program = Program::compile(gl.clone(), shaders::VERTEX, shaders::FRAGMENT)?;
        unsafe {
            let vao = gl.create_vertex_array()?;
            let vbo = gl.create_buffer()?;
            gl.bind_vertex_array(Some(vao));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
            let stride = (FLOATS * 4) as i32;
            let layout = [(0u32, 2i32, 0i32), (1, 2, 2), (2, 4, 4), (3, 4, 8), (4, 4, 12), (5, 4, 16)];
            for (index, size, offset) in layout {
                gl.enable_vertex_attrib_array(index);
                gl.vertex_attrib_pointer_f32(index, size, glow::FLOAT, false, stride, offset * 4);
            }
            gl.bind_vertex_array(None);
            Ok(Self {
                gl,
                program,
                vao,
                vbo,
                vertices: Vec::with_capacity(FLOATS * 6 * 4096),
            })
        }
    }

    pub fn push(&mut self, quad: Quad, atlas_size: f32) {
        let b = quad.bounds;
        let corners = [
            (b.x, b.y, 0.0, 0.0),
            (b.right(), b.y, 1.0, 0.0),
            (b.right(), b.bottom(), 1.0, 1.0),
            (b.x, b.y, 0.0, 0.0),
            (b.right(), b.bottom(), 1.0, 1.0),
            (b.x, b.bottom(), 0.0, 1.0),
        ];
        for (x, y, u, v) in corners {
            let extra = match quad.extra {
                Extra::Constant(values) => values,
                Extra::Uv(uv) => [
                    (uv.x + uv.w * u) / atlas_size,
                    (uv.y + uv.h * v) / atlas_size,
                    0.0,
                    0.0,
                ],
            };
            self.vertices.extend_from_slice(&[x, y, x - quad.center.x, y - quad.center.y]);
            self.vertices.extend_from_slice(&quad.shape);
            self.vertices.extend_from_slice(&quad.color);
            self.vertices.extend_from_slice(&quad.params);
            self.vertices.extend_from_slice(&extra);
        }
    }

    pub fn flush(&mut self, width: f32, height: f32, radius: f32) {
        if self.vertices.is_empty() {
            return;
        }
        unsafe {
            self.program.bind(width, height, radius);
            self.gl.bind_vertex_array(Some(self.vao));
            self.gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.vbo));
            let bytes = std::slice::from_raw_parts(
                self.vertices.as_ptr() as *const u8,
                self.vertices.len() * 4,
            );
            self.gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, bytes, glow::STREAM_DRAW);
            self.gl
                .draw_arrays(glow::TRIANGLES, 0, (self.vertices.len() / FLOATS) as i32);
            self.gl.bind_vertex_array(None);
        }
        self.vertices.clear();
    }
}

impl Drop for Batch {
    fn drop(&mut self) {
        unsafe {
            self.gl.delete_vertex_array(self.vao);
            self.gl.delete_buffer(self.vbo);
        }
    }
}

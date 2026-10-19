use std::sync::Arc;

use glow::HasContext;

pub struct Program {
    gl: Arc<glow::Context>,
    handle: glow::Program,
    viewport: Option<glow::UniformLocation>,
    clip: Option<glow::UniformLocation>,
    atlas: Option<glow::UniformLocation>,
    images: Option<glow::UniformLocation>,
}

impl Program {
    pub fn compile(gl: Arc<glow::Context>, vertex: &str, fragment: &str) -> Result<Self, String> {
        unsafe {
            let handle = gl.create_program()?;
            let stages = [(glow::VERTEX_SHADER, vertex), (glow::FRAGMENT_SHADER, fragment)];
            let mut shaders = Vec::with_capacity(stages.len());
            for (stage, source) in stages {
                let shader = gl.create_shader(stage)?;
                gl.shader_source(shader, source);
                gl.compile_shader(shader);
                if !gl.get_shader_compile_status(shader) {
                    return Err(gl.get_shader_info_log(shader));
                }
                gl.attach_shader(handle, shader);
                shaders.push(shader);
            }
            gl.link_program(handle);
            if !gl.get_program_link_status(handle) {
                return Err(gl.get_program_info_log(handle));
            }
            for shader in shaders {
                gl.detach_shader(handle, shader);
                gl.delete_shader(shader);
            }
            let viewport = gl.get_uniform_location(handle, "u_viewport");
            let clip = gl.get_uniform_location(handle, "u_clip");
            let atlas = gl.get_uniform_location(handle, "u_atlas");
            let images = gl.get_uniform_location(handle, "u_images");
            Ok(Self {
                gl,
                handle,
                viewport,
                clip,
                atlas,
                images,
            })
        }
    }

    pub fn bind(&self, width: f32, height: f32, radius: f32) {
        unsafe {
            self.gl.use_program(Some(self.handle));
            self.gl.uniform_2_f32(self.viewport.as_ref(), width, height);
            self.gl
                .uniform_4_f32(self.clip.as_ref(), width * 0.5, height * 0.5, radius, 0.0);
            self.gl.uniform_1_i32(self.atlas.as_ref(), 0);
            self.gl.uniform_1_i32(self.images.as_ref(), 1);
        }
    }
}

impl Drop for Program {
    fn drop(&mut self) {
        unsafe { self.gl.delete_program(self.handle) };
    }
}

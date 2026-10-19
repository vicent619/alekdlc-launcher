use std::error::Error;
use std::num::NonZeroU32;
use std::sync::Arc;

use glutin::config::{Config, ConfigTemplateBuilder, GlConfig};
use glutin::context::{
    ContextApi, ContextAttributesBuilder, NotCurrentGlContext, PossiblyCurrentContext, Version,
};
use glutin::display::{GetGlDisplay, GlDisplay};
use glutin::surface::{GlSurface, Surface, SwapInterval, WindowSurface};
use glutin_winit::{DisplayBuilder, GlWindow};
use raw_window_handle::HasWindowHandle;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowAttributes};

pub struct GlSurfaceTarget {
    window: Window,
    surface: Surface<WindowSurface>,
    context: PossiblyCurrentContext,
    gl: Arc<glow::Context>,
}

impl GlSurfaceTarget {
    pub fn open(
        event_loop: &ActiveEventLoop,
        attributes: WindowAttributes,
    ) -> Result<Self, Box<dyn Error>> {
        let template = ConfigTemplateBuilder::new()
            .with_alpha_size(8)
            .with_transparency(true);
        let (window, config) = DisplayBuilder::new()
            .with_window_attributes(Some(attributes))
            .build(event_loop, template, Self::pick)?;
        let window = window.ok_or("window")?;
        let display = config.display();
        let raw = window.window_handle()?.as_raw();
        let attributes = ContextAttributesBuilder::new()
            .with_context_api(ContextApi::OpenGl(Some(Version::new(3, 3))))
            .build(Some(raw));
        let context = unsafe { display.create_context(&config, &attributes)? };
        let surface_attributes = window.build_surface_attributes(Default::default())?;
        let surface = unsafe { display.create_window_surface(&config, &surface_attributes)? };
        let context = context.make_current(&surface)?;
        let _ = surface.set_swap_interval(&context, SwapInterval::Wait(NonZeroU32::MIN));
        let gl = unsafe { glow::Context::from_loader_function_cstr(|s| display.get_proc_address(s)) };
        Ok(Self {
            window,
            surface,
            context,
            gl: Arc::new(gl),
        })
    }

    fn pick(configs: Box<dyn Iterator<Item = Config> + '_>) -> Config {
        let mut all: Vec<Config> = configs.collect();
        all.sort_by_key(|c| {
            (
                !c.supports_transparency().unwrap_or(false),
                c.num_samples(),
            )
        });
        all.into_iter().next().expect("gl config")
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    pub fn gl(&self) -> Arc<glow::Context> {
        self.gl.clone()
    }

    pub fn resize(&self, width: u32, height: u32) {
        if let (Some(w), Some(h)) = (NonZeroU32::new(width), NonZeroU32::new(height)) {
            self.surface.resize(&self.context, w, h);
        }
    }

    pub fn present(&self) {
        let _ = self.surface.swap_buffers(&self.context);
    }
}

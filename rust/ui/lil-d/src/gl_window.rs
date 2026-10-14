//! The winit window with a glutin/glow OpenGL context.

use core::ffi::CStr;
use core::num::NonZeroU32;

use winit::raw_window_handle::HasWindowHandle as _;

pub struct GlWindow {
    window: winit::window::Window,
    gl_context: glutin::context::PossiblyCurrentContext,
    gl_display: glutin::display::Display,
    gl_surface: glutin::surface::Surface<glutin::surface::WindowSurface>,
}

impl GlWindow {
    /// Create the window and a current GL context.
    ///
    /// # Safety
    ///
    /// Must be called on the event-loop thread.
    pub unsafe fn new(
        event_loop: &winit::event_loop::ActiveEventLoop,
        title: &str,
        size: winit::dpi::LogicalSize<f64>,
    ) -> Self {
        use glutin::context::NotCurrentGlContext as _;
        use glutin::display::GetGlDisplay as _;
        use glutin::display::GlDisplay as _;
        use glutin::prelude::GlSurface as _;

        let window_attributes = winit::window::WindowAttributes::default()
            .with_resizable(true)
            .with_inner_size(size)
            .with_title(title)
            .with_visible(false);

        let config_template = glutin::config::ConfigTemplateBuilder::new()
            .prefer_hardware_accelerated(None)
            .with_depth_size(0)
            .with_stencil_size(0)
            .with_transparency(false);

        let (mut window, gl_config) = glutin_winit::DisplayBuilder::new()
            .with_preference(glutin_winit::ApiPreference::FallbackEgl)
            .with_window_attributes(Some(window_attributes.clone()))
            .build(event_loop, config_template, |mut configs| {
                configs.next().expect("no matching GL configuration")
            })
            .expect("failed to create GL configuration");

        let gl_display = gl_config.display();

        let raw_window_handle = window
            .as_ref()
            .map(|w| w.window_handle().expect("window handle").as_raw());
        let context_attributes =
            glutin::context::ContextAttributesBuilder::new().build(raw_window_handle);
        let fallback_context_attributes = glutin::context::ContextAttributesBuilder::new()
            .with_context_api(glutin::context::ContextApi::Gles(None))
            .build(raw_window_handle);
        let not_current_gl_context = gl_display
            .create_context(&gl_config, &context_attributes)
            .unwrap_or_else(|_| {
                gl_config
                    .display()
                    .create_context(&gl_config, &fallback_context_attributes)
                    .expect("failed to create GL context")
            });

        let window = window.take().unwrap_or_else(|| {
            glutin_winit::finalize_window(event_loop, window_attributes.clone(), &gl_config)
                .expect("failed to finalize window")
        });

        let (width, height): (u32, u32) = window.inner_size().into();
        let width = NonZeroU32::new(width).unwrap_or(NonZeroU32::MIN);
        let height = NonZeroU32::new(height).unwrap_or(NonZeroU32::MIN);
        let surface_attributes =
            glutin::surface::SurfaceAttributesBuilder::<glutin::surface::WindowSurface>::new()
                .build(
                    window.window_handle().expect("window handle").as_raw(),
                    width,
                    height,
                );
        let gl_surface = gl_display
            .create_window_surface(&gl_config, &surface_attributes)
            .expect("failed to create window surface");
        let gl_context = not_current_gl_context
            .make_current(&gl_surface)
            .expect("failed to make context current");

        gl_surface
            .set_swap_interval(
                &gl_context,
                glutin::surface::SwapInterval::Wait(NonZeroU32::MIN),
            )
            .expect("failed to set swap interval");

        Self {
            window,
            gl_context,
            gl_display,
            gl_surface,
        }
    }

    pub fn window(&self) -> &winit::window::Window {
        &self.window
    }

    pub fn size(&self) -> winit::dpi::PhysicalSize<u32> {
        self.window.inner_size()
    }

    pub fn scale_factor(&self) -> f64 {
        self.window.scale_factor()
    }

    pub fn resize(&self, width: NonZeroU32, height: NonZeroU32) {
        use glutin::surface::GlSurface as _;
        self.gl_surface.resize(&self.gl_context, width, height);
    }

    pub fn swap_buffers(&self) {
        use glutin::surface::GlSurface as _;
        self.gl_surface
            .swap_buffers(&self.gl_context)
            .expect("failed to swap buffers");
    }

    pub fn get_proc_address(&self, addr: &CStr) -> *const core::ffi::c_void {
        use glutin::display::GlDisplay as _;
        self.gl_display.get_proc_address(addr)
    }
}

//! The `lil-d` application: window, render loop, console, and state.

use std::sync::Arc;

use egui_glow::EguiGlow;
use glow::HasContext as _;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::WindowId;

use crate::cli::Args;
use crate::console::{self, ConsoleAction};
use crate::gl_window::GlWindow;
use crate::layout;

const WINDOW_TITLE: &str = "lil-d";
const WINDOW_WIDTH: f64 = 720.0;
const WINDOW_HEIGHT: f64 = 720.0;

/// Background of the console (and of the whole window).
const CONSOLE_BG: [f32; 4] = [0.06, 0.06, 0.07, 1.0];
/// Placeholder for the region delegated to the display library: dark,
/// unsaturated green.
const FORMAT_GREEN: [f32; 4] = [0.10, 0.18, 0.10, 1.0];

#[derive(Clone, Copy, Debug, Default)]
pub struct AppState {
    pub format_on: bool,
}

pub struct App {
    args: Args,
    state: AppState,
    gl_window: Option<GlWindow>,
    gl: Option<Arc<glow::Context>>,
    egui_glow: Option<EguiGlow>,
}

impl App {
    pub fn new(args: Args) -> Self {
        Self {
            args,
            state: AppState::default(),
            gl_window: None,
            gl: None,
            egui_glow: None,
        }
    }

    fn setup(&mut self, event_loop: &ActiveEventLoop) {
        let gl_window = unsafe {
            GlWindow::new(
                event_loop,
                WINDOW_TITLE,
                winit::dpi::LogicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT),
            )
        };
        let gl = unsafe {
            glow::Context::from_loader_function(|s| {
                let cs = std::ffi::CString::new(s).expect("gl proc name");
                gl_window.get_proc_address(&cs)
            })
        };
        let gl = Arc::new(gl);
        let egui_glow = EguiGlow::new(event_loop, Arc::clone(&gl), None, None, true);

        self.gl_window = Some(gl_window);
        self.gl = Some(gl);
        self.egui_glow = Some(egui_glow);
        self.state.format_on = true;
        if let Some(gl_window) = self.gl_window.as_ref() {
            gl_window.window().request_redraw();
        }
    }

    fn render(&mut self) {
        // 1. Raw GL: console background over the whole window, then the
        //    delegated region. Also derive the console height for egui.
        let console_height = {
            let (Some(gl), Some(gl_window)) = (self.gl.as_ref(), self.gl_window.as_ref()) else {
                return;
            };
            let scale = gl_window.scale_factor();
            let layout = layout::compute(gl_window.size(), scale);
            clear(gl, gl_window, CONSOLE_BG);
            let format_color = if self.state.format_on {
                FORMAT_GREEN
            } else {
                CONSOLE_BG
            };
            paint_format(gl, gl_window, layout.format, format_color);
            layout.console.h as f32 / scale as f32
        };

        // 2. egui: the console.
        let station = self.args.station;
        let format_on = self.state.format_on;
        let mut action = ConsoleAction::default();
        {
            let (Some(gl_window), Some(egui_glow)) =
                (self.gl_window.as_ref(), self.egui_glow.as_mut())
            else {
                return;
            };
            egui_glow.run(gl_window.window(), |ui| {
                action = console::draw(ui, station, format_on, console_height);
            });
        }
        if action.toggle_power {
            self.state.format_on = !self.state.format_on;
            if let Some(gl_window) = self.gl_window.as_ref() {
                gl_window.window().request_redraw();
            }
        }

        // 3. Paint egui and present.
        if let (Some(gl_window), Some(egui_glow)) =
            (self.gl_window.as_ref(), self.egui_glow.as_mut())
        {
            egui_glow.paint(gl_window.window());
            gl_window.swap_buffers();
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.gl_window.is_none() {
            self.setup(event_loop);
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        // Let egui see the event first (mouse, keyboard, focus).
        if let (Some(gl_window), Some(egui_glow)) =
            (self.gl_window.as_ref(), self.egui_glow.as_mut())
        {
            let response = egui_glow.on_window_event(gl_window.window(), &event);
            if response.repaint {
                gl_window.window().request_redraw();
            }
        }

        match event {
            WindowEvent::CloseRequested | WindowEvent::Destroyed => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(gl_window) = self.gl_window.as_ref() {
                    let width = core::num::NonZeroU32::new(size.width)
                        .unwrap_or(core::num::NonZeroU32::MIN);
                    let height = core::num::NonZeroU32::new(size.height)
                        .unwrap_or(core::num::NonZeroU32::MIN);
                    gl_window.resize(width, height);
                    gl_window.window().request_redraw();
                }
            }
            WindowEvent::RedrawRequested => self.render(),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {}

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(egui_glow) = self.egui_glow.as_mut() {
            egui_glow.destroy();
        }
    }
}

/// Clear the whole window to `color`.
fn clear(gl: &glow::Context, gl_window: &GlWindow, color: [f32; 4]) {
    let size = gl_window.size();
    unsafe {
        gl.viewport(0, 0, size.width as i32, size.height as i32);
        gl.clear_color(color[0], color[1], color[2], color[3]);
        gl.clear(glow::COLOR_BUFFER_BIT);
    }
}

/// Clear `rect` (top-left origin) to `color` using a GL scissor rect.
fn paint_format(gl: &glow::Context, gl_window: &GlWindow, rect: layout::Rect, color: [f32; 4]) {
    if rect.w <= 0 || rect.h <= 0 {
        return;
    }
    let height = gl_window.size().height as i32;
    let y_gl = height - rect.y - rect.h; // GL origin is bottom-left
    unsafe {
        gl.enable(glow::SCISSOR_TEST);
        gl.scissor(rect.x, y_gl, rect.w, rect.h);
        gl.clear_color(color[0], color[1], color[2], color[3]);
        gl.clear(glow::COLOR_BUFFER_BIT);
        gl.disable(glow::SCISSOR_TEST);
    }
}

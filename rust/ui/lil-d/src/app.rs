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
use crate::display;
use crate::gl_window::GlWindow;
use crate::layout;

const WINDOW_TITLE: &str = "lil-d";
const WINDOW_WIDTH: f64 = 720.0;
const WINDOW_HEIGHT: f64 = 720.0;

/// Background of the console (and of the whole window).
const CONSOLE_BG: [f32; 4] = [0.06, 0.06, 0.07, 1.0];

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
    display: Option<display::Display>,
    /// The last cursor position (physical pixels, top-left origin), needed to
    /// place mouse button events.
    cursor: Option<winit::dpi::PhysicalPosition<f64>>,
}

impl App {
    pub fn new(args: Args) -> Self {
        Self {
            args,
            state: AppState::default(),
            gl_window: None,
            gl: None,
            egui_glow: None,
            display: None,
            cursor: None,
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
        let display = {
            let (loader, userdata) = gl_window.gl_loader();
            let path = display::resolve_path(&self.args.display_lib);
            display::Display::load(&path, loader, userdata).expect("display-lib load")
        };

        self.gl_window = Some(gl_window);
        self.gl = Some(gl);
        self.egui_glow = Some(egui_glow);
        self.display = Some(display);
        self.state.format_on = true;
        // Paint one frame while hidden, then show the window.
        self.render();
        if let Some(gl_window) = self.gl_window.as_ref() {
            gl_window.window().set_visible(true);
            gl_window.window().request_redraw();
        }
    }

    fn render(&mut self) {
        // 1. Raw GL: console background over the whole window, then the
        //    delegated region (rendered by the display library). Also derive
        //    the console height for egui.
        let console_height = {
            let (Some(gl), Some(gl_window), Some(display)) = (
                self.gl.as_ref(),
                self.gl_window.as_ref(),
                self.display.as_mut(),
            ) else {
                return;
            };
            let scale = gl_window.scale_factor();
            let size = gl_window.size();
            let layout = layout::compute(size, scale);
            clear(gl, gl_window, CONSOLE_BG);
            if self.state.format_on {
                display.render(size.height as i32, layout.format);
            }
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

    /// Forward an unconsumed left-button event in the format region to the
    /// display library, then repaint in case the active format changed.
    fn forward_mouse(&mut self, state: winit::event::ElementState, button: winit::event::MouseButton) {
        if !self.state.format_on || button != winit::event::MouseButton::Left {
            return;
        }
        let Some(position) = self.cursor else {
            return;
        };
        {
            let Some(display) = self.display.as_mut() else {
                return;
            };
            let pressed = state == winit::event::ElementState::Pressed;
            display.mouse_button(position.x as i32, position.y as i32, 0, pressed);
        }
        if let Some(gl_window) = self.gl_window.as_ref() {
            gl_window.window().request_redraw();
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
        // Let egui see the event first (mouse, keyboard, focus). `consumed`
        // tells us whether egui used the event (e.g. the console power button).
        let mut consumed = false;
        if let (Some(gl_window), Some(egui_glow)) =
            (self.gl_window.as_ref(), self.egui_glow.as_mut())
        {
            let response = egui_glow.on_window_event(gl_window.window(), &event);
            consumed = response.consumed;
            if response.repaint {
                gl_window.window().request_redraw();
            }
        }

        match event {
            WindowEvent::CloseRequested | WindowEvent::Destroyed => event_loop.exit(),
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = Some(position);
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if !consumed {
                    self.forward_mouse(state, button);
                }
            }
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
        gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);
    }
}

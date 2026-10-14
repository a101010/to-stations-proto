//! Window geometry: the format region and the console strip.

/// Console strip height in logical pixels.
pub const CONSOLE_HEIGHT_LOGICAL: f64 = 64.0;

/// A rectangle in physical pixels, origin top-left.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

/// The window split into the region delegated to the display library and the
/// console.
#[derive(Clone, Copy, Debug)]
pub struct Layout {
    pub format: Rect,
    pub console: Rect,
}

/// Compute the layout for a window of `size` (physical pixels) at `scale`.
pub fn compute(size: winit::dpi::PhysicalSize<u32>, scale: f64) -> Layout {
    let w = size.width as i32;
    let h = size.height as i32;

    let console_h = ((CONSOLE_HEIGHT_LOGICAL * scale).round() as i32).clamp(0, h);
    let console = Rect {
        x: 0,
        y: h - console_h,
        w,
        h: console_h,
    };

    let avail_h = h - console_h;
    let side = w.min(avail_h);
    let format = Rect {
        x: (w - side) / 2,
        y: (avail_h - side) / 2,
        w: side,
        h: side,
    };

    Layout { format, console }
}

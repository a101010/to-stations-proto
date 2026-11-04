//! A Windows named-mutex guard for manually launched singleton GUI tools.
//!
//! The primary instance holds a named mutex for its lifetime. A second launch
//! receives [`Acquire::Duplicate`], can call [`activate_existing_window`] to
//! restore and foreground the primary window, and then exits.
//!
//! This crate is intentionally not used by fleet-launched applications or
//! per-station/per-display windows, where multiple instances are legitimate.

/// The result of trying to acquire the single-instance mutex.
pub enum Acquire {
    /// This process is the primary instance; keep the guard alive.
    Primary(SingleInstance),
    /// Another instance owns the mutex.
    Duplicate,
}

/// The RAII guard held by the primary instance.
pub struct SingleInstance {
    #[cfg(windows)]
    handle: Option<windows_sys::Win32::Foundation::HANDLE>,
}

impl Drop for SingleInstance {
    fn drop(&mut self) {
        #[cfg(windows)]
        if let Some(handle) = self.handle.take() {
            if !handle.is_null() {
                unsafe {
                    windows_sys::Win32::Foundation::CloseHandle(handle);
                }
            }
        }
    }
}

/// Try to acquire the single-instance mutex for `tool_name`.
///
/// The mutex name is `Local\to-stations-<tool_name>`, so it is scoped to the
/// current Windows session.
#[cfg(windows)]
pub fn acquire(tool_name: &str) -> Acquire {
    use std::ptr;
    use windows_sys::Win32::Foundation::{GetLastError, SetLastError, ERROR_ALREADY_EXISTS};
    use windows_sys::Win32::System::Threading::CreateMutexW;

    let name = wide(&format!(r"Local\to-stations-{tool_name}"));
    unsafe { SetLastError(0) };
    let handle = unsafe { CreateMutexW(ptr::null(), 0, name.as_ptr()) };
    if handle.is_null() {
        let error = unsafe { GetLastError() };
        eprintln!("single-instance: CreateMutexW failed with error {error}");
        return Acquire::Primary(SingleInstance { handle: None });
    }

    if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(handle);
        }
        return Acquire::Duplicate;
    }

    Acquire::Primary(SingleInstance {
        handle: Some(handle),
    })
}

/// Try to acquire the single-instance mutex for `tool_name`.
#[cfg(not(windows))]
pub fn acquire(_tool_name: &str) -> Acquire {
    Acquire::Primary(SingleInstance {})
}

/// Restore and foreground the window titled `window_title`.
///
/// This is best-effort: Windows can refuse foreground activation, and the
/// primary window may not exist yet. The caller can exit regardless.
#[cfg(windows)]
pub fn activate_existing_window(window_title: &str) {
    use std::ptr;
    use std::time::Duration;
    use windows_sys::Win32::Foundation::{FALSE, TRUE};
    use windows_sys::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::SetFocus;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        BringWindowToTop, FindWindowW, GetForegroundWindow, GetWindowThreadProcessId, IsIconic,
        PeekMessageW, SetForegroundWindow, SetWindowPos, ShowWindow, SwitchToThisWindow, HWND_TOP,
        MSG, PM_NOREMOVE, SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW, SW_RESTORE, SW_SHOW,
    };

    let title = wide(window_title);
    for _ in 0..40 {
        let hwnd = unsafe { FindWindowW(ptr::null(), title.as_ptr()) };
        if !hwnd.is_null() {
            unsafe {
                if IsIconic(hwnd) != 0 {
                    ShowWindow(hwnd, SW_RESTORE);
                } else {
                    ShowWindow(hwnd, SW_SHOW);
                }
                SetWindowPos(
                    hwnd,
                    HWND_TOP,
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW,
                );

                // AttachThreadInput requires this thread to have a message
                // queue; PeekMessage creates one if needed.
                let mut message: MSG = std::mem::zeroed();
                PeekMessageW(&mut message, ptr::null_mut(), 0, 0, PM_NOREMOVE);

                let mut foreground_thread = 0;
                let foreground = GetForegroundWindow();
                if !foreground.is_null() {
                    foreground_thread = GetWindowThreadProcessId(foreground, ptr::null_mut());
                }
                let this_thread = GetCurrentThreadId();
                let attached = foreground_thread != 0
                    && AttachThreadInput(this_thread, foreground_thread, TRUE) != 0;

                if SetForegroundWindow(hwnd) == 0 {
                    SwitchToThisWindow(hwnd, TRUE);
                }
                SetFocus(hwnd);
                BringWindowToTop(hwnd);

                if attached {
                    AttachThreadInput(this_thread, foreground_thread, FALSE);
                }
            }
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// Restore and foreground the window titled `window_title`.
#[cfg(not(windows))]
pub fn activate_existing_window(_window_title: &str) {}

#[cfg(windows)]
fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn detects_a_second_acquire_and_releases_on_drop() {
        let tool = format!("test-{}", std::process::id());
        let first = match acquire(&tool) {
            Acquire::Primary(guard) => guard,
            Acquire::Duplicate => panic!("first acquire should be primary"),
        };

        match acquire(&tool) {
            Acquire::Duplicate => {}
            Acquire::Primary(_) => panic!("second acquire should be a duplicate"),
        }

        drop(first);
        match acquire(&tool) {
            Acquire::Primary(_) => {}
            Acquire::Duplicate => panic!("acquire after drop should be primary"),
        }
    }
}

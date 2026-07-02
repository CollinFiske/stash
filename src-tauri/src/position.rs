//! Cross-platform window positioning.
//! After a process is launched we poll x-win until the window appears
//! then apply the saved bounding box using native APIs.

use std::time::Duration;

const POLL_INTERVAL_MS: u64 = 400;
const POLL_ATTEMPTS: usize = 15;

pub fn apply_position(exe_path: &str, x: i32, y: i32, w: i32, h: i32) {
    let exe_lower = exe_path.to_lowercase();
    for _ in 0..POLL_ATTEMPTS {
        std::thread::sleep(Duration::from_millis(POLL_INTERVAL_MS));
        if let Ok(windows) = x_win::get_open_windows() {
            for win in &windows {
                if win.info.path.to_lowercase() == exe_lower {
                    let _ = native_set_position(win.id, x, y, w.max(50), h.max(50));
                    return;
                }
            }
        }
    }
    // Window did not appear within timeout; no-op.
}

// ── Platform implementations ──────────────────────────────────────────────────

#[cfg(target_os = "windows")]
fn native_set_position(window_id: u32, x: i32, y: i32, w: i32, h: i32) -> Result<(), String> {
    const SWP_NOZORDER: u32 = 0x0004;
    const SWP_SHOWWINDOW: u32 = 0x0040;

    #[link(name = "user32")]
    extern "system" {
        fn SetWindowPos(
            h_wnd: isize,
            h_wnd_insert_after: isize,
            x: i32,
            y: i32,
            cx: i32,
            cy: i32,
            u_flags: u32,
        ) -> i32;
    }

    let result = unsafe {
        SetWindowPos(
            window_id as isize,
            0,
            x,
            y,
            w,
            h,
            SWP_NOZORDER | SWP_SHOWWINDOW,
        )
    };
    if result == 0 {
        Err(format!("SetWindowPos failed for hwnd {window_id}"))
    } else {
        Ok(())
    }
}

#[cfg(target_os = "macos")]
fn native_set_position(window_id: u32, x: i32, y: i32, w: i32, h: i32) -> Result<(), String> {
    // macOS bounds uses {left, top, right, bottom}.
    let right = x + w;
    let bottom = y + h;
    let script = format!(
        "tell application \"System Events\" to set bounds of (first window of (first process whose unix id is {window_id})) to {{{x}, {y}, {right}, {bottom}}}"
    );
    std::process::Command::new("osascript")
        .args(["-e", &script])
        .output()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[cfg(target_os = "linux")]
fn native_set_position(window_id: u32, x: i32, y: i32, w: i32, h: i32) -> Result<(), String> {
    // xdotool uses X11 window IDs (same as what x-win exposes on X11).
    let wid = window_id.to_string();
    std::process::Command::new("xdotool")
        .args(["windowmove", &wid, &x.to_string(), &y.to_string()])
        .output()
        .map_err(|e| e.to_string())?;
    std::process::Command::new("xdotool")
        .args(["windowsize", &wid, &w.to_string(), &h.to_string()])
        .output()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

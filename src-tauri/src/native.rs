//! Cross-platform facade over native window and process control.
//!
//! Windows work goes through the hand-written bindings in [`crate::win32`];
//! macOS uses AppleScript via `osascript`; Linux uses `xdotool` / `kill`.
//! Everything above this module is platform-agnostic.

/// A native window handle. On Windows this is an `HWND`; on Linux an X11 window
/// id; on macOS x-win reports a process-scoped id.
pub type WindowId = isize;

// ── Positioning ───────────────────────────────────────────────────────────────

#[cfg(target_os = "windows")]
pub fn set_window_position(id: WindowId, x: i32, y: i32, w: i32, h: i32) -> Result<(), String> {
    crate::win32::set_position(id, x, y, w.max(50), h.max(50))
}

#[cfg(target_os = "macos")]
pub fn set_window_position(id: WindowId, x: i32, y: i32, w: i32, h: i32) -> Result<(), String> {
    let (w, h) = (w.max(50), h.max(50));
    let (right, bottom) = (x + w, y + h);
    let script = format!(
        "tell application \"System Events\" to set bounds of (first window of (first process whose unix id is {id})) to {{{x}, {y}, {right}, {bottom}}}"
    );
    run_quiet("osascript", &["-e", &script])
}

#[cfg(target_os = "linux")]
pub fn set_window_position(id: WindowId, x: i32, y: i32, w: i32, h: i32) -> Result<(), String> {
    let wid = id.to_string();
    run_quiet(
        "xdotool",
        &["windowmove", &wid, &x.to_string(), &y.to_string()],
    )?;
    run_quiet(
        "xdotool",
        &[
            "windowsize",
            &wid,
            &w.max(50).to_string(),
            &h.max(50).to_string(),
        ],
    )
}

// ── Graceful close ────────────────────────────────────────────────────────────

/// Ask a single window to close, exactly as if the user clicked its ✕.
/// The owning application is free to prompt about unsaved work.
#[cfg(target_os = "windows")]
pub fn close_window(id: WindowId) -> bool {
    crate::win32::close_window(id)
}

#[cfg(not(target_os = "windows"))]
pub fn close_window(_id: WindowId) -> bool {
    false
}

/// Ask every window owned by `pid` to close gracefully.
/// Returns the number of close requests that were delivered.
#[cfg(target_os = "windows")]
pub fn close_process_gracefully(pid: u32, _exe_name: &str) -> usize {
    crate::win32::close_pid_windows(pid)
}

#[cfg(target_os = "macos")]
pub fn close_process_gracefully(_pid: u32, exe_name: &str) -> usize {
    // `quit` is AppleScript's graceful terminate — the app runs its normal
    // shutdown path and may present a save dialog.
    let app = exe_name.trim_end_matches(".app");
    let script = format!("tell application \"{app}\" to quit");
    if run_quiet("osascript", &["-e", &script]).is_ok() {
        1
    } else {
        0
    }
}

#[cfg(target_os = "linux")]
pub fn close_process_gracefully(pid: u32, _exe_name: &str) -> usize {
    // SIGTERM is the POSIX equivalent of "please shut down cleanly".
    if run_quiet("kill", &["-TERM", &pid.to_string()]).is_ok() {
        1
    } else {
        0
    }
}

/// Last-resort forced termination of a single process by PID.
/// Only used when the caller explicitly opts into force-closing.
pub fn force_kill_pid(pid: u32) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        run_quiet("taskkill", &["/F", "/PID", &pid.to_string()])
    }
    #[cfg(not(target_os = "windows"))]
    {
        run_quiet("kill", &["-KILL", &pid.to_string()])
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Run a helper binary without flashing a console window on Windows.
pub fn run_quiet(program: &str, args: &[&str]) -> Result<(), String> {
    let mut cmd = std::process::Command::new(program);
    cmd.args(args);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(crate::restore::CREATE_NO_WINDOW);
    }
    cmd.output().map(|_| ()).map_err(|e| e.to_string())
}

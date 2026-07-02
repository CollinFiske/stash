use tauri::WebviewWindow;

use crate::capture::{self, MonitorInfo};
use crate::models::{CapturedApp, RestoreSummary};
use crate::restore;

/// Build monitor info list from available monitors.
fn get_monitors(window: &WebviewWindow) -> Vec<MonitorInfo> {
    window
        .available_monitors()
        .unwrap_or_default()
        .into_iter()
        .map(|m| MonitorInfo {
            x: m.position().x,
            y: m.position().y,
            width: m.size().width,
            height: m.size().height,
        })
        .collect()
}

/// Snapshot all visible user-facing windows, with multi-monitor support.
#[tauri::command]
pub async fn capture_snapshot(window: WebviewWindow) -> Result<Vec<CapturedApp>, String> {
    let (width, height) = primary_monitor_size(&window)?;
    let monitors = get_monitors(&window);
    capture::capture_windows(width, height, &monitors)
}

/// Launch every app in the supplied list and attempt to restore window
/// positions on the correct monitor.
#[tauri::command]
pub async fn restore_workspace(
    window: WebviewWindow,
    apps: Vec<CapturedApp>,
) -> Result<RestoreSummary, String> {
    let (width, height) = primary_monitor_size(&window)?;
    let monitors = get_monitors(&window);
    Ok(restore::restore_workspace(apps, monitors, width, height))
}

/// Expose current primary monitor dimensions to the frontend.
#[tauri::command]
pub async fn get_screen_size(window: WebviewWindow) -> Result<(u32, u32), String> {
    primary_monitor_size(&window)
}

/// Close a list of apps by their executable paths.
#[tauri::command]
pub async fn close_apps(exe_paths: Vec<String>) -> Result<(), String> {
    for path in &exe_paths {
        let name = exe_name(path);
        let _ = close_single(&name);
    }
    Ok(())
}

/// Write arbitrary text content to a file path (used for workspace export).
#[tauri::command]
pub async fn write_file(path: String, content: String) -> Result<(), String> {
    std::fs::write(&path, content).map_err(|e| e.to_string())
}

/// Read text content from a file path (used for workspace import).
#[tauri::command]
pub async fn read_file(path: String) -> Result<String, String> {
    std::fs::read_to_string(&path).map_err(|e| e.to_string())
}

/// Show and focus the main window (called from tray / global shortcut).
#[tauri::command]
pub async fn show_window(window: WebviewWindow) -> Result<(), String> {
    window.show().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())
}

fn exe_name(path: &str) -> String {
    let normalized = path.replace('\\', "/");
    let base = normalized.split('/').next_back().unwrap_or(path);
    base.to_string()
}

#[cfg(target_os = "windows")]
fn close_single(exe: &str) -> std::io::Result<()> {
    use std::os::windows::process::CommandExt;
    std::process::Command::new("taskkill")
        .args(["/F", "/IM", exe])
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .output()
        .map(|_| ())
}

#[cfg(target_os = "macos")]
fn close_single(exe: &str) -> std::io::Result<()> {
    let app_name = exe.trim_end_matches(".app");
    let script = format!("tell application \"{}\" to quit", app_name);
    std::process::Command::new("osascript")
        .args(["-e", &script])
        .output()
        .map(|_| ())
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn close_single(exe: &str) -> std::io::Result<()> {
    let name = exe.trim_end_matches(".exe");
    std::process::Command::new("pkill")
        .arg("-f")
        .arg(name)
        .output()
        .map(|_| ())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn primary_monitor_size(window: &WebviewWindow) -> Result<(u32, u32), String> {
    let monitor = window
        .primary_monitor()
        .map_err(|e| format!("Monitor query failed: {e}"))?
        .ok_or_else(|| "No primary monitor detected".to_string())?;

    let size = monitor.size();
    Ok((size.width, size.height))
}

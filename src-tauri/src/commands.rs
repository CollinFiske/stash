use tauri::{State, WebviewWindow};

use crate::browser::BrowserSnapshot;
use crate::bridge::{Bridge, BridgeCommand, BridgeStatus};
use crate::capture::{self, MonitorInfo};
use crate::models::{CapturedApp, CloseSummary, RestoreSummary};
use crate::procinfo;
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
///
/// When the Stash browser extension is connected, each browser window is paired
/// with its tab and tab-group state so the snapshot captures what was open, not
/// just that a browser was running.
#[tauri::command]
pub async fn capture_snapshot(
    window: WebviewWindow,
    bridge: State<'_, Bridge>,
) -> Result<Vec<CapturedApp>, String> {
    let (width, height) = primary_monitor_size(&window)?;
    let monitors = get_monitors(&window);
    let browser_state = bridge.snapshot();
    capture::capture_windows(width, height, &monitors, browser_state.as_ref())
}

/// Launch every app in the supplied list and place each one's window.
#[tauri::command]
pub async fn restore_workspace(
    window: WebviewWindow,
    apps: Vec<CapturedApp>,
    bridge: State<'_, Bridge>,
) -> Result<RestoreSummary, String> {
    let (width, height) = primary_monitor_size(&window)?;
    let monitors = get_monitors(&window);
    Ok(restore::restore_workspace(
        apps,
        monitors,
        width,
        height,
        Some(bridge.inner().clone()),
    ))
}

/// Expose current primary monitor dimensions to the frontend.
#[tauri::command]
pub async fn get_screen_size(window: WebviewWindow) -> Result<(u32, u32), String> {
    primary_monitor_size(&window)
}

/// Close every application in a workspace.
///
/// Windows are asked to close the same way clicking ✕ would, so applications can
/// prompt about unsaved work. This replaced a blanket `taskkill /F /IM <exe>`,
/// which force-killed *every* process sharing an image name — closing one
/// workspace containing Chrome took down all the user's other Chrome windows and
/// discarded unsaved work without asking.
///
/// Set `force` to escalate to termination for anything still running afterwards.
#[tauri::command]
pub async fn close_workspace(
    apps: Vec<CapturedApp>,
    force: bool,
    bridge: State<'_, Bridge>,
) -> Result<CloseSummary, String> {
    let mut summary = CloseSummary::default();

    // ── Browser windows close through the extension, tab state intact ────────
    let mut browser_window_ids: Vec<i64> = Vec::new();
    if bridge.connected() {
        for app in &apps {
            if let Some(state) = &app.browser_window {
                browser_window_ids.push(state.window_id);
            }
        }
    }
    if !browser_window_ids.is_empty() {
        summary.browser_windows_closed = browser_window_ids.len();
        summary.details.push(format!(
            "{} browser window(s) closed via extension",
            browser_window_ids.len()
        ));
        bridge.enqueue(BridgeCommand::Close {
            id: format!("close-{}", std::process::id()),
            window_ids: browser_window_ids,
        });
    }

    // ── Everything else closes through the OS ────────────────────────────────
    let live = live_windows();
    let mut targeted_pids: Vec<u32> = Vec::new();

    for app in &apps {
        // A browser handled by the extension needs no OS-level close.
        if app.browser_window.is_some() && bridge.connected() {
            continue;
        }

        let target = app.exe_path.to_lowercase();
        let matches: Vec<(isize, u32)> = live
            .iter()
            .filter(|(_, _, path)| path.to_lowercase() == target)
            .map(|(id, pid, _)| (*id, *pid))
            .collect();

        if matches.is_empty() {
            summary.skipped += 1;
            continue;
        }

        for (window_id, pid) in matches {
            if crate::native::close_window(window_id) {
                summary.closed += 1;
            } else {
                // Platforms without per-window close fall back to per-process.
                summary.closed += crate::native::close_process_gracefully(
                    pid,
                    &procinfo::exe_stem(&app.exe_path),
                );
            }
            if !targeted_pids.contains(&pid) {
                targeted_pids.push(pid);
            }
        }
    }

    // ── Optional escalation ──────────────────────────────────────────────────
    if force && !targeted_pids.is_empty() {
        // Give each app a moment to run its own shutdown path first.
        std::thread::sleep(std::time::Duration::from_millis(2500));
        let still_running = live_windows();
        for pid in targeted_pids {
            if still_running.iter().any(|(_, p, _)| *p == pid)
                && crate::native::force_kill_pid(pid).is_ok()
            {
                summary.force_killed += 1;
            }
        }
        if summary.force_killed > 0 {
            summary
                .details
                .push(format!("{} process(es) force-closed", summary.force_killed));
        }
    }

    Ok(summary)
}

/// Pairing details and live connection state for the browser extension.
#[tauri::command]
pub async fn bridge_status(bridge: State<'_, Bridge>) -> Result<BridgeStatus, String> {
    Ok(bridge.status())
}

/// The extension's latest view of the browser: every window, tab and tab group.
#[tauri::command]
pub async fn browser_state(bridge: State<'_, Bridge>) -> Result<Option<BrowserSnapshot>, String> {
    Ok(bridge.snapshot())
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

// ── Helpers ───────────────────────────────────────────────────────────────────

/// `(window_id, pid, exe_path)` for every window currently on the desktop.
fn live_windows() -> Vec<(isize, u32, String)> {
    x_win::get_open_windows()
        .map(|windows| {
            windows
                .into_iter()
                .map(|w| (w.id as isize, w.info.process_id, w.info.path))
                .collect()
        })
        .unwrap_or_default()
}

fn primary_monitor_size(window: &WebviewWindow) -> Result<(u32, u32), String> {
    let monitor = window
        .primary_monitor()
        .map_err(|e| format!("Monitor query failed: {e}"))?
        .ok_or_else(|| "No primary monitor detected".to_string())?;

    let size = monitor.size();
    Ok((size.width, size.height))
}

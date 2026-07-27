use crate::browser::{self, BrowserSnapshot};
use crate::models::{CapturedApp, RelativePosition};
use crate::procinfo;
use std::collections::HashSet;

// Identifiers checked against the lowercase exe name or path.
const VSCODE_IDS: &[&str] = &["code.exe", "code", "code - insiders.exe", "code - insiders"];
const BROWSER_IDS: &[&str] = &[
    "chrome.exe",
    "chrome",
    "msedge.exe",
    "msedge",
    "brave.exe",
    "brave",
    "brave browser",
    "firefox.exe",
    "firefox",
    "safari",
    "arc",
];

// System shell noise that should never appear in a saved workspace.
const SYSTEM_TITLES: &[&str] = &[
    "program manager",
    "desktop window manager",
    "task switching",
    "windows security",
    "microsoft text input application",
    "cortana",
    "search",
    "taskbar",
    "windows input experience",
    "notification centre",
    "action center",
    "start",
];

/// Monitor geometry used to assign monitor indices and compute relative positions.
#[derive(Debug, Clone)]
pub struct MonitorInfo {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// Determine which monitor a window at (wx, wy) belongs to.
/// Returns the index into `monitors`, or 0 as a fallback.
fn monitor_for_pos(monitors: &[MonitorInfo], wx: i32, wy: i32) -> u32 {
    for (i, m) in monitors.iter().enumerate() {
        if wx >= m.x && wx < m.x + m.width as i32 && wy >= m.y && wy < m.y + m.height as i32 {
            return i as u32;
        }
    }
    0
}

/// Exclude the Stash application itself from snapshots.
/// Matches both release (`stash.exe`) and Tauri dev-mode binaries.
pub fn is_self_app(name: &str, path: &str) -> bool {
    let n = name.to_lowercase();
    let p = path.to_lowercase();
    n == "stash"
        || p.ends_with("\\stash.exe")
        || p.ends_with("/stash")
        || p.contains("\\target\\debug\\stash")
        || p.contains("/target/debug/stash")
        || p.contains("\\target\\release\\stash")
        || p.contains("/target/release/stash")
}

pub fn is_vscode(name: &str, path: &str) -> bool {
    let n = name.to_lowercase();
    let p = path.to_lowercase();
    VSCODE_IDS.iter().any(|id| n == *id)
        || p.contains("\\code.exe")
        || p.contains("/code")
        || p.contains("visual studio code")
        || p.contains("vscode")
}

pub fn is_browser(name: &str, path: &str) -> bool {
    let n = name.to_lowercase();
    let p = path.to_lowercase();
    BROWSER_IDS.iter().any(|id| n.contains(id))
        || p.contains("chrome.exe")
        || p.contains("msedge.exe")
        || p.contains("brave.exe")
        || p.contains("firefox.exe")
}

fn is_system_noise(title: &str) -> bool {
    let t = title.trim().to_lowercase();
    if t.is_empty() {
        return true;
    }
    SYSTEM_TITLES.iter().any(|s| t == *s)
}

/// Parse VS Code's title bar to extract the workspace folder name.
/// VS Code format: "filename — Workspace Name — Visual Studio Code"
/// or:             "filename - Workspace Name - Visual Studio Code"
pub fn extract_vscode_workspace(title: &str) -> Option<String> {
    // Normalise em-dash to ASCII hyphen for consistent splitting.
    let normalised = title.replace(" \u{2014} ", " - ");
    let parts: Vec<&str> = normalised.split(" - ").collect();
    if parts.len() >= 3 {
        // Second-to-last segment is the workspace folder.
        let candidate = parts[parts.len() - 2].trim();
        if !candidate.is_empty()
            && !candidate.eq_ignore_ascii_case("visual studio code")
            && !candidate.eq_ignore_ascii_case("vscode")
        {
            return Some(candidate.to_string());
        }
    }
    // Single-segment title: the whole string may be the folder name.
    if parts.len() == 2 {
        let candidate = parts[0].trim();
        if !candidate.is_empty() {
            return Some(candidate.to_string());
        }
    }
    None
}

/// Capture all visible user-facing windows and return their data with
/// positions expressed as fractions of the monitor they reside on.
///
/// `monitors` provides geometry for all available monitors (primary first).
/// `browser_state` is the Stash extension's view of the browser, when connected;
/// each browser window found on screen is matched to it by geometry so the tab
/// list travels with the snapshot.
pub fn capture_windows(
    screen_width: u32,
    screen_height: u32,
    monitors: &[MonitorInfo],
    browser_state: Option<&BrowserSnapshot>,
) -> Result<Vec<CapturedApp>, String> {
    let windows = x_win::get_open_windows().map_err(|e| format!("x-win capture failed: {e}"))?;

    let mut captured: Vec<CapturedApp> = Vec::new();
    // Deduplicate same app+title pairs (e.g. multiple shell chrome processes
    // that map to the same visible window).
    let mut seen: HashSet<String> = HashSet::new();
    // Browser windows already paired with an on-screen window, so two Chrome
    // windows never claim the same tab list.
    let mut claimed_browser_windows: Vec<usize> = Vec::new();

    for win in windows {
        let title = win.title.trim().to_string();
        if is_system_noise(&title) {
            continue;
        }

        let name = win.info.name.clone();
        let exe_path = win.info.path.clone();
        let pid = win.info.process_id;

        // Skip Stash itself — users don't need the manager in their snapshots.
        if is_self_app(&name, &exe_path) {
            continue;
        }

        let dedup_key = format!("{}::{}", exe_path, title);
        if seen.contains(&dedup_key) {
            continue;
        }
        seen.insert(dedup_key);

        let vscode = is_vscode(&name, &exe_path);
        let browser_app = is_browser(&name, &exe_path);
        let terminal = !vscode && !browser_app && procinfo::is_terminal(&name, &exe_path);

        let context_path = if vscode {
            extract_vscode_workspace(&title)
                .and_then(|name| crate::vscode_state::resolve_workspace_path(&name, &exe_path))
        } else {
            None
        };

        // Terminals remember the directory their shell was working in, so they
        // reopen where the user left off instead of at the shell's default.
        let working_dir = if terminal {
            procinfo::terminal_cwd(pid)
        } else {
            None
        };

        let p = &win.position;
        let wx = p.x;
        let wy = p.y;

        // Pair this window with the extension's view of the same browser window.
        let browser_window = if browser_app {
            browser_state.and_then(|state| {
                let idx = browser::correlate_window(
                    &state.windows,
                    &claimed_browser_windows,
                    wx,
                    wy,
                    p.width,
                    p.height,
                )?;
                claimed_browser_windows.push(idx);
                Some(state.windows[idx].clone())
            })
        } else {
            None
        };

        // When the extension supplied tabs, keep the active tab's URL as the
        // fallback for restoring without the extension installed.
        let target_url: Option<String> = browser_window.as_ref().and_then(|bw| {
            bw.tabs
                .iter()
                .find(|t| t.active)
                .or_else(|| bw.tabs.first())
                .map(|t| t.url.clone())
        });

        // Determine which monitor the window lives on.
        let monitor_index = if monitors.is_empty() {
            0
        } else {
            monitor_for_pos(monitors, wx, wy)
        };

        // Compute position relative to the monitor it belongs to.
        let relative_position = if !monitors.is_empty() {
            let idx = monitor_index as usize;
            let m = if idx < monitors.len() {
                &monitors[idx]
            } else {
                &monitors[0]
            };
            let mw = m.width.max(1) as f64;
            let mh = m.height.max(1) as f64;
            RelativePosition {
                x: ((wx - m.x) as f64 / mw).clamp(0.0, 1.0),
                y: ((wy - m.y) as f64 / mh).clamp(0.0, 1.0),
                w: (p.width as f64 / mw).clamp(0.0, 1.0),
                h: (p.height as f64 / mh).clamp(0.0, 1.0),
            }
        } else {
            // Fallback: primary monitor only
            let sw = screen_width.max(1) as f64;
            let sh = screen_height.max(1) as f64;
            RelativePosition {
                x: (wx as f64 / sw).clamp(0.0, 1.0),
                y: (wy as f64 / sh).clamp(0.0, 1.0),
                w: (p.width as f64 / sw).clamp(0.0, 1.0),
                h: (p.height as f64 / sh).clamp(0.0, 1.0),
            }
        };

        captured.push(CapturedApp {
            name,
            exe_path,
            title,
            is_vscode: vscode,
            is_browser: browser_app,
            is_terminal: terminal,
            context_path,
            target_url,
            working_dir,
            relative_position,
            monitor_index,
            browser_window,
        });
    }

    Ok(captured)
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_position_arithmetic() {
        // A 1920×1080 window filling the left half of the screen.
        let sw = 1920u32;
        let sh = 1080u32;
        let (wx, wy, ww, wh) = (0i32, 0i32, 960u32, 1080u32);

        let rx = wx as f64 / sw as f64;
        let ry = wy as f64 / sh as f64;
        let rw = ww as f64 / sw as f64;
        let rh = wh as f64 / sh as f64;

        assert!((rx - 0.0).abs() < f64::EPSILON);
        assert!((ry - 0.0).abs() < f64::EPSILON);
        assert!((rw - 0.5).abs() < f64::EPSILON);
        assert!((rh - 1.0).abs() < f64::EPSILON);

        // Restoring on a 2560×1440 display.
        let new_sw = 2560u32;
        let new_sh = 1440u32;
        let abs_x = (rx * new_sw as f64) as i32;
        let abs_w = (rw * new_sw as f64) as u32;
        assert_eq!(abs_x, 0);
        assert_eq!(abs_w, 1280);
        let _ = new_sh; // used implicitly
    }

    #[test]
    fn vscode_workspace_extraction() {
        let title = "main.rs - my-project - Visual Studio Code";
        assert_eq!(
            extract_vscode_workspace(title),
            Some("my-project".to_string())
        );

        let title2 = "App.vue \u{2014} stash-frontend \u{2014} Visual Studio Code";
        assert_eq!(
            extract_vscode_workspace(title2),
            Some("stash-frontend".to_string())
        );

        // No workspace: just a single file open
        let title3 = "Untitled-1 - Visual Studio Code";
        // Only 2 parts: first segment is used as context
        assert!(extract_vscode_workspace(title3).is_some());
    }

    #[test]
    fn system_noise_filtered() {
        assert!(is_system_noise(""));
        assert!(is_system_noise("Program Manager"));
        assert!(is_system_noise("  taskbar  "));
        assert!(!is_system_noise("Slack"));
        assert!(!is_system_noise("Visual Studio Code"));
    }

    #[test]
    fn vscode_detection() {
        assert!(is_vscode(
            "Code",
            "C:\\Users\\user\\AppData\\Local\\Programs\\Microsoft VS Code\\Code.exe"
        ));
        assert!(is_vscode("code", "/usr/share/code/code"));
        assert!(!is_vscode(
            "chrome",
            "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe"
        ));
    }

    #[test]
    fn browser_detection() {
        assert!(is_browser(
            "chrome",
            "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe"
        ));
        assert!(is_browser(
            "msedge",
            "C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe"
        ));
        assert!(is_browser("firefox", "/usr/bin/firefox"));
        assert!(!is_browser("Code", "C:\\Code.exe"));
    }

    #[test]
    fn monitor_detection() {
        let monitors = vec![
            MonitorInfo {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            },
            MonitorInfo {
                x: 1920,
                y: 0,
                width: 2560,
                height: 1440,
            },
        ];
        assert_eq!(monitor_for_pos(&monitors, 0, 0), 0);
        assert_eq!(monitor_for_pos(&monitors, 960, 540), 0);
        assert_eq!(monitor_for_pos(&monitors, 1920, 0), 1);
        assert_eq!(monitor_for_pos(&monitors, 2500, 700), 1);
        // out of bounds falls back to 0
        assert_eq!(monitor_for_pos(&monitors, -100, 0), 0);
    }

    #[test]
    fn terminals_are_classified_separately_from_editors() {
        // A terminal must not be mistaken for VS Code or a browser.
        assert!(procinfo::is_terminal(
            "WindowsTerminal",
            "C:\\Program Files\\WindowsTerminal.exe"
        ));
        assert!(!is_vscode(
            "WindowsTerminal",
            "C:\\Program Files\\WindowsTerminal.exe"
        ));
        assert!(!is_browser(
            "WindowsTerminal",
            "C:\\Program Files\\WindowsTerminal.exe"
        ));
    }
}

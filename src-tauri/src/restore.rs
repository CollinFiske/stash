use crate::browser::BrowserWindowState;
use crate::bridge::{Bridge, BridgeCommand};
use crate::capture::MonitorInfo;
use crate::models::{CapturedApp, RestoreResult, RestoreSummary};
use crate::position::WindowClaimer;
use crate::procinfo;
use std::time::Duration;

/// Suppress the console window Windows would otherwise create for a child process.
#[cfg(target_os = "windows")]
pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;
/// Give a terminal its own console. Mutually exclusive with `CREATE_NO_WINDOW`.
#[cfg(target_os = "windows")]
pub const CREATE_NEW_CONSOLE: u32 = 0x0000_0010;

/// Stagger between launches, so a workspace of ten apps does not thrash the disk.
const LAUNCH_STAGGER: Duration = Duration::from_millis(350);
/// How long to wait for each app's window to appear before giving up on placing it.
const WINDOW_WAIT: Duration = Duration::from_secs(12);

/// Launch every app in the saved group, then place each one's window.
///
/// Window placement is the part that used to go wrong: matching windows by
/// executable path alone meant two VS Code windows both resolved to whichever
/// window happened to be enumerated first, so one got positioned twice and the
/// other not at all. [`WindowClaimer`] fixes that by tracking which windows
/// existed *before* the restore and letting each app claim exactly one of the
/// windows that appeared *after* it.
pub fn restore_workspace(
    apps: Vec<CapturedApp>,
    monitors: Vec<MonitorInfo>,
    screen_width: u32,
    screen_height: u32,
    bridge: Option<Bridge>,
) -> RestoreSummary {
    let total = apps.len();
    let mut results: Vec<RestoreResult> = Vec::with_capacity(total);

    // ── Browser windows go to the extension, not to a process launch ─────────
    let extension_ready = bridge.as_ref().map(|b| b.connected()).unwrap_or(false);
    let mut browser_windows: Vec<BrowserWindowState> = Vec::new();
    let mut launchable: Vec<CapturedApp> = Vec::new();

    for app in apps {
        match (&app.browser_window, extension_ready) {
            // The extension can rebuild the window with every tab and group
            // intact, which no amount of command-line launching can do.
            (Some(state), true) => {
                browser_windows.push(state.clone());
                results.push(RestoreResult {
                    app_name: format!("{} ({} tabs)", app.name, state.tabs.len()),
                    success: true,
                    error: None,
                    positioned: true,
                });
            }
            // No extension: fall back to launching the browser on one URL.
            _ => launchable.push(app),
        }
    }

    let browser_windows_restored = browser_windows.len();
    if let (Some(bridge), false) = (bridge.as_ref(), browser_windows.is_empty()) {
        bridge.enqueue(BridgeCommand::Restore {
            id: format!("restore-{}", now_millis()),
            windows: browser_windows,
        });
    }

    // ── Record which windows already existed, then launch ────────────────────
    let mut claimer = WindowClaimer::new();

    let mut launched: Vec<(CapturedApp, Option<u32>)> = Vec::with_capacity(launchable.len());
    for app in launchable {
        std::thread::sleep(LAUNCH_STAGGER);
        match platform_launch(&app) {
            Ok(pid) => {
                results.push(RestoreResult {
                    app_name: app.name.clone(),
                    success: true,
                    error: None,
                    positioned: false,
                });
                launched.push((app, pid));
            }
            Err(msg) => {
                results.push(RestoreResult {
                    app_name: app.name.clone(),
                    success: false,
                    error: Some(msg),
                    positioned: false,
                });
            }
        }
    }

    // ── Place windows on a background thread so the UI returns immediately ───
    std::thread::spawn(move || {
        // Give the first applications a moment to draw their initial window.
        std::thread::sleep(Duration::from_millis(1200));

        for (app, pid) in &launched {
            let Some(window_id) = claimer.claim(&app.exe_path, *pid, WINDOW_WAIT) else {
                continue; // app never showed a window; leave it where it landed
            };

            let (x, y, w, h) = absolute_bounds(app, &monitors, screen_width, screen_height);
            let _ = crate::native::set_window_position(window_id, x, y, w, h);
        }
    });

    let succeeded = results.iter().filter(|r| r.success).count();
    RestoreSummary {
        total,
        succeeded,
        failed: total - succeeded,
        results,
        browser_windows_restored,
    }
}

/// Expand a stored fractional rectangle back into pixels on the target monitor.
fn absolute_bounds(
    app: &CapturedApp,
    monitors: &[MonitorInfo],
    screen_width: u32,
    screen_height: u32,
) -> (i32, i32, i32, i32) {
    let pos = &app.relative_position;
    let idx = app.monitor_index as usize;

    if monitors.is_empty() {
        return (
            (pos.x * screen_width as f64) as i32,
            (pos.y * screen_height as f64) as i32,
            (pos.w * screen_width as f64) as i32,
            (pos.h * screen_height as f64) as i32,
        );
    }

    // A monitor that is no longer connected falls back to the primary.
    let m = monitors.get(idx).unwrap_or(&monitors[0]);
    let mw = m.width as f64;
    let mh = m.height as f64;
    (
        m.x + (pos.x * mw) as i32,
        m.y + (pos.y * mh) as i32,
        (pos.w * mw) as i32,
        (pos.h * mh) as i32,
    )
}

fn now_millis() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

/// True when `exe_path` looks like a real file we can execute directly rather
/// than a bare command name that needs shell resolution.
fn is_direct_executable(exe_path: &str) -> bool {
    let p = std::path::Path::new(exe_path);
    p.is_absolute() && p.is_file()
}

// ── Platform launch implementations ──────────────────────────────────────────
//
// Every branch returns the spawned process id when one is available, so the
// window claimer can prefer windows belonging to the process it just started.

#[cfg(target_os = "windows")]
fn platform_launch(app: &CapturedApp) -> Result<Option<u32>, String> {
    use std::os::windows::process::CommandExt;
    use std::process::Command;

    // Terminals need a console of their own; everything else must not flash one.
    let flags = if app.is_terminal {
        CREATE_NEW_CONSOLE
    } else {
        CREATE_NO_WINDOW
    };

    // Spawning the executable directly avoids `cmd /C start`, which was the
    // source of the stray console windows and orphaned shell processes.
    if is_direct_executable(&app.exe_path) {
        let mut cmd = Command::new(&app.exe_path);

        if app.is_vscode {
            // --new-window is what stops two projects collapsing into one window.
            cmd.arg("--new-window");
            if let Some(ctx) = &app.context_path {
                cmd.arg(ctx);
            }
        } else if app.is_terminal {
            if let Some(dir) = app.working_dir.as_deref() {
                if let Some(args) = procinfo::terminal_launch_args(&app.exe_path, dir) {
                    cmd.args(args);
                } else {
                    // No directory flag for this terminal — inheriting the working
                    // directory achieves the same thing for most shells.
                    cmd.current_dir(dir);
                }
            }
        } else if app.is_browser {
            if let Some(url) = &app.target_url {
                cmd.arg(url);
            }
        }

        return cmd
            .creation_flags(flags)
            .spawn()
            .map(|c| Some(c.id()))
            .map_err(|e| format!("{}: {e}", app.exe_path));
    }

    // Fallback for bare commands typed into a template (`code`, `chrome`).
    // `start` resolves them against PATH and app-path registry entries.
    let mut cmd = Command::new("cmd");
    cmd.args(["/C", "start", ""]);
    cmd.arg(&app.exe_path);
    if app.is_vscode {
        cmd.arg("--new-window");
        if let Some(ctx) = &app.context_path {
            cmd.arg(ctx);
        }
    } else if app.is_browser {
        if let Some(url) = &app.target_url {
            cmd.arg(url);
        }
    } else if app.is_terminal {
        if let Some(dir) = app.working_dir.as_deref() {
            if let Some(args) = procinfo::terminal_launch_args(&app.exe_path, dir) {
                cmd.args(args);
            }
        }
    }
    // `start` returns immediately, so its own pid is not the app's pid.
    cmd.creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map(|_| None)
        .map_err(|e| format!("{}: {e}", app.exe_path))
}

#[cfg(target_os = "macos")]
fn platform_launch(app: &CapturedApp) -> Result<Option<u32>, String> {
    use std::process::Command;

    if app.is_vscode {
        let mut cmd = Command::new("open");
        cmd.args(["-n", "-a", "Visual Studio Code", "--args", "--new-window"]);
        if let Some(ctx) = &app.context_path {
            cmd.arg(ctx);
        }
        return cmd.spawn().map(|_| None).map_err(|e| e.to_string());
    }

    if app.is_terminal {
        // `open -a Terminal <dir>` opens a new terminal already in that directory.
        let mut cmd = Command::new("open");
        cmd.args(["-n", "-a", &app.exe_path]);
        if let Some(dir) = &app.working_dir {
            cmd.arg(dir);
        }
        return cmd.spawn().map(|_| None).map_err(|e| e.to_string());
    }

    if app.is_browser {
        if let Some(url) = &app.target_url {
            return Command::new("open")
                .arg(url)
                .spawn()
                .map(|_| None)
                .map_err(|e| e.to_string());
        }
    }

    Command::new("open")
        .args(["-n", &app.exe_path])
        .spawn()
        .map(|_| None)
        .map_err(|e| e.to_string())
}

#[cfg(target_os = "linux")]
fn platform_launch(app: &CapturedApp) -> Result<Option<u32>, String> {
    use std::process::Command;

    let mut cmd = Command::new(&app.exe_path);

    if app.is_vscode {
        cmd.arg("--new-window");
        if let Some(ctx) = &app.context_path {
            cmd.arg(ctx);
        }
    } else if app.is_terminal {
        if let Some(dir) = app.working_dir.as_deref() {
            if let Some(args) = procinfo::terminal_launch_args(&app.exe_path, dir) {
                cmd.args(args);
            } else {
                cmd.current_dir(dir);
            }
        }
    } else if app.is_browser {
        if let Some(url) = &app.target_url {
            cmd.arg(url);
        }
    }

    cmd.spawn()
        .map(|c| Some(c.id()))
        .map_err(|e| format!("{}: {e}", app.exe_path))
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::MonitorInfo;
    use crate::models::RelativePosition;

    fn app_at(rel: RelativePosition, monitor_index: u32) -> CapturedApp {
        CapturedApp {
            name: "Test".into(),
            exe_path: "C:\\test.exe".into(),
            title: "Test".into(),
            is_vscode: false,
            is_browser: false,
            is_terminal: false,
            context_path: None,
            target_url: None,
            working_dir: None,
            relative_position: rel,
            monitor_index,
            browser_window: None,
        }
    }

    #[test]
    fn coordinate_expansion_1080p() {
        let monitors = vec![MonitorInfo {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        }];
        let app = app_at(
            RelativePosition {
                x: 0.5,
                y: 0.0,
                w: 0.5,
                h: 1.0,
            },
            0,
        );
        assert_eq!(absolute_bounds(&app, &monitors, 1920, 1080), (960, 0, 960, 1080));
    }

    #[test]
    fn coordinate_expansion_4k() {
        let monitors = vec![MonitorInfo {
            x: 0,
            y: 0,
            width: 3840,
            height: 2160,
        }];
        let app = app_at(
            RelativePosition {
                x: 0.0,
                y: 0.0,
                w: 0.5,
                h: 1.0,
            },
            0,
        );
        let (_, _, w, h) = absolute_bounds(&app, &monitors, 3840, 2160);
        assert_eq!((w, h), (1920, 2160));
    }

    #[test]
    fn coordinate_expansion_secondary_monitor() {
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
        let app = app_at(
            RelativePosition {
                x: 0.5,
                y: 0.0,
                w: 0.5,
                h: 1.0,
            },
            1,
        );
        // 1920 (monitor origin) + 0.5 * 2560 = 3200
        assert_eq!(absolute_bounds(&app, &monitors, 1920, 1080).0, 3200);
    }

    #[test]
    fn missing_monitor_falls_back_to_primary() {
        let monitors = vec![MonitorInfo {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        }];
        // Saved on a second monitor that has since been unplugged.
        let app = app_at(
            RelativePosition {
                x: 0.0,
                y: 0.0,
                w: 1.0,
                h: 1.0,
            },
            5,
        );
        assert_eq!(
            absolute_bounds(&app, &monitors, 1920, 1080),
            (0, 0, 1920, 1080)
        );
    }

    #[test]
    fn bare_command_is_not_treated_as_a_direct_executable() {
        assert!(!is_direct_executable("code"));
        assert!(!is_direct_executable("chrome"));
        // A path that does not exist cannot be spawned directly either.
        assert!(!is_direct_executable("C:\\definitely\\not\\here.exe"));
    }

    #[test]
    fn the_running_test_binary_is_a_direct_executable() {
        let exe = std::env::current_exe().unwrap();
        assert!(is_direct_executable(&exe.to_string_lossy()));
    }
}

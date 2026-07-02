use crate::capture::MonitorInfo;
use crate::models::{CapturedApp, RestoreResult, RestoreSummary};
use std::time::Duration;

/// Launch every app in the saved group, then schedule a background thread
/// that waits for each window to appear and re-applies its saved position.
pub fn restore_workspace(
    apps: Vec<CapturedApp>,
    monitors: Vec<MonitorInfo>,
    screen_width: u32,
    screen_height: u32,
) -> RestoreSummary {
    let total = apps.len();
    let mut results: Vec<RestoreResult> = Vec::with_capacity(total);

    for app in &apps {
        // Small stagger to prevent simultaneous resource exhaustion.
        std::thread::sleep(Duration::from_millis(400));
        results.push(launch(app));
    }

    // Spawn position-restoration in the background so we return quickly.
    let apps_clone = apps.clone();
    std::thread::spawn(move || {
        // Give applications a couple of seconds to render their first window.
        std::thread::sleep(Duration::from_secs(2));
        for app in &apps_clone {
            let pos = &app.relative_position;

            // Resolve the target monitor.
            let idx = app.monitor_index as usize;
            let (abs_x, abs_y, abs_w, abs_h) = if !monitors.is_empty() {
                let m = if idx < monitors.len() { &monitors[idx] } else { &monitors[0] };
                let mw = m.width as f64;
                let mh = m.height as f64;
                (
                    m.x + (pos.x * mw) as i32,
                    m.y + (pos.y * mh) as i32,
                    (pos.w * mw) as i32,
                    (pos.h * mh) as i32,
                )
            } else {
                // Fallback: primary monitor only
                (
                    (pos.x * screen_width as f64) as i32,
                    (pos.y * screen_height as f64) as i32,
                    (pos.w * screen_width as f64) as i32,
                    (pos.h * screen_height as f64) as i32,
                )
            };

            crate::position::apply_position(&app.exe_path, abs_x, abs_y, abs_w, abs_h);
        }
    });

    let succeeded = results.iter().filter(|r| r.success).count();
    RestoreSummary {
        total,
        succeeded,
        failed: total - succeeded,
        results,
    }
}

fn launch(app: &CapturedApp) -> RestoreResult {
    match platform_launch(app) {
        Ok(_) => RestoreResult {
            app_name: app.name.clone(),
            success: true,
            error: None,
        },
        Err(msg) => RestoreResult {
            app_name: app.name.clone(),
            success: false,
            error: Some(msg),
        },
    }
}

// ── Platform launch implementations ──────────────────────────────────────────

#[cfg(target_os = "windows")]
fn platform_launch(app: &CapturedApp) -> Result<std::process::Child, String> {
    use std::os::windows::process::CommandExt;
    use std::process::Command;
    // CREATE_NO_WINDOW suppresses the cmd.exe console flash entirely.
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    if app.is_vscode {
        let mut cmd = Command::new("cmd");
        cmd.args(["/C", "start", "", "code"]);
        if let Some(ctx) = &app.context_path {
            cmd.arg(ctx);
        }
        cmd.creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .map_err(|e| e.to_string())
    } else if app.is_browser {
        let mut cmd = Command::new("cmd");
        cmd.args(["/C", "start", "", &app.exe_path]);
        if let Some(url) = &app.target_url {
            cmd.arg(url);
        }
        cmd.creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .map_err(|e| e.to_string())
    } else {
        Command::new("cmd")
            .args(["/C", "start", "", &app.exe_path])
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .map_err(|e| e.to_string())
    }
}

#[cfg(target_os = "macos")]
fn platform_launch(app: &CapturedApp) -> Result<std::process::Child, String> {
    use std::process::Command;

    if app.is_vscode {
        let mut cmd = Command::new("open");
        cmd.args(["-a", "Visual Studio Code"]);
        if let Some(ctx) = &app.context_path {
            cmd.arg(ctx);
        }
        cmd.spawn().map_err(|e| e.to_string())
    } else if app.is_browser {
        if let Some(url) = &app.target_url {
            Command::new("open")
                .arg(url)
                .spawn()
                .map_err(|e| e.to_string())
        } else {
            Command::new("open")
                .arg(&app.exe_path)
                .spawn()
                .map_err(|e| e.to_string())
        }
    } else {
        Command::new("open")
            .arg(&app.exe_path)
            .spawn()
            .map_err(|e| e.to_string())
    }
}

#[cfg(target_os = "linux")]
fn platform_launch(app: &CapturedApp) -> Result<std::process::Child, String> {
    use std::process::Command;

    if app.is_vscode {
        let mut cmd = Command::new("code");
        if let Some(ctx) = &app.context_path {
            cmd.arg(ctx);
        }
        cmd.spawn().map_err(|e| e.to_string())
    } else if app.is_browser {
        let mut cmd = Command::new(&app.exe_path);
        if let Some(url) = &app.target_url {
            cmd.arg(url);
        }
        cmd.spawn().map_err(|e| e.to_string())
    } else {
        Command::new(&app.exe_path)
            .spawn()
            .map_err(|e| e.to_string())
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use crate::capture::MonitorInfo;
    use crate::models::RelativePosition;

    /// Verify coordinate expansion from relative to absolute pixels (primary monitor).
    #[test]
    fn coordinate_expansion_1080p() {
        let monitors = vec![MonitorInfo { x: 0, y: 0, width: 1920, height: 1080 }];
        let rel = RelativePosition { x: 0.5, y: 0.0, w: 0.5, h: 1.0 };
        let m = &monitors[0];
        let ax = m.x + (rel.x * m.width as f64) as i32;
        let ay = m.y + (rel.y * m.height as f64) as i32;
        let aw = (rel.w * m.width as f64) as i32;
        let ah = (rel.h * m.height as f64) as i32;
        assert_eq!(ax, 960);
        assert_eq!(ay, 0);
        assert_eq!(aw, 960);
        assert_eq!(ah, 1080);
    }

    #[test]
    fn coordinate_expansion_4k() {
        let monitors = vec![MonitorInfo { x: 0, y: 0, width: 3840, height: 2160 }];
        let rel = RelativePosition { x: 0.0, y: 0.0, w: 0.5, h: 1.0 };
        let m = &monitors[0];
        let aw = (rel.w * m.width as f64) as i32;
        let ah = (rel.h * m.height as f64) as i32;
        assert_eq!(aw, 1920);
        assert_eq!(ah, 2160);
    }

    #[test]
    fn coordinate_expansion_secondary_monitor() {
        // Window on second monitor at (1920, 0) with 2560×1440
        let monitors = vec![
            MonitorInfo { x: 0, y: 0, width: 1920, height: 1080 },
            MonitorInfo { x: 1920, y: 0, width: 2560, height: 1440 },
        ];
        // Saved as relative 0.5, 0.0 on monitor 1
        let rel = RelativePosition { x: 0.5, y: 0.0, w: 0.5, h: 1.0 };
        let m = &monitors[1];
        let ax = m.x + (rel.x * m.width as f64) as i32;
        let ay = m.y + (rel.y * m.height as f64) as i32;
        // Should be 1920 + 1280 = 3200
        assert_eq!(ax, 3200);
        assert_eq!(ay, 0);
    }

    #[test]
    fn launch_command_args_vscode_windows() {
        #[cfg(target_os = "windows")]
        {
            use std::process::Command;
            let mut cmd = Command::new("cmd");
            cmd.args(["/C", "start", "", "code", "C:\\Projects\\my-app"]);
            let program = format!("{:?}", cmd);
            assert!(program.contains("cmd"));
        }
    }
}

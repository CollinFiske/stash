use serde::{Deserialize, Serialize};

use crate::browser::BrowserWindowState;

/// Window position stored as percentages of the monitor's dimensions.
/// This allows layouts to be restored correctly on any monitor size.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelativePosition {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

/// A single captured application window ready for persistence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapturedApp {
    pub name: String,
    pub exe_path: String,
    pub title: String,
    pub is_vscode: bool,
    pub is_browser: bool,
    /// Terminal emulator or shell — restored into [`Self::working_dir`].
    #[serde(default)]
    pub is_terminal: bool,
    /// VS Code only: the absolute workspace folder resolved at capture time.
    pub context_path: Option<String>,
    /// Browser only: a single URL, used when the extension is not connected.
    pub target_url: Option<String>,
    /// Terminal only: the directory the shell was sitting in when captured.
    #[serde(default)]
    pub working_dir: Option<String>,
    pub relative_position: RelativePosition,
    /// Index into the monitor list (0 = primary). Default 0 for legacy data.
    #[serde(default)]
    pub monitor_index: u32,
    /// Browser only: full tab/group state supplied by the Stash extension.
    #[serde(default)]
    pub browser_window: Option<BrowserWindowState>,
}

/// Outcome of a single app launch attempt during restore.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreResult {
    pub app_name: String,
    pub success: bool,
    pub error: Option<String>,
    /// True when the window was found and moved into place. A launch can succeed
    /// while positioning fails (slow app, or a window Stash never saw appear).
    #[serde(default)]
    pub positioned: bool,
}

/// Aggregated result of restoring an entire workspace group.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreSummary {
    pub total: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub results: Vec<RestoreResult>,
    /// Number of browser windows handed to the extension to rebuild.
    #[serde(default)]
    pub browser_windows_restored: usize,
}

/// Outcome of closing a workspace's applications.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CloseSummary {
    /// Windows sent a graceful close request.
    pub closed: usize,
    /// Processes force-killed because graceful close was declined or timed out.
    pub force_killed: usize,
    /// Browser windows closed through the extension.
    pub browser_windows_closed: usize,
    /// Apps that were in the workspace but not running.
    pub skipped: usize,
    pub details: Vec<String>,
}

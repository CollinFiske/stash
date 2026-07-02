use serde::{Deserialize, Serialize};

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
    /// VS Code only: the workspace folder extracted from the window title.
    pub context_path: Option<String>,
    /// Browser only: the active tab URL from the accessibility API.
    pub target_url: Option<String>,
    pub relative_position: RelativePosition,
    /// Index into the monitor list (0 = primary). Default 0 for legacy data.
    #[serde(default)]
    pub monitor_index: u32,
}

/// Outcome of a single app launch attempt during restore.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreResult {
    pub app_name: String,
    pub success: bool,
    pub error: Option<String>,
}

/// Aggregated result of restoring an entire workspace group.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreSummary {
    pub total: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub results: Vec<RestoreResult>,
}

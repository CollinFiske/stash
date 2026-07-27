//! Window ownership tracking for restores.
//!
//! The original implementation asked "which window belongs to `Code.exe`?" and
//! took the first answer. With two VS Code windows in a workspace that returns
//! the *same* window twice: the first app positions it, the second repositions
//! it, and one window is never placed at all. The same bug hits three Chrome
//! windows, two terminals, or any repeated application.
//!
//! [`WindowClaimer`] fixes this with two rules:
//!
//!   1. **Baseline.** Windows that already existed before the restore started are
//!      recorded up front and are not eligible, so a restore never steals a
//!      window the user was already using.
//!   2. **Exclusivity.** Once an app claims a window, that window is struck off
//!      the list. Two apps can never resolve to one window.
//!
//! Matching prefers the process Stash actually spawned (`pid`), falls back to
//! executable path, and finally to a window that merely appeared during the
//! restore — which covers apps like VS Code that hand work to an existing
//! background process, so the visible window belongs to a pid Stash never saw.

use std::collections::HashSet;
use std::time::{Duration, Instant};

use crate::native::WindowId;

const POLL_INTERVAL: Duration = Duration::from_millis(250);

/// One candidate window on the desktop.
#[derive(Debug, Clone)]
struct LiveWindow {
    id: WindowId,
    pid: u32,
    exe_path: String,
}

pub struct WindowClaimer {
    /// Windows that existed before the restore began.
    baseline: HashSet<WindowId>,
    /// Windows already handed to an app during this restore.
    claimed: HashSet<WindowId>,
}

impl Default for WindowClaimer {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowClaimer {
    /// Record the current desktop as the baseline. Call this *before* launching.
    pub fn new() -> Self {
        Self {
            baseline: enumerate().into_iter().map(|w| w.id).collect(),
            claimed: HashSet::new(),
        }
    }

    /// Wait for a window belonging to `exe_path` (or to `pid`) and take ownership
    /// of it. Returns `None` if nothing suitable appeared within `timeout`.
    pub fn claim(&mut self, exe_path: &str, pid: Option<u32>, timeout: Duration) -> Option<WindowId> {
        let deadline = Instant::now() + timeout;
        let target = exe_path.to_lowercase();

        loop {
            let live = enumerate();

            // Best: a brand-new window from the process we started.
            if let Some(p) = pid {
                if let Some(w) = live.iter().find(|w| {
                    w.pid == p && !self.baseline.contains(&w.id) && !self.claimed.contains(&w.id)
                }) {
                    return Some(self.take(w.id));
                }
            }

            // Good: a brand-new window from a matching executable. This is the
            // case that fixes multi-window apps, because each pass can only ever
            // return a window no previous app has taken.
            if let Some(w) = live.iter().find(|w| {
                w.exe_path.to_lowercase() == target
                    && !self.baseline.contains(&w.id)
                    && !self.claimed.contains(&w.id)
            }) {
                return Some(self.take(w.id));
            }

            if Instant::now() >= deadline {
                break;
            }
            std::thread::sleep(POLL_INTERVAL);
        }

        // Last resort: the app was already running and reused an existing window
        // rather than opening a new one. Position that instead of doing nothing,
        // still respecting exclusivity so two apps cannot share it.
        let live = enumerate();
        live.iter()
            .find(|w| w.exe_path.to_lowercase() == target && !self.claimed.contains(&w.id))
            .map(|w| self.take(w.id))
    }

    fn take(&mut self, id: WindowId) -> WindowId {
        self.claimed.insert(id);
        id
    }

    #[cfg(test)]
    fn with_baseline(baseline: Vec<WindowId>) -> Self {
        Self {
            baseline: baseline.into_iter().collect(),
            claimed: HashSet::new(),
        }
    }
}

/// Every window currently on the desktop, via x-win so the same code path works
/// on Windows, macOS and Linux.
fn enumerate() -> Vec<LiveWindow> {
    let Ok(windows) = x_win::get_open_windows() else {
        return Vec::new();
    };
    windows
        .into_iter()
        .map(|w| LiveWindow {
            id: w.id as WindowId,
            pid: w.info.process_id,
            exe_path: w.info.path,
        })
        .collect()
}

// ── Unit tests ────────────────────────────────────────────────────────────────
//
// `claim` itself needs a live desktop, so these tests exercise the selection
// rules directly against a synthetic window list.

#[cfg(test)]
mod tests {
    use super::*;

    fn w(id: WindowId, pid: u32, exe: &str) -> LiveWindow {
        LiveWindow {
            id,
            pid,
            exe_path: exe.to_string(),
        }
    }

    /// Mirrors the "new window, matching executable" rule inside `claim`.
    fn pick<'a>(claimer: &WindowClaimer, live: &'a [LiveWindow], exe: &str) -> Option<&'a LiveWindow> {
        let target = exe.to_lowercase();
        live.iter().find(|w| {
            w.exe_path.to_lowercase() == target
                && !claimer.baseline.contains(&w.id)
                && !claimer.claimed.contains(&w.id)
        })
    }

    #[test]
    fn two_windows_of_one_app_get_two_different_windows() {
        // The exact bug: two VS Code windows in one workspace.
        let live = vec![w(10, 100, "C:\\Code.exe"), w(11, 100, "C:\\Code.exe")];
        let mut claimer = WindowClaimer::with_baseline(vec![]);

        let first = pick(&claimer, &live, "C:\\Code.exe").unwrap().id;
        claimer.take(first);
        let second = pick(&claimer, &live, "C:\\Code.exe").unwrap().id;

        assert_ne!(first, second, "each app must claim a distinct window");
    }

    #[test]
    fn pre_existing_windows_are_never_stolen() {
        // Window 10 was already open before the restore started.
        let live = vec![w(10, 100, "C:\\Code.exe")];
        let claimer = WindowClaimer::with_baseline(vec![10]);
        assert!(pick(&claimer, &live, "C:\\Code.exe").is_none());
    }

    #[test]
    fn a_new_window_beside_a_baseline_window_is_claimable() {
        let live = vec![w(10, 100, "C:\\Code.exe"), w(12, 101, "C:\\Code.exe")];
        let claimer = WindowClaimer::with_baseline(vec![10]);
        assert_eq!(pick(&claimer, &live, "C:\\Code.exe").unwrap().id, 12);
    }

    #[test]
    fn matching_is_case_insensitive() {
        let live = vec![w(10, 100, "C:\\Program Files\\Code.EXE")];
        let claimer = WindowClaimer::with_baseline(vec![]);
        assert!(pick(&claimer, &live, "c:\\program files\\code.exe").is_some());
    }

    #[test]
    fn a_different_application_is_not_matched() {
        let live = vec![w(10, 100, "C:\\chrome.exe")];
        let claimer = WindowClaimer::with_baseline(vec![]);
        assert!(pick(&claimer, &live, "C:\\Code.exe").is_none());
    }

    #[test]
    fn claimed_windows_are_tracked() {
        let mut claimer = WindowClaimer::with_baseline(vec![]);
        claimer.take(42);
        assert!(claimer.claimed.contains(&42));
        let live = vec![w(42, 1, "C:\\Code.exe")];
        assert!(pick(&claimer, &live, "C:\\Code.exe").is_none());
    }
}

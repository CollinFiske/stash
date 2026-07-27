//! Browser state model and OS-window ↔ browser-window correlation.
//!
//! The OS window manager can see that Chrome has three windows open; it cannot
//! see a single URL. The Chrome extension can enumerate every tab and tab group;
//! it has no idea where those windows sit on screen relative to everything else.
//!
//! This module joins the two halves. Both sides report window bounds in screen
//! coordinates, so a captured Chrome window can be matched to the extension's
//! view of the same window by geometry, and the tab list attached to it.

use serde::{Deserialize, Serialize};

/// One browser tab.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BrowserTab {
    pub url: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub pinned: bool,
    #[serde(default)]
    pub active: bool,
    /// Chrome tab-group id, or `None` when the tab is ungrouped.
    #[serde(default)]
    pub group_id: Option<i64>,
}

/// A Chrome tab group (the coloured, named pill above a run of tabs).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BrowserTabGroup {
    pub id: i64,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub color: String,
    #[serde(default)]
    pub collapsed: bool,
}

/// One browser window, with everything needed to rebuild it.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BrowserWindowState {
    /// The browser's own window id — only meaningful in the live session.
    #[serde(default)]
    pub window_id: i64,
    #[serde(default)]
    pub focused: bool,
    #[serde(default)]
    pub incognito: bool,
    /// `"normal"`, `"maximized"`, `"minimized"` or `"fullscreen"`.
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub left: i32,
    #[serde(default)]
    pub top: i32,
    #[serde(default)]
    pub width: i32,
    #[serde(default)]
    pub height: i32,
    #[serde(default)]
    pub tabs: Vec<BrowserTab>,
    #[serde(default)]
    pub groups: Vec<BrowserTabGroup>,
}

/// Everything the extension knows about the browser right now.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BrowserSnapshot {
    /// `"chrome"`, `"edge"`, `"brave"`, … as reported by the extension.
    #[serde(default)]
    pub browser: String,
    #[serde(default)]
    pub captured_at: String,
    #[serde(default)]
    pub windows: Vec<BrowserWindowState>,
}

impl BrowserSnapshot {
    pub fn tab_count(&self) -> usize {
        self.windows.iter().map(|w| w.tabs.len()).sum()
    }
}

/// Windows whose top-left corners are further apart than this (in pixels) are
/// never considered the same window. Chrome reports the *outer* frame while the
/// OS may report a slightly different rectangle, so an exact match is too strict.
const MATCH_TOLERANCE_PX: i64 = 120;

/// Find the extension-reported window that best matches an OS window at
/// `(x, y, w, h)`, ignoring any window index already in `taken`.
///
/// Scores candidates by summed corner distance and rejects anything beyond
/// [`MATCH_TOLERANCE_PX`], so an unmatched Chrome window yields `None` rather
/// than being wrongly paired with a window on another monitor.
pub fn correlate_window(
    windows: &[BrowserWindowState],
    taken: &[usize],
    x: i32,
    y: i32,
    w: i32,
    h: i32,
) -> Option<usize> {
    let mut best: Option<(i64, usize)> = None;

    for (i, bw) in windows.iter().enumerate() {
        if taken.contains(&i) {
            continue;
        }
        // A minimised browser window reports bogus coordinates; skip it.
        if bw.state == "minimized" {
            continue;
        }
        let origin_delta =
            (bw.left as i64 - x as i64).abs() + (bw.top as i64 - y as i64).abs();
        let size_delta =
            (bw.width as i64 - w as i64).abs() + (bw.height as i64 - h as i64).abs();

        if origin_delta > MATCH_TOLERANCE_PX {
            continue;
        }
        let score = origin_delta * 2 + size_delta;
        if best.map(|(s, _)| score < s).unwrap_or(true) {
            best = Some((score, i));
        }
    }

    best.map(|(_, i)| i)
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn win(id: i64, left: i32, top: i32, width: i32, height: i32) -> BrowserWindowState {
        BrowserWindowState {
            window_id: id,
            left,
            top,
            width,
            height,
            state: "normal".into(),
            ..Default::default()
        }
    }

    #[test]
    fn matches_the_geometrically_closest_window() {
        let windows = vec![win(1, 0, 0, 960, 1080), win(2, 960, 0, 960, 1080)];
        assert_eq!(correlate_window(&windows, &[], 0, 0, 960, 1080), Some(0));
        assert_eq!(correlate_window(&windows, &[], 960, 0, 960, 1080), Some(1));
    }

    #[test]
    fn tolerates_small_frame_differences() {
        let windows = vec![win(1, 8, 8, 1000, 1050)];
        // OS reports a slightly different rectangle than Chrome does.
        assert_eq!(correlate_window(&windows, &[], 0, 0, 960, 1080), Some(0));
    }

    #[test]
    fn refuses_to_match_a_far_away_window() {
        let windows = vec![win(1, 3000, 1200, 960, 1080)];
        assert_eq!(correlate_window(&windows, &[], 0, 0, 960, 1080), None);
    }

    #[test]
    fn never_returns_an_already_claimed_window() {
        let windows = vec![win(1, 0, 0, 960, 1080), win(2, 10, 10, 960, 1080)];
        let first = correlate_window(&windows, &[], 0, 0, 960, 1080).unwrap();
        let second = correlate_window(&windows, &[first], 0, 0, 960, 1080).unwrap();
        assert_ne!(first, second);
    }

    #[test]
    fn skips_minimized_windows() {
        let mut w = win(1, 0, 0, 960, 1080);
        w.state = "minimized".into();
        assert_eq!(correlate_window(&[w], &[], 0, 0, 960, 1080), None);
    }

    #[test]
    fn counts_tabs_across_windows() {
        let mut a = win(1, 0, 0, 100, 100);
        a.tabs = vec![BrowserTab::default(), BrowserTab::default()];
        let mut b = win(2, 0, 0, 100, 100);
        b.tabs = vec![BrowserTab::default()];
        let snap = BrowserSnapshot {
            windows: vec![a, b],
            ..Default::default()
        };
        assert_eq!(snap.tab_count(), 3);
    }
}

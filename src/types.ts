export interface RelativePosition {
  x: number;
  y: number;
  w: number;
  h: number;
}

// ── Browser state (supplied by the Stash Bridge extension) ────────────────────

export interface BrowserTab {
  url: string;
  title: string;
  pinned: boolean;
  active: boolean;
  /** Chrome tab-group id, or null when the tab is ungrouped. */
  group_id: number | null;
}

export interface BrowserTabGroup {
  id: number;
  title: string;
  color: string;
  collapsed: boolean;
}

export interface BrowserWindowState {
  window_id: number;
  focused: boolean;
  incognito: boolean;
  /** "normal" | "maximized" | "minimized" | "fullscreen" */
  state: string;
  left: number;
  top: number;
  width: number;
  height: number;
  tabs: BrowserTab[];
  groups: BrowserTabGroup[];
}

export interface BrowserSnapshot {
  browser: string;
  captured_at: string;
  windows: BrowserWindowState[];
}

/** Pairing and liveness details for the browser extension bridge. */
export interface BridgeStatus {
  running: boolean;
  port: number;
  token: string;
  connected: boolean;
  browser: string;
  window_count: number;
  tab_count: number;
  last_seen_secs: number | null;
  pending_commands: number;
}

// ── Captured applications ─────────────────────────────────────────────────────

export interface CapturedApp {
  name: string;
  exe_path: string;
  title: string;
  is_vscode: boolean;
  is_browser: boolean;
  /** Terminal emulator or shell — reopened in `working_dir`. */
  is_terminal?: boolean;
  context_path: string | null;
  target_url: string | null;
  /** Terminal only: the directory the shell was working in. */
  working_dir?: string | null;
  relative_position: RelativePosition;
  monitor_index?: number;
  /** Browser only: tabs and tab groups captured via the extension. */
  browser_window?: BrowserWindowState | null;
}

export interface HardFreeze {
  time: string;            // "HH:MM" 24-hour
  days: number[];          // 0=Sun … 6=Sat; empty array = every day
  warning_seconds: number; // countdown shown before trigger
  enabled: boolean;
}

export interface SavedGroup {
  name: string;
  created_at: string;
  apps: CapturedApp[];
  hard_freeze?: HardFreeze;
  launch_count?: number;
  last_used?: string;
  tags?: string[];
}

export interface WorkspaceStore {
  saved_groups: Record<string, SavedGroup>;
}

export interface RestoreResult {
  app_name: string;
  success: boolean;
  error: string | null;
  positioned?: boolean;
}

export interface RestoreSummary {
  total: number;
  succeeded: number;
  failed: number;
  results: RestoreResult[];
  browser_windows_restored?: number;
}

/** Outcome of closing a workspace's applications. */
export interface CloseSummary {
  closed: number;
  force_killed: number;
  browser_windows_closed: number;
  skipped: number;
  details: string[];
}

// ── Project Templates ─────────────────────────────────────────────────────────

/** One app entry in a manually-defined project template. */
export interface TemplateEntry {
  name: string;
  exe_path: string;
  is_vscode: boolean;
  is_browser: boolean;
  /** Terminal emulator or shell — opened in `working_dir`. */
  is_terminal?: boolean;
  context_path: string | null;
  target_url: string | null;
  /** Terminal only: the directory to open in. */
  working_dir?: string | null;
}

/** A reusable blueprint for a particular type of work session. */
export interface ProjectTemplate {
  name: string;
  created_at: string;
  entries: TemplateEntry[];
  hard_freeze?: HardFreeze;
  launch_count?: number;
  last_used?: string;
  tags?: string[];
}

// ── Import/Export ─────────────────────────────────────────────────────────────

export interface StashExport {
  version: number;
  exported_at: string;
  groups: Record<string, SavedGroup>;
  templates: Record<string, ProjectTemplate>;
}

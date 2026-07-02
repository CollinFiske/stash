export interface RelativePosition {
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface CapturedApp {
  name: string;
  exe_path: string;
  title: string;
  is_vscode: boolean;
  is_browser: boolean;
  context_path: string | null;
  target_url: string | null;
  relative_position: RelativePosition;
  monitor_index?: number;
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
}

export interface RestoreSummary {
  total: number;
  succeeded: number;
  failed: number;
  results: RestoreResult[];
}

// ── Project Templates ─────────────────────────────────────────────────────────

/** One app entry in a manually-defined project template. */
export interface TemplateEntry {
  name: string;
  exe_path: string;
  is_vscode: boolean;
  is_browser: boolean;
  context_path: string | null;
  target_url: string | null;
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

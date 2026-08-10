# Stash — Complete Project Context

> **⚠️ This document describes the state as of 2026-07-26 *before* the browser-extension /
> Win32 / terminal-CWD work landed.** For the current architecture see
> [README.md](../README.md) and [BROWSER-EXTENSION.md](BROWSER-EXTENSION.md).
> A changelog of what has since changed is at the **end of this file**.

> **Purpose of this document:** a single, self-contained brief describing what Stash is,
> what it does, why it exists, how it is built, what has broken along the way, and what is
> still open. Written to be pasted wholesale as context into another chat/LLM session.
> Everything below was derived by reading the actual source tree, not from memory.
>
> **Snapshot date:** 2026-07-26 · **Repo:** `<path-to-stash-repo>`
> **Branch:** `main` · **Head:** `cadfec8` · **Declared version:** `0.1.0` (UI About page says "Version 1.0")

---

## 1. Identity

**Stash** is a cross-platform **desktop workspace snapshot manager**. It captures every
open application window on your machine — including window geometry, the VS Code folder
you had open, and which monitor each window was on — saves that as a named **workspace**,
and later relaunches and repositions all of it in one click.

It is a **local-first, offline, no-account, no-cloud, no-telemetry** desktop app. All data
lives in one JSON file in the OS app-data directory.

One-sentence pitch (from the app's own About page):

> *"Stash is a workspace snapshot manager for your desktop. Save your open apps, restore
> them later, and build reusable launch templates for different types of work."*

---

## 2. Purpose & Problem Statement

### The problem
Knowledge/dev work is fragmented across 6–15 windows: an editor on a specific project
folder, two or three browser windows, a terminal, Slack, a notes app, a music player —
each in a specific place on a specific monitor. That arrangement is *the workspace*, and
it is destroyed every time you:

- shut down at the end of the day,
- reboot for an update,
- switch from "client A work" to "personal project" to "code review",
- or get pulled into a different context mid-day.

Rebuilding it manually costs several minutes of window-wrangling and a real amount of
attention. Nothing in Windows/macOS/Linux does this well out of the box — OS "restore
apps on restart" features do not restore *project context* (which folder VS Code had
open) and do not let you keep **multiple named layouts** you can switch between.

### The solution Stash implements
1. **Snapshot** — one click enumerates all visible user-facing windows and records app,
   executable path, title, window rectangle (as *fractions*, not pixels), monitor index,
   and — for VS Code — the resolved absolute workspace folder path.
2. **Restore** — one click relaunches every app, opening VS Code directly on the correct
   folder, then repositions each window against the *current* monitor layout.
3. **Templates** — hand-authored launch blueprints that are not tied to any moment in
   time, laid out on an auto-generated grid. "Frontend Dev" = VS Code + Chrome + terminal,
   defined once, launched forever.
4. **Hard Freeze** — a scheduled forced shutdown of a workspace's apps at a set time on
   set weekdays, with a countdown warning. A work/life boundary enforcement tool.

### Design philosophy (inferred consistently from the code)
- **Local-first and private.** Zero network calls anywhere in the codebase. No analytics.
- **Resolution-independent.** Positions are stored as `0.0–1.0` fractions of the monitor
  they were captured on, so a layout snapped on a 4K desktop restores sensibly on a 1080p
  laptop.
- **Best-effort, never blocking.** Restore returns a summary immediately; repositioning
  happens on a background thread and silently gives up rather than hanging.
- **Small surface, hand-rolled.** No UI framework, no CSS framework, no state library, no
  router, no database. ~2,100 lines of hand-written CSS with CSS custom properties; a
  hand-rolled 4-page "router" that is a `ref<"home"|"about"|"faq"|"contact">`.

### Explicit non-goals
- No cloud sync, accounts, or multi-device (import/export files are the sync story).
- No per-app deep state restore (browser tabs beyond one URL, unsaved documents, terminal
  scrollback).
- No mobile. `tauri::mobile_entry_point` is cfg-gated in but nothing targets mobile.

---

## 3. Users & Primary Use Cases

| # | Use case | Flow |
|---|---|---|
| UC-1 | End-of-day save / next-morning restore | Snapshot → name it → next day Launch |
| UC-2 | Context switch between projects | Keep N workspaces, launch the one you need |
| UC-3 | Repeatable work setups | Build a Template once, launch it forever |
| UC-4 | Quick launch without touching the UI | `Ctrl+Shift+S` → command palette → type → Enter |
| UC-5 | Shut down a whole context | "Close Apps" button on an active card |
| UC-6 | Enforced end of workday | Hard Freeze at 17:30 Mon–Fri with a 60s warning |
| UC-7 | Move setups between machines | Export `.stash` file → Import on the other machine |

---

## 4. Domain Model / Glossary

| Term | Meaning |
|---|---|
| **Workspace** (a.k.a. *group*, `SavedGroup`) | A **captured** point-in-time set of windows with real recorded positions. Created by Snapshot. |
| **Template** (`ProjectTemplate`) | A **hand-authored** list of apps with *no* recorded positions. Positions are generated by `autoLayout()` at launch time. |
| **App / Entry** (`CapturedApp` / `TemplateEntry`) | One application in a workspace/template. |
| **Relative position** (`RelativePosition`) | `{x, y, w, h}` as `0.0–1.0` fractions of the monitor the window lives on. |
| **Monitor index** | Index into the OS monitor list; `0` = primary. Stored per app. |
| **Context path** | For VS Code only: the **absolute** folder (or `.code-workspace` file) path to reopen. |
| **Target URL** | For browsers only: a URL to open. Currently only ever set by hand (see §12.3). |
| **Active** | A card is "active" when *every* app in it is currently detected as running. |
| **Hard Freeze** | Scheduled force-close of a workspace's apps at `HH:MM` on chosen weekdays. |
| **Snapshot** | The act of capturing; also the resulting workspace. |

**Key semantic difference:** *workspaces remember where things were; templates decide
where things go.*

---

## 5. Complete Feature Inventory

### 5.1 Capture / Snapshot
- Enumerates all open windows via the `x-win` Rust crate.
- Filters system noise by exact title match against a denylist: `program manager`,
  `desktop window manager`, `task switching`, `windows security`,
  `microsoft text input application`, `cortana`, `search`, `taskbar`,
  `windows input experience`, `notification centre`, `action center`, `start`, plus any
  empty title.
- **Excludes Stash itself**, including Tauri dev-mode binaries
  (`target/debug/stash`, `target/release/stash`).
- **Deduplicates** on `exe_path + "::" + title` — Chrome/Electron apps spawn many helper
  processes that would otherwise appear as separate entries.
- Classifies each window as VS Code / browser / regular.
- Computes per-monitor relative position and monitor index.
- Snapshot modal lets you **name** the workspace, and in "Edit" mode **remove** captured
  windows or **manually add** an app (with a native file-picker "Browse" button and
  auto-detection of type from the path).

### 5.2 Restore / Launch
- Launches each app with a **400 ms stagger** to avoid startup resource contention.
- Platform-specific launchers (see §8.4), including `--new-window` for VS Code on all
  three platforms.
- Returns a `RestoreSummary` (total / succeeded / failed / per-app results) immediately;
  the Restore modal shows per-app success dots and error strings.
- **Background repositioning thread:** sleeps 2 s, then per app polls `x_win::get_open_windows()`
  every 400 ms up to 15 attempts (~6 s ceiling per app), then applies the native geometry
  call. Silently no-ops if the window never appears.
- Increments `launch_count` and stamps `last_used` on success.

### 5.3 Project Templates
- Create/edit with a modal: name, tags, and an app list.
- Each entry: executable path *or bare command* (e.g. `code`, `chrome`), display name,
  type (regular / VS Code / browser), optional workspace folder, optional URL.
- Type auto-detection from the typed path.
- On launch, `autoLayout(n)` assigns positions (see §9.4) — templates never store geometry.

### 5.4 Active Workspace Detection
- Polls `capture_snapshot` every **8 seconds**.
- A card is *active* when every one of its apps is open. Matching is by **exe base name**
  (case-insensitive, `.exe` stripped); for VS Code entries it additionally requires the
  same `context_path`, so two different projects in VS Code are distinguished.
- Active cards get a colored border, an "● Active" / "● Live" badge, and a **Close Apps**
  button.

### 5.5 Close Apps
- `close_apps(exe_paths)` — force-terminates by image name.
- Windows: `taskkill /F /IM <exe>` with `CREATE_NO_WINDOW`.
- macOS: `osascript -e 'tell application "X" to quit'`.
- Linux: `pkill -f <name>`.

### 5.6 Hard Freeze
- Per workspace *and* per template. Config: `time` (`HH:MM`), `days` (`0`=Sun…`6`=Sat,
  empty = every day), `warning_seconds` (30–120, slider, step 10), `enabled` toggle.
- A frontend `setInterval` ticks every **5 seconds**.
- Inside the warning window, a full-width banner appears with a live countdown and a
  progress bar.
- At trigger time, `close_apps` is called with every exe path in the target.
- `triggeredToday` is an in-memory `Set` keyed `g-<name>-<YYYY-MM-DD>` / `t-<name>-<date>`
  to prevent double-firing within a day.

### 5.7 Command Palette
- `Ctrl+Shift+S` **global shortcut**, registered in Rust, works when the app is hidden or
  unfocused: shows + focuses the window, then emits a `palette-open` event the frontend
  listens for.
- Also openable from a lightning-bolt header button.
- Lists all workspaces + templates sorted by `last_used` desc, then name.
- Fuzzy-ish substring search over workspace name, app/entry names, exe base names, tags.
- Full keyboard driving: `↑`/`↓` navigate, `Enter` launch, `Esc` close, mouse hover
  syncs the active index.

### 5.8 Organization: search, sort, filter, tags, drag-reorder
- **Search** across name, tag, app name, and exe base name.
- **Sort** by Date Created / Last Used / Most Used / Most Apps / Name, asc or desc.
- **Tags** — lowercase free-text labels on both workspaces and templates. Filter chips are
  computed from the union of all tags; multiple selected tags are **AND**-ed. Tag pill
  colors are deterministic from a `hash * 31 + charCode` over a 6-color palette.
- **Manual drag-and-drop reordering** via HTML5 DnD, persisted separately as
  `group_order` / `template_order` arrays. Manual order is **bypassed while a search
  query or tag filter is active** (indices would not line up).

### 5.9 Edit / Delete / Undo
- Kebab (`⋮`) menu per card → Edit / Delete.
- Edit workspace: rename, edit tags, remove apps. Renaming rewrites the store key *and*
  the order array.
- Delete is a **soft delete with 5-second undo**: the item is removed from local state
  immediately, a toast with an "Undo" button appears, and a `setTimeout` commits the
  real store deletion. Starting a second delete flushes the first
  (`finalizePendingDelete`).

### 5.10 Import / Export
- Export → native save dialog → writes a `StashExport` JSON (`version`, `exported_at`,
  `groups`, `templates`) via the `write_file` command. Default filename
  `stash-export.stash`; filters accept `.stash` and `.json`.
- Import → native open dialog → `read_file` → validated for presence of `groups` and
  `templates` → each item saved (overwriting same-named items).

### 5.11 System tray & window behavior
- Tray icon with tooltip "Stash – Workspace Manager" and a menu: **Open Stash** /
  separator / **Quit Stash** (`std::process::exit(0)`).
- Left-click the tray icon → show + focus.
- **Closing the window does not quit** — `CloseRequested` is intercepted with
  `api.prevent_close()` and the window is hidden. Quitting is tray-only.

### 5.12 Icons & visual identity
- **All app icons are hand-authored inline SVG strings** in `src/appicons.ts` — no image
  files, no icon-font, no network fetch.
- ~17 fully drawn brand marks (VS Code, Chrome, Firefox, Safari, Edge, Brave, Opera,
  Slack, Spotify, Discord, WhatsApp, Figma, VLC, folder, Finder, OBS, terminal,
  PowerShell) plus ~40 generated "brand-color + initials" tiles (Photoshop `Ps`,
  IntelliJ `IJ`, Excel `X`, Claude `C`, …).
- Detection is an ordered cascade over lowercased name + path across categories: editors →
  communication → email → media/creative → productivity → Office → AI tools → engineering
  → system/terminal, with a final fallback of the first letter on a color picked by
  `charCodeAt(0) % 8`.
- Rendered via `v-html` (ESLint's `vue/no-v-html` is deliberately disabled; the `letter()`
  generator escapes `<` and `>`).

### 5.13 Static pages
About / FAQ / Contact, reachable from a hamburger menu, rendered by a `currentPage` ref.
The Contact page links to GitHub Issues, Discussions and the repo — no email is published.

### 5.14 Feedback & polish
Toasts (success/error, 3.5 s, or 5.5 s when carrying an Undo action), spinners on
launching cards, fade/slide modal transitions, empty states (full welcome state vs.
"templates exist but no snapshots" vs. "no search results"), ARIA roles throughout
(`role="dialog"`, `aria-modal`, `role="listbox"/"option"`, `role="switch"`,
`aria-live="polite"`, `role="alert"`).

---

## 6. Technology Stack

### 6.1 Runtime & shell
| Layer | Choice | Version | Notes |
|---|---|---|---|
| Desktop shell | **Tauri** | v2 | Rust core + OS webview; `tray-icon` feature on |
| Backend language | **Rust** | edition 2021, rustc 1.77+ | |
| Frontend framework | **Vue 3** | ^3.4.0 | Composition API, `<script setup lang="ts">` |
| Language | **TypeScript** | ^5.3.0 | strict-ish via `vue-tsc` |
| Bundler | **Vite** | ^5.0.0 | dev server pinned to port **1420** (`strictPort`) |
| Package manager | **Bun** | 1.1+ | `bun.lock` committed |
| Webview | WebView2 (Win) / WKWebView (mac) / WebKitGTK (Linux) | — | build targets `chrome105` / `safari13` / `firefox115` per platform |

### 6.2 Rust crates
| Crate | Version | Role |
|---|---|---|
| `tauri` | 2 (+`tray-icon`) | App shell, IPC, windows, tray |
| `tauri-plugin-store` | 2 | JSON key-value persistence |
| `tauri-plugin-dialog` | 2 | Native open/save file dialogs |
| `tauri-plugin-global-shortcut` | 2 | `Ctrl+Shift+S` hotkey |
| `x-win` | 5 | **Cross-platform window enumeration** — the load-bearing dependency |
| `serde` / `serde_json` | 1 | Serialization across the IPC boundary and for VS Code state parsing |

`user32.dll`'s `SetWindowPos` is called via a **raw `extern "system"` block** — no
`windows`/`winapi` crate dependency.

### 6.3 npm dependencies (deliberately tiny)
`@tauri-apps/api` ^2, `@tauri-apps/plugin-store` ^2, `@tauri-apps/plugin-dialog` ^2.7.1,
`vue` ^3.4.0. **That is the entire runtime dependency list.**

Dev: `@tauri-apps/cli` ^2, `@vitejs/plugin-vue` ^5, `vite` ^5, `typescript` ^5.3,
`vue-tsc` ^2, `vitest` ^4.1.9, `eslint` ^10.6 + `@eslint/js` ^10 +
`typescript-eslint` ^8.62 + `eslint-plugin-vue` ^10.9, `globals` ^17.7.

### 6.4 Deliberately **not** used
No Tailwind/UnoCSS · no Pinia/Vuex · no Vue Router · no component library · no SQLite or
any DB · no HTTP client · no icon package · no CSS preprocessor · no test-DOM
(`vitest` runs in the `node` environment against pure logic only).

### 6.5 Release profile (`Cargo.toml`)
```toml
panic = "abort"; codegen-units = 1; lto = true; opt-level = "s"; strip = true
```
Tuned hard for **binary size**.

---

## 7. Architecture

### 7.1 Shape
```
┌────────────────────────────────────────────────────────────┐
│  Vue 3 SPA  (webview)                                      │
│  App.vue = all state + orchestration (single-file store)   │
│  10 components · 3 pages · store.ts · layout.ts · appicons │
└───────────────┬─────────────────────────┬──────────────────┘
      invoke()  │                         │  @tauri-apps/plugin-store
                ▼                         ▼
┌───────────────────────────┐   ┌──────────────────────────┐
│ Rust core (7 commands)    │   │ workspaces.json          │
│ capture · restore ·       │   │ (OS app-data dir)        │
│ position · vscode_state   │   └──────────────────────────┘
└───────────┬───────────────┘
            ▼
   x-win  ·  SetWindowPos / AppleScript / xdotool
   cmd start / open / direct exec  ·  taskkill / osascript / pkill
   VS Code workspaceStorage (filesystem read)
```

Notable: **there is no state-management library and no router.** All application state is
`ref`s in `App.vue` (1,104 lines), passed down as props and returned as emitted events.
Persistence is called imperatively from event handlers.

### 7.2 IPC contract — all 7 Tauri commands

| Command | Args | Returns | Used by frontend? |
|---|---|---|---|
| `capture_snapshot` | *(window)* | `Vec<CapturedApp>` | ✅ Snapshot modal + 8 s active-detection poll |
| `restore_workspace` | `apps: Vec<CapturedApp>` | `RestoreSummary` | ✅ workspace + template launch |
| `get_screen_size` | *(window)* | `(u32, u32)` | ❌ **never invoked — dead surface** |
| `close_apps` | `exePaths: Vec<String>` | `()` | ✅ Close Apps button + Hard Freeze |
| `write_file` | `path, content` | `()` | ✅ Export |
| `read_file` | `path` | `String` | ✅ Import |
| `show_window` | *(window)* | `()` | ❌ **never invoked — tray/shortcut do it in Rust** |

Events (Rust → JS): **`palette-open`**, emitted by the global-shortcut handler.

Capabilities granted in `src-tauri/capabilities/default.json`:
`core:default`, `store:default`, `dialog:default`. The global-shortcut plugin needs no
JS-side permission because it is only driven from Rust.

### 7.3 Frontend file map

| File | Lines | Responsibility |
|---|---:|---|
| `src/App.vue` | 1104 | **Everything**: all state, all handlers, layout, page switching |
| `src/style.css` | 2135 | Hand-written design system, CSS custom properties, dark theme only |
| `src/types.ts` | 84 | 9 exported interfaces, the frontend/backend contract |
| `src/store.ts` | 109 | 12 thin wrappers over `tauri-plugin-store` |
| `src/appicons.ts` | 165 | Inline-SVG icon registry + detection cascade |
| `src/layout.ts` | 35 | `autoLayout(n)` grid generator |
| `src/main.ts` | 5 | Mount |
| `src/components/GroupCard.vue` | 177 | Workspace card |
| `src/components/TemplateCard.vue` | 177 | Template card (near-duplicate of GroupCard) |
| `src/components/SnapshotModal.vue` | 240 | Capture + name + edit + add-app |
| `src/components/TemplateModal.vue` | 237 | Create/edit template |
| `src/components/EditGroupModal.vue` | 133 | Rename workspace, edit tags, remove apps |
| `src/components/HardFreezeModal.vue` | 176 | Freeze scheduling UI |
| `src/components/CommandPalette.vue` | 163 | Quick launcher |
| `src/components/RestoreModal.vue` | 56 | Per-app restore results |
| `src/components/KebabMenu.vue` | 44 | `⋮` dropdown with outside-click close |
| `src/components/AppIcon.vue` | 41 | `v-html` SVG wrapper |
| `src/pages/{About,Faq,Contact}Page.vue` | 35/70/23 | Static content |

### 7.4 Rust module map

| Module | Responsibility |
|---|---|
| `main.rs` | Binary entry → `stash_lib::run()` |
| `lib.rs` | Builder, 3 plugins, `setup_tray`, `setup_shortcuts`, `setup_window_close`, handler registration |
| `commands.rs` | The 7 IPC commands, monitor queries, platform `close_single` |
| `models.rs` | `RelativePosition`, `CapturedApp`, `RestoreResult`, `RestoreSummary` |
| `capture.rs` | Window enumeration, noise/self filtering, dedupe, VS Code/browser classification, title parsing, monitor assignment, relative-coordinate math |
| `restore.rs` | Staggered launching, platform launch commands, background repositioning scheduler |
| `position.rs` | Poll-until-window-exists, then `SetWindowPos` / AppleScript / `xdotool` |
| `vscode_state.rs` | **New/uncommitted** — resolves a VS Code folder *base name* to a real absolute path |

---

## 8. Data Model & Persistence

### 8.1 Store
Single file **`workspaces.json`** in the Tauri app-data directory, via
`tauri-plugin-store` with `autoSave: false` and explicit `store.save()` after every
mutation. Four top-level keys:

| Key | Type |
|---|---|
| `saved_groups` | `Record<string, SavedGroup>` — **keyed by name** |
| `project_templates` | `Record<string, ProjectTemplate>` — keyed by name |
| `group_order` | `string[]` |
| `template_order` | `string[]` |

**Name is the primary key.** Saving over an existing name silently overwrites it.

### 8.2 TypeScript interfaces (`src/types.ts`, verbatim semantics)

```ts
interface RelativePosition { x: number; y: number; w: number; h: number }   // 0..1

interface CapturedApp {
  name: string;                 // process/app name from x-win
  exe_path: string;             // absolute exe path, or a bare command for hand-added
  title: string;                // window title at capture time
  is_vscode: boolean;
  is_browser: boolean;
  context_path: string | null;  // VS Code: ABSOLUTE folder path (resolved, see §9.3)
  target_url: string | null;    // browser: URL to open
  relative_position: RelativePosition;
  monitor_index?: number;       // 0 = primary
}

interface HardFreeze {
  time: string;                 // "HH:MM" 24-hour
  days: number[];               // 0=Sun..6=Sat; [] = every day
  warning_seconds: number;      // 30..120
  enabled: boolean;
}

interface SavedGroup {
  name: string; created_at: string; apps: CapturedApp[];
  hard_freeze?: HardFreeze; launch_count?: number; last_used?: string; tags?: string[];
}

interface TemplateEntry {       // NOTE: no position, no monitor_index
  name: string; exe_path: string; is_vscode: boolean; is_browser: boolean;
  context_path: string | null; target_url: string | null;
}

interface ProjectTemplate {
  name: string; created_at: string; entries: TemplateEntry[];
  hard_freeze?: HardFreeze; launch_count?: number; last_used?: string; tags?: string[];
}

interface RestoreResult  { app_name: string; success: boolean; error: string | null }
interface RestoreSummary { total: number; succeeded: number; failed: number; results: RestoreResult[] }
interface StashExport    { version: number; exported_at: string;
                           groups: Record<string, SavedGroup>;
                           templates: Record<string, ProjectTemplate> }
interface WorkspaceStore { saved_groups: Record<string, SavedGroup> }  // ⚠ vestigial, unused
```

### 8.3 Rust ↔ TS contract notes
- `models.rs` mirrors these with `serde`. Field names are `snake_case` on **both** sides —
  no rename attributes, no camelCase conversion.
- Tauri **command arguments** are the exception: they are camelCase over the wire
  (`invoke("close_apps", { exePaths })` → `pub async fn close_apps(exe_paths: Vec<String>)`).
- `monitor_index` is `Option`-less `u32` in Rust with `#[serde(default)]` for backward
  compatibility with workspaces saved before the field existed; it is `monitor_index?:
  number` (optional) in TS. Templates never set it → defaults to `0`.

### 8.4 Example persisted record
```json
{
  "saved_groups": {
    "Morning Coding": {
      "name": "Morning Coding",
      "created_at": "2026-06-25T09:00:00.000Z",
      "launch_count": 12,
      "last_used": "2026-07-25T08:41:03.221Z",
      "tags": ["work", "dev"],
      "hard_freeze": { "time": "17:30", "days": [1,2,3,4,5], "warning_seconds": 60, "enabled": true },
      "apps": [
        {
          "name": "Code",
          "exe_path": "C:\\Users\\me\\AppData\\Local\\Programs\\Microsoft VS Code\\Code.exe",
          "title": "main.rs — stash — Visual Studio Code",
          "is_vscode": true, "is_browser": false,
          "context_path": "C:\\Users\\me\\Documents\\prjs\\stash",
          "target_url": null,
          "relative_position": { "x": 0.0, "y": 0.0, "w": 0.5, "h": 1.0 },
          "monitor_index": 0
        }
      ]
    }
  },
  "project_templates": {},
  "group_order": ["Morning Coding"],
  "template_order": []
}
```

---

## 9. Key Algorithms

### 9.1 Resolution-independent positioning
**Capture** (per monitor `m` the window sits on):
```
rel.x = (win.x - m.x) / m.width      rel.w = win.width  / m.width
rel.y = (win.y - m.y) / m.height     rel.h = win.height / m.height     // all clamped 0..1
```
**Restore** (against the *current* monitor list):
```
abs.x = m.x + rel.x * m.width        abs.w = max(50, rel.w * m.width)
abs.y = m.y + rel.y * m.height       abs.h = max(50, rel.h * m.height)
```
Monitor assignment at capture is a point-in-rect test of the window's top-left corner
against each monitor's virtual-desktop rectangle, falling back to index `0`. If the saved
`monitor_index` exceeds the current monitor count at restore time, it falls back to
monitor `0` — so unplugging a second display degrades gracefully.

### 9.2 VS Code workspace name extraction (`capture.rs::extract_vscode_workspace`)
VS Code titles look like `"file — workspace — Visual Studio Code"`. The em-dash form
`" — "` is normalized to `" - "`, the title is split, and the **second-to-last** segment
is taken as the workspace name (rejecting `"visual studio code"` / `"vscode"` themselves).
A 2-segment title (`"Untitled-1 - Visual Studio Code"`) falls back to the first segment.

### 9.3 VS Code workspace **path** resolution (`vscode_state.rs`) — the hard part
The title only ever contains the folder's **base name**, never a path. Resolution:

1. Locate VS Code's own state dir, variant-aware (`Code` vs `Code - Insiders`, chosen by
   whether the exe path contains `insiders`):
   - Windows `%APPDATA%\Code\User\workspaceStorage`
   - macOS `~/Library/Application Support/Code/User/workspaceStorage`
   - Linux `~/.config/Code/User/workspaceStorage`
2. Read every subdirectory's `workspace.json` and take its `folder` or `workspace` field
   (a `file://` URI).
3. Decode the URI: percent-decoding (hand-rolled, no `percent-encoding` crate) plus the
   Windows leading-slash drive-letter form — `file:///c%3A/Users/…` → `C:\Users\…`,
   uppercasing the drive letter and converting `/` to `\`.
4. Keep entries whose `file_stem()` matches the title's base name case-insensitively.
5. **Tie-break by the storage directory's mtime**, preferring the most recently touched —
   this is what disambiguates two different projects that happen to share a folder name.

### 9.4 `autoLayout(n)` — template grid generator (`src/layout.ts`)
| n | Layout |
|---|---|
| 0 | `[]` |
| 1 | full screen |
| 2 | two halves side by side |
| 3 | left half + two right quarters |
| 4 | 2×2 |
| 5+ | `cols = ceil(sqrt(n))`, `rows = ceil(n/cols)` uniform grid |

Unit-tested to always produce `n` in-bounds, distinct positions for `n = 1..12`.

### 9.5 Active detection matching
Exe **base name** comparison (path separators normalized, lowercased, `.exe` stripped) —
so a workspace saved with an absolute path still matches a running process, and a template
entry written as the bare command `code` matches too. VS Code entries additionally require
a `context_path` match. A card is active only when **every** app matches.

### 9.6 Hard Freeze scheduler
Every 5 s: compute `today`, `currentDay`, `currentSecs`. For each enabled freeze not yet
in `triggeredToday`, skip if today is not in `days` (empty = all days), then compute
`until = freezeSecs - currentSecs`:
- `until <= 0 && until > -30` → **fire** (mark triggered, `close_apps`, toast, refresh)
- `0 < until <= warning_seconds` → show the countdown banner

### 9.7 Restore timing budget
`0.4 s × n` launch stagger (synchronous) → return summary → background thread sleeps 2 s →
per app up to `15 × 400 ms ≈ 6 s` of polling before giving up.

---

## 10. Build, Test & Quality Tooling

### 10.1 Commands
```bash
bun install          # deps
bun tauri dev        # dev (first run compiles x-win + all crates — several minutes)
bun tauri build      # release bundle (targets: "all")
bun run lint         # eslint src --ext .ts,.vue
bun run typecheck    # vue-tsc --noEmit
bun run test         # vitest run
bun run arch-check   # custom architecture-drift checker
bun run validate     # all 7 gates in sequence
cd src-tauri && cargo test
```

### 10.2 `bun run validate` — the 7 gates (`scripts/validate.ts`)
1. ESLint → 2. `vue-tsc --noEmit` → 3. Vitest → 4. arch-check → 5. `cargo clippy -- -D warnings`
→ 6. `cargo test` → 7. `vite build`.
Collects failures rather than bailing on the first, prints per-step timings, exits 1 on
any failure.

### 10.3 `scripts/arch-check.ts` — custom drift detector
Four checks, **bidirectional** where it matters:
1. Every `#[tauri::command]` in `commands.rs` is registered in `generate_handler!` **and**
   every registered name actually exists.
2. Every relative `.vue` import in `src/` resolves to a real file.
3. Every type exported from `types.ts` is referenced elsewhere in `src/`
   (with an explicit allowlist: `WorkspaceStore`).
4. Every file in `src-tauri/capabilities/` parses as JSON.

### 10.4 Test coverage
- **Vitest (34 tests, node env, no DOM):** `appicons.test.ts` — every input returns valid
  SVG including empty strings, brand colors present, Chrome ≠ Edge, path-based detection,
  letter fallback. `layout.test.ts` — exact geometry for n=1..4, 3×2 for 6, 3×3 for 9,
  in-bounds and distinct for 1..12.
- **Cargo (13 tests):** relative↔absolute math for 1080p/4K/secondary-monitor, VS Code
  title parsing (both dash forms), system-noise filtering, VS Code and browser detection,
  monitor point-in-rect assignment, Windows launch-arg construction, and (new)
  `vscode_state` URI decoding — Windows drive letters, spaces, percent-decode basics.
- **No component/UI tests at all.** No E2E. `App.vue` — the largest and most stateful file
  — has zero test coverage.

### 10.5 Verified status as of this document
```
arch-check : 37 checks, 0 errors, 1 warning (WorkspaceStore — expected)
Vitest     : 2 files, 34/34 passed
cargo test : 13/13 passed
```

### 10.6 Git hygiene
- `.gitignore` covers `node_modules/`, `dist/`, `build/`, `src-tauri/target/`,
  `chromium-data/`, `*.log`, `.DS_Store`, `Thumbs.db`, `.vscode`.
- `scripts/install-hooks.ps1` points `core.hooksPath` at `.githooks` for a pre-push
  `bun run validate` hook — **see §12.10, the hook directory does not exist.**
- Commit messages are currently untidy (`stuff`, `3`, `rest`, `easfdsf`, `ear`, `1232`).

---

## 11. Requirements Set

### 11.1 Functional requirements

| ID | Requirement | Status |
|---|---|---|
| FR-1 | Enumerate all visible user-facing windows on demand | ✅ |
| FR-2 | Exclude system/shell windows and Stash itself from capture | ✅ |
| FR-3 | Deduplicate multi-process apps into one entry per visible window | ✅ |
| FR-4 | Record window geometry monitor-relative as 0–1 fractions | ✅ |
| FR-5 | Record which monitor each window was on | ✅ |
| FR-6 | Detect VS Code windows and resolve the absolute open folder path | ✅ (new, uncommitted) |
| FR-7 | Detect browser windows and capture the active tab URL | ❌ **regressed — see §12.3** |
| FR-8 | Persist named workspaces locally with no account or network | ✅ |
| FR-9 | Relaunch every app in a workspace | ✅ |
| FR-10 | Open VS Code directly on the saved folder, in a new window | ✅ |
| FR-11 | Reposition restored windows against the current monitor layout | ✅ best-effort |
| FR-12 | Report per-app launch success/failure to the user | ✅ |
| FR-13 | Create/edit reusable templates with hand-picked apps | ✅ |
| FR-14 | Auto-generate a sensible grid layout for templates | ✅ |
| FR-15 | Detect and display which workspaces are currently live | ✅ (8 s poll) |
| FR-16 | Close every app in a workspace on demand | ✅ (force-kill) |
| FR-17 | Schedule a timed forced close (Hard Freeze) with warning | ✅ app-must-be-running |
| FR-18 | Global hotkey quick-launch palette | ✅ `Ctrl+Shift+S` |
| FR-19 | Search / sort / tag-filter / manually reorder saved items | ✅ |
| FR-20 | Rename, edit, and delete workspaces and templates | ✅ |
| FR-21 | Undo an accidental delete | ✅ 5 s window |
| FR-22 | Export and import all data as a portable file | ✅ `.stash` JSON |
| FR-23 | Run in the tray; closing the window must not quit the app | ✅ |
| FR-24 | Track usage stats (launch count, last used) | ✅ |
| FR-25 | Render a recognizable icon for every app, offline | ✅ inline SVG |

### 11.2 Non-functional requirements

| ID | Requirement | How it's met |
|---|---|---|
| NFR-1 | **Privacy** — no data leaves the machine | Zero network code anywhere |
| NFR-2 | **Offline** — full function with no connectivity | All assets inline; no CDN |
| NFR-3 | **Resolution independence** | Fractional coordinates + monitor rebasing |
| NFR-4 | **Graceful degradation** | Positioning silently skipped on timeout; unknown monitor → primary |
| NFR-5 | **Responsiveness** — UI never blocks on restore | Summary returns before repositioning |
| NFR-6 | **Small binary** | `opt-level="s"`, LTO, strip, `panic=abort`, 4 npm deps |
| NFR-7 | **Cross-platform** | `#[cfg(target_os)]` branches in `restore`, `position`, `commands`, `vscode_state` |
| NFR-8 | **Backward-compatible data** | `#[serde(default)]` on `monitor_index`; optional fields throughout |
| NFR-9 | **Accessibility** | ARIA roles/labels, keyboard-driven palette, focus management |
| NFR-10 | **Type safety across the IPC boundary** | Mirrored `types.ts` ↔ `models.rs`, enforced by `vue-tsc` + arch-check |
| NFR-11 | **Architecture-drift resistance** | `scripts/arch-check.ts` in the validate chain |
| NFR-12 | **Startup robustness** | Global shortcut unregistered before registering (survives crash/hot-reload) |

### 11.3 Platform requirements

| Platform | Build requirements | Runtime requirements |
|---|---|---|
| **Windows** (primary dev target) | MSVC C++ Build Tools | WebView2 (bundled with Win 11) |
| **macOS** | Xcode CLT (`xcode-select --install`) | **Accessibility permission** (Privacy & Security → Accessibility) for x-win + AppleScript positioning |
| **Linux** | `libwebkit2gtk-4.1-dev`, optional `libayatana-appindicator3-dev` | **`xdotool`** for positioning; X11 (Wayland only via XWayland) |

Toolchain: Bun 1.1+, Rust 1.77+, Tauri v2 system deps.

### 11.4 Data requirements
- DR-1 One JSON store, four keys, name-keyed maps.
- DR-2 All timestamps ISO-8601 strings (`new Date().toISOString()`).
- DR-3 Positions always fractional, never pixels.
- DR-4 Export format carries a `version` integer for future migration (currently `1`;
  **no migration code exists yet**).
- DR-5 Unknown/missing optional fields must not break loading older data.

### 11.5 Documentation requirements (what a complete doc set needs)
- DOC-1 README: what it is, install, run, build — **exists but has drifted (§12.11)**.
- DOC-2 Full IPC command reference including the two unused commands.
- DOC-3 Data schema + example file + where it lives per OS.
- DOC-4 Platform permission guide (macOS accessibility, Linux xdotool/X11).
- DOC-5 Known limitations page — currently only partially covered by the in-app FAQ.
- DOC-6 Contributing/quality-gate doc (`bun run validate`, the 7 gates, the hook setup).
- DOC-7 Contact page placeholder email must be replaced before any release.

---

## 12. Issues Encountered & How They Were Resolved

*These are the real problems this codebase has hit; each is evidenced by a fix, a filter,
a workaround, or an explicit comment in the source.*

### 12.1 ⭐ VS Code opened a blank file instead of the project — **the biggest one**
**Symptom:** restoring a VS Code window ran `code my-app` and VS Code created an empty
*file* named `my-app` instead of opening the project folder.
**Root cause:** the window title only ever exposes the folder's **base name**, never its
path. The original code passed that bare name straight to the `code` CLI.
**Fix:** new `src-tauri/src/vscode_state.rs` that cross-references **VS Code's own
`workspaceStorage` history** — reads every `workspace.json`, decodes its `file://` URI
(percent-decoding + Windows drive-letter handling), matches on base name, and tie-breaks
on directory mtime to pick the most recently used when several projects share a name.
`context_path` now stores an absolute path.
**Status:** implemented, tested (3 unit tests), **uncommitted working-tree change.**

### 12.2 VS Code reused an existing window instead of opening a new one
Restoring a multi-VS-Code-window workspace collapsed into one window.
**Fix:** `--new-window` added on all three platforms — Windows
`cmd /C start "" code --new-window <path>`, macOS
`open -n -a "Visual Studio Code" --args --new-window <path>`, Linux
`code --new-window <path>`. **Also uncommitted.**

### 12.3 ⚠️ Browser tab URL capture is disabled (live regression)
The README, the models, the restore launchers, the card chip labels, and the edit forms
all support `target_url` — but `capture.rs` now contains a hard-coded
`let target_url: Option<String> = None;`. Reading browser URLs through the OS
accessibility layer evidently proved unreliable and was cut.
**Effect today:** browsers restore to their homepage/session unless you type the URL in by
hand via the Snapshot-edit or Template form. `chipLabel()`'s hostname display is
effectively dead code for captured (as opposed to hand-authored) apps.
**Status:** open — the plumbing is intact, only the extraction is missing.

### 12.4 Console window flashed on every app launch (Windows)
Launching through `cmd /C start` popped a visible console window per app.
**Fix:** `CREATE_NO_WINDOW` (`0x08000000`) creation flag on every spawned `cmd` — and on
`taskkill` in `close_apps` too.

### 12.5 Stash captured itself, including the dev binary
**Fix:** `is_self_app()` matching `stash.exe`, `/stash`, and both `target/debug/stash` and
`target/release/stash` so `tauri dev` runs don't pollute snapshots.

### 12.6 One app produced a dozen entries
Chrome and Electron apps spawn many helper processes sharing one exe path.
**Fix:** dedupe on `exe_path + "::" + title`.

### 12.7 Shell/system windows polluted snapshots
Program Manager, taskbar, "Windows Input Experience", notification centre, etc.
**Fix:** the 12-entry `SYSTEM_TITLES` exact-match denylist plus an empty-title check.

### 12.8 Windows on a second monitor restored to the wrong place
The original model stored positions relative to the **primary** monitor only.
**Fix:** `MonitorInfo` list plumbed through both commands, `monitor_for_pos()`
point-in-rect assignment, per-monitor fractions, and a `monitor_index` field — added with
`#[serde(default)]` so pre-existing saved workspaces keep loading.

### 12.9 Windows were repositioned before they existed
Apps take seconds to create their first window; `SetWindowPos` on a not-yet-existing
window is a no-op.
**Fix:** 400 ms launch stagger, a background thread with a 2 s initial delay, then poll
`get_open_windows()` every 400 ms for up to 15 attempts, and give up silently rather than
hang.

### 12.10 Global shortcut stayed registered after a crash / hot reload
Re-registering `Ctrl+Shift+S` failed on the next `tauri dev` run.
**Fix:** an unconditional `unregister("Ctrl+Shift+S")` before registration, with an
explicit comment saying why.

### 12.11 Closing the window quit the whole app
For a tray-resident tool that's wrong.
**Fix:** `setup_window_close` intercepts `CloseRequested`, calls `api.prevent_close()`, and
hides the window. Quit is tray-menu-only.

### 12.12 Accidental deletes were unrecoverable
**Fix:** soft delete — remove from local state, show a 5 s toast with **Undo**, commit the
store deletion on a timer, and flush any pending delete before starting a new one so two
rapid deletes can't corrupt state.

### 12.13 Manual drag order fought with search/filter
Reordering a *filtered* list wrote nonsense into the persisted order.
**Fix:** `applyOrder()` returns the list untouched whenever a search query or tag filter is
active; items not present in the saved order are appended rather than dropped.

### 12.14 Architecture drift between the Rust and TS halves
Adding a Tauri command and forgetting to register it, or renaming a type, failed silently
or only at runtime.
**Fix:** `scripts/arch-check.ts`, wired into `bun run validate` as gate 4.

### 12.15 Build artifacts were committed to git
`dist/` was committed (`69fad90`), then deleted and gitignored (`cadfec8`). `.gitignore`
grew across three separate commits as this was cleaned up.

---

## 13. Open Issues, Limitations & Known Gaps

*Nothing in this section is fixed. This is the honest backlog.*

### 13.1 Correctness / behavior
1. **Browser URLs are never captured** (§12.3). The single biggest functional gap.
2. **Two windows of the same app get the same position.** `position.rs::apply_position()`
   matches the **first** window whose `info.path` equals the exe path and then `return`s.
   A workspace with two VS Code windows or two Chrome windows will have one placed and the
   other left alone (or both fighting for one rectangle). There is no window-identity
   tracking — no PID, no HWND, no title match at restore time.
3. **`close_apps` is a blunt instrument.** `taskkill /F /IM <exe>` force-kills **every**
   process with that image name, not only the ones belonging to the workspace — so closing
   a workspace containing Chrome kills *all* Chrome windows, including ones from another
   workspace, **with `/F`, discarding unsaved work without prompting**. Linux `pkill -f`
   is broader still.
4. **Hard Freeze only fires while Stash is running.** It is a frontend `setInterval`, not
   an OS-level scheduled task. Fully quitting Stash (tray → Quit) disables all freezes
   silently.
5. **Hard Freeze has a 30-second firing window.** The trigger condition is
   `until <= 0 && until > -30`. If the machine sleeps, hibernates, or the interval is
   starved across that window, the freeze silently never fires that day.
6. **`triggeredToday` is in-memory only.** Restarting Stash within a freeze's 30 s window
   can re-fire it; more importantly the guard is lost on every restart.
7. **Name is the primary key.** Saving a workspace with an existing name silently
   overwrites it — no confirmation prompt. The same applies to Import.
8. **Import has almost no validation.** It only checks that `groups` and `templates` exist;
   `version` is read but never acted on, and there is no migration path. A malformed or
   future-version file can write garbage into the store.
9. **Restore blocks for `0.4 s × n` inside an `async` command.** `restore_workspace` uses
   `std::thread::sleep` on the async runtime thread — ~3.2 s of blocking for 8 apps.
   Should be `tokio::time::sleep` or moved off-thread entirely.
10. **Active detection re-enumerates every window every 8 seconds** for the lifetime of the
    app, even when hidden in the tray, and `capture_snapshot` is invoked again for the
    Snapshot modal. Measurable idle CPU cost with no caching or backoff.
11. **Templates always restore to the primary monitor.** `TemplateEntry` has no
    `monitor_index`, so `autoLayout` positions are always relative to monitor 0.
12. **Freeze warning progress bar is hard-coded to a 120 s scale**
    (`Math.min(100, (secondsLeft / 120) * 100)`), so with the default 60 s warning the bar
    starts half-full instead of full.
13. **VS Code resolution can pick the wrong project.** Two folders with the same base name
    are disambiguated only by `workspaceStorage` mtime — a heuristic, not a guarantee. If
    the project was never opened via a path VS Code recorded, resolution returns `None` and
    the folder is not reopened at all.

### 13.2 Dead / unused surface
14. **`get_screen_size` and `show_window` are registered but never invoked** from the
    frontend. arch-check verifies *registration*, not *usage*, so it can't catch this.
15. **`WorkspaceStore` in `types.ts` is vestigial** — explicitly allowlisted in arch-check
    rather than deleted.
16. **`GroupCard.vue` and `TemplateCard.vue` are ~90% duplicated** (177 lines each) —
    identical `TAG_COLORS`, `tagColor`, `formatDate`, `formatLastUsed`, and near-identical
    templates. Same for the add-app form duplicated between `SnapshotModal` and
    `TemplateModal`.

### 13.3 Tooling / repo
17. **`scripts/install-hooks.ps1` references a `.githooks/` directory that does not exist
    in the repo.** Verified: no `.githooks` directory, and `core.hooksPath` is currently
    unset. Running the script points git at an empty path, so the pre-push `validate` hook
    it advertises **never runs**. Either commit `.githooks/pre-push` or drop the script.
18. **Commit messages are non-descriptive** (`stuff`, `3`, `rest`, `easfdsf`, `ear`,
    `1232`) — the history is not usable for archaeology.
19. **Uncommitted work in the tree** — the entire VS Code path-resolution feature
    (§12.1/§12.2) is unstaged. `src-tauri/src/vscode_state.rs` is untracked.
20. **`Cargo.toml` and the two `gen/schemas/*.json` show as modified but have empty
    diffs** — CRLF/LF line-ending churn only. Consider a `.gitattributes`.

### 13.4 Product / polish
21. **Dark theme only.** No light mode, no `prefers-color-scheme` handling.
22. **`:root` is capped at `max-width: 900px`** in `style.css` while the default window is
    960 px wide — content is centered and letterboxed, and there is commented-out
    experimental CSS left in the file.
23. ~~Contact page shows `placeholder@gmail.com`.~~ **Fixed** — now links to GitHub Issues.
24. ~~Version mismatch across the manifests and the About page.~~ **Fixed** — all three
    manifests say `1.0.0` and the About page reads it from `package.json` at build time.
25. **`csp: null`** in `tauri.conf.json` — no content-security policy is enforced.
26. **`write_file` / `read_file` are unrestricted arbitrary-path filesystem commands**
    exposed to the webview. Safe today because the paths come from native dialogs, but
    there is no path validation in the Rust handlers.
27. **No component or E2E tests.** `App.vue` (1,104 lines, all the state) is entirely
    untested; the test suites cover only the two pure-logic modules.
28. **README has drifted** — it documents 3 components and 3 commands (there are 13 `.vue`
    files under `components/`+`pages/` and 7 commands), describes browser URL capture as
    working, and predates templates, tags, Hard Freeze, the command palette, the tray,
    import/export, undo, and drag-reordering.
29. **No CI.** All quality gates are local-only; there is no GitHub Actions workflow.
30. **No app signing/notarization config** — macOS `signingIdentity` is `null`.

---

## 14. Current Repository State (2026-07-26)

```
Branch main @ cadfec8 · clean apart from the VS Code fix below

Modified:  src-tauri/src/capture.rs   (+1  → call resolve_workspace_path)
           src-tauri/src/lib.rs       (+1  → mod vscode_state)
           src-tauri/src/restore.rs   (+5/-2 → --new-window on all 3 platforms)
           src-tauri/Cargo.toml, gen/schemas/*.json  (line-endings only, no real diff)
Untracked: src-tauri/src/vscode_state.rs  (158 lines, the resolver + 3 tests)

Verified green: arch-check 37/37 · Vitest 34/34 · cargo test 13/13
```

The working tree contains a **complete, tested, unshipped feature**: VS Code project-path
resolution. That should be the next commit.

---

## 15. Fast Orientation Cheat-Sheet

- **Want to change what gets captured or filtered?** → `src-tauri/src/capture.rs`
- **Want to change how apps are launched?** → `src-tauri/src/restore.rs::platform_launch`
- **Want to change how windows are moved?** → `src-tauri/src/position.rs`
- **Adding a Tauri command?** → define in `commands.rs` **and** register in
  `lib.rs::generate_handler!` (arch-check gate 4 fails otherwise)
- **Adding a data field?** → mirror it in `src/types.ts` **and** `src-tauri/src/models.rs`,
  with `#[serde(default)]` if old data won't have it
- **All UI state lives in `src/App.vue`** — there is no store to look for
- **All styling is in `src/style.css`** — global classes, CSS custom properties, no scoped
  styles except two tiny blocks in `App.vue` and `AppIcon.vue`
- **Before pushing:** `bun run validate` (7 gates, ~1–2 min warm)


---

# Appendix — Changes since this document was written (2026-07-26)

The issues catalogued in §13 have largely been addressed. What changed:

## New Rust modules
| Module | Purpose |
|---|---|
| `win32.rs` | Hand-written Win32 FFI: `EnumWindows`, `GetWindowThreadProcessId`, `PostMessageW(WM_CLOSE)`, `SetWindowPos`, `ShowWindow`, `CreateToolhelp32Snapshot`, `NtQueryInformationProcess`, `ReadProcessMemory`, `DwmGetWindowAttribute` |
| `native.rs` | Cross-platform facade over window/process actions; nothing above it is platform-aware |
| `procinfo.rs` | Terminal working-directory resolution + relaunch argument construction |
| `browser.rs` | Browser tab/group model and OS-window ↔ browser-window correlation |
| `bridge.rs` | Loopback HTTP server (127.0.0.1:8787) the Chrome extension talks to |

`position.rs` was repurposed from "poll and call SetWindowPos" into `WindowClaimer`.

## Resolved from §13
| # | Issue | Resolution |
|---|---|---|
| 1 | Browser URLs never captured | **Fixed** via the Stash Bridge extension — full tabs, pinned state and tab groups, not just a URL |
| 2 | Two windows of one app got the same position | **Fixed** by `WindowClaimer`: pre-restore baseline + exclusive claiming, so each app gets a distinct window |
| 3 | `close_apps` force-killed by image name | **Fixed** — `close_workspace` posts `WM_CLOSE` / AppleScript `quit` / `SIGTERM` per window, so apps can prompt to save. `force: true` escalates after 2.5s |
| 4 | Hard Freeze only fires while Stash runs | **Unchanged** — still a frontend interval |
| 5–8 | Freeze window, `triggeredToday`, name-as-key, import validation | **Unchanged** |
| 9 | Restore blocked the async runtime | **Partly** — stagger reduced to 350ms and positioning moved fully to a background thread |
| 11 | Templates always restored to the primary monitor | **Unchanged** (`TemplateEntry` still has no `monitor_index`) |
| 14 | Registered-but-uninvoked commands | **Fixed** — arch-check gate 5 flags them, with an explicit allowlist |
| 16 | `GroupCard`/`TemplateCard` duplication | **Partly** — shared helpers extracted to `src/paths.ts` (`baseName`, `exeBase`, `hostOf`, `tabSummary`, `groupTitles`), removing four divergent copies |
| 27 | No tests for shared logic | **Improved** — Rust 13 → 55 tests, Vitest 34 → 53 |
| 28 | README drifted | **Fixed** — rewritten |

## New in this round
- **Terminal working directories.** Captured by reading the target process's PEB
  (`NtQueryInformationProcess` → `RTL_USER_PROCESS_PARAMETERS.CurrentDirectory`) on
  Windows, `/proc/<pid>/cwd` on Linux, `lsof` on macOS. Terminal *hosts* are walked up
  to 3 child levels to find the actual shell. Restored via `wt -d`, `cmd /K cd /d`,
  `pwsh -NoExit -Command Set-Location`, `alacritty --working-directory`, etc.
  This replaces the "run `cd` before closing" idea — it needs no cooperation from the
  shell and cannot disturb a half-typed command line.
- **Direct process spawning.** `Command::new(exe_path)` instead of `cmd /C start`,
  with `CREATE_NO_WINDOW` for GUI apps and `CREATE_NEW_CONSOLE` for terminals.
  Removes console flashes and orphaned shells. `cmd /C start` remains only as a
  fallback for bare command names typed into templates.
- **`CloseSummary`** reports closed / force-killed / browser-closed / skipped counts.
- **Global shortcut conflicts are non-fatal.** Previously a second Stash instance (or
  any app owning Ctrl+Shift+S) made `setup_shortcuts` return `Err`, which failed the
  whole Tauri setup hook and **panicked the app on startup**. Found by actually
  running the app. Bridge bind failures are non-fatal for the same reason.

## New data fields
`CapturedApp` gained `is_terminal`, `working_dir`, `browser_window`.
`TemplateEntry` gained `is_terminal`, `working_dir`.
`RestoreResult` gained `positioned`; `RestoreSummary` gained `browser_windows_restored`.
All are `#[serde(default)]` / optional, so existing `workspaces.json` files load unchanged.

## Command surface (was 7, now 9)
Added `close_workspace`, `bridge_status`, `browser_state`. Removed `close_apps`.
`get_screen_size` and `show_window` remain deliberately uninvoked (allowlisted in arch-check).

## The extension
`extension/` — Chrome MV3. Service worker pushes state every ~5s (and on tab changes,
debounced 800ms) and polls for commands every ~2s, with a `chrome.alarms` tick every
30s to survive service-worker eviction. Token-gated, loopback-only, incognito never
captured. Restored background tabs are `chrome.tabs.discard`-ed so reopening a 40-tab
workspace costs almost no memory. See [BROWSER-EXTENSION.md](BROWSER-EXTENSION.md).

## Still open
- Hard Freeze remains frontend-scheduled (§13.1 items 4–6).
- Name is still the primary key; import validation is still thin (§13.1 items 7–8).
- Templates still restore to the primary monitor (§13.1 item 11).
- Freeze warning bar still hard-codes a 120s scale (§13.1 item 12).
- No CI, no component/E2E tests, `csp: null`, placeholder contact email,
  version mismatch (0.1.0 vs "Version 1.0"), `.githooks/` still missing.

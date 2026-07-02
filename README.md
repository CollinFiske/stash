# Stash — Workspace Snapshot Manager

Stash is a cross-platform desktop application built with **Tauri v2** and **Vue 3** that lets you save a snapshot of your open application windows at the end of the day and restore them exactly—with positions—the next morning.

---

## How It Works (Under the Hood)

### IPC Bridge

The frontend (Vue 3 + Vite) communicates with the Rust backend via Tauri's IPC layer. Every user action that touches native OS APIs is issued as a `tauri::command` call using `invoke()` from `@tauri-apps/api/core`:

| Frontend call | Rust command | What it does |
|---|---|---|
| `invoke("capture_snapshot")` | `commands::capture_snapshot` | Queries all open windows via x-win and returns relative-position data |
| `invoke("restore_workspace", { apps })` | `commands::restore_workspace` | Launches each saved app and schedules background window positioning |
| `invoke("get_screen_size")` | `commands::get_screen_size` | Returns the primary monitor's pixel dimensions |

### JSON Persistence

All data is persisted via `tauri-plugin-store` to a local `workspaces.json` file located in the app's data directory (no cloud, no SQLite). The frontend manages reads/writes directly through `@tauri-apps/plugin-store`.

Schema:

```json
{
  "saved_groups": {
    "Morning Coding": {
      "name": "Morning Coding",
      "created_at": "2026-06-25T09:00:00.000Z",
      "apps": [
        {
          "name": "Code",
          "exe_path": "C:\\...\\Code.exe",
          "title": "main.rs — stash — Visual Studio Code",
          "is_vscode": true,
          "is_browser": false,
          "context_path": "stash",
          "target_url": null,
          "relative_position": { "x": 0.0, "y": 0.0, "w": 0.5, "h": 1.0 }
        }
      ]
    }
  }
}
```

### Window Capture (`src-tauri/src/capture.rs`)

1. Calls `x_win::get_open_windows()` to enumerate all visible windows.
2. Filters out system noise (Program Manager, Taskbar, empty titles, etc.).
3. Deduplicates by `exe_path + title` to avoid multiple shell processes appearing as separate entries.
4. Detects VS Code by matching `code.exe` or `visual studio code` in the name/path.
5. Extracts the VS Code workspace folder by parsing the title: `"file — workspace — Visual Studio Code"` → `"workspace"`.
6. Detects browsers (Chrome, Edge, Brave, Firefox, Safari, Arc) and reads the active tab URL from x-win's accessibility layer.
7. Converts absolute pixel coordinates to relative floats using the primary monitor's dimensions.

### Dynamic Rescaling

Positions are never stored as raw pixels. Instead:

```
relative_x = window_x / screen_width
relative_y = window_y / screen_height
relative_w = window_w / screen_width
relative_h = window_h / screen_height
```

On restore, the inverse is applied against the *current* monitor dimensions, so a layout snapped on a 4K monitor restores sensibly on a 1080p laptop.

### Window Restore (`src-tauri/src/restore.rs`)

1. Iterates the app list with a 400 ms stagger to avoid resource contention at launch.
2. Calls the platform-native launcher for each app:
   - **Windows**: `cmd /C start "" <exe>` (or `code <workspace>` for VS Code)
   - **macOS**: `open -a "Visual Studio Code" <workspace>` / `open <url>`
   - **Linux**: Direct binary invocation; `code <workspace>`
3. Returns a `RestoreSummary` immediately (non-blocking).
4. Spawns a background thread that polls `get_open_windows()` until each app's window appears, then applies `SetWindowPos` (Windows) / AppleScript (macOS) / `xdotool` (Linux).

---

## Directory Structure

```
stash/
├── index.html                         Entry HTML
├── package.json                       Bun/Node dependencies
├── vite.config.ts                     Vite build config
├── tsconfig.json                      TypeScript config
├── public/
│   └── stash.svg                      App icon (SVG)
├── src/                               Vue 3 frontend
│   ├── main.ts                        Vue entry point
│   ├── App.vue                        Root component — layout, state, orchestration
│   ├── style.css                      Global CSS design system (no Tailwind)
│   ├── types.ts                       Shared TypeScript interfaces
│   ├── store.ts                       tauri-plugin-store helpers (load/save/delete groups)
│   └── components/
│       ├── GroupCard.vue              Saved workspace card with Launch / Delete
│       ├── SnapshotModal.vue          Capture modal — name input + app preview
│       └── RestoreModal.vue           Restore result modal
└── src-tauri/                         Tauri/Rust backend
    ├── Cargo.toml                     Rust crate manifest
    ├── build.rs                       Tauri build script
    ├── tauri.conf.json                Tauri v2 configuration
    ├── capabilities/
    │   └── default.json               IPC permission grants (store:*, core:default)
    └── src/
        ├── main.rs                    Binary entry point
        ├── lib.rs                     Tauri Builder setup, plugin registration
        ├── commands.rs                Tauri IPC command handlers
        ├── models.rs                  Serde data models (CapturedApp, RestoreSummary…)
        ├── capture.rs                 Window enumeration, filtering, VS Code/browser detection
        ├── restore.rs                 App launching (platform-specific) + position scheduling
        └── position.rs                Native window positioning (Win32 / AppleScript / xdotool)
```

---

## Prerequisites

| Tool | Minimum version | Notes |
|---|---|---|
| [Bun](https://bun.sh) | 1.1+ | Package manager & runtime |
| [Rust](https://rustup.rs) | 1.77+ | Via rustup |
| Tauri v2 system deps | — | See platform notes below |

### Windows
- [Microsoft C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) (MSVC toolchain)
- WebView2 Runtime (ships with Windows 11; downloadable for Windows 10)

### macOS
- Xcode Command Line Tools: `xcode-select --install`
- **Accessibility permission**: On first launch, macOS will prompt to grant Stash access to the Accessibility API (required for x-win to read window URLs and for AppleScript positioning). Grant it in *System Settings → Privacy & Security → Accessibility*.

### Linux
- `libwebkit2gtk-4.1-dev` (Debian/Ubuntu) or equivalent
- `libayatana-appindicator3-dev` (optional, for tray support)
- For window positioning: `xdotool` (`apt install xdotool` / `dnf install xdotool`)
- Note: x-win uses X11; Wayland is supported via XWayland.

---

## Setup & Running

```bash
# 1. Install frontend dependencies
bun install

# 2. Start the development server (hot-reload frontend + Rust recompile)
bun tauri dev

# 3. Build a release bundle
bun tauri build
```

The first `tauri dev` will compile all Rust crates (including x-win) which takes a few minutes. Subsequent runs are incremental.

---

## Running Rust Tests

```bash
cd src-tauri
cargo test
```

Tests cover:
- Relative ↔ absolute coordinate conversion for 1080p and 4K displays
- VS Code workspace folder extraction from title strings
- System noise / browser / VS Code detection logic
- Launch command argument construction

---

## Feature Notes

### VS Code Workspace Detection
Stash parses the VS Code window title using the pattern `"file — workspace — Visual Studio Code"`. If multiple editors are open, each window is captured separately with its own workspace folder. On restore, `code <workspace-path>` opens VS Code directly in that folder.

### Browser URL Capture
Browser URLs are read via the OS accessibility layer (x-win). This requires:
- **Windows**: no extra steps (uses UI Automation / IAccessible)
- **macOS**: Accessibility permission (see above)
- **Linux**: X11 accessibility support; may not work on all distros

If URL capture fails, the browser is still launched without a specific URL.

### Window Positioning Fallback
Window repositioning is best-effort. If a launched app takes longer than ~6 seconds to show its first window (e.g. slow startup), Stash will skip repositioning for that app. The app still launches correctly.

### Name Conflicts
If you save a new workspace with the same name as an existing one, it overwrites the old entry. Choose unique names to avoid this.

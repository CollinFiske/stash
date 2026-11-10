# Stash — Workspace Snapshot Manager

Stash is a cross-platform desktop application built with **Tauri v2**, **Rust** and
**Vue 3** that saves your open application windows as a named workspace and
restores them later — with positions, VS Code project folders, terminal working
directories, and browser tabs intact.

Local-first: no account, no cloud, no telemetry. Everything lives in one JSON
file in your OS app-data folder.

---

## Download

**[Latest release →](https://github.com/CollinFiske/stash/releases/latest)**

| Platform | File | Notes |
|---|---|---|
| Windows | `Stash_x64-setup.exe` | Per-user install, no admin required |
| Linux | `Stash_amd64.AppImage` | Runs on any distro — `chmod +x` it first |
| Linux | `.deb` / `.rpm` | For Debian/Ubuntu and Fedora/RHEL |
| macOS | n/a | Coming Soon!! |

Browser-tab capture additionally needs the companion extension —
download from chrome web store: placeholderlink

---

## What it captures

| | Without help from the app | What Stash captures |
|---|---|---|
| **Any window** | — | executable, title, position, size, monitor |
| **VS Code** | window title only shows a folder *name* | the **absolute project path**, resolved from VS Code's own `workspaceStorage` |
| **Terminals** | reopen at the shell's default directory | the **working directory** the shell was actually in |
| **Browsers** | the OS cannot see a single tab | every **tab, pinned tab and tab group**, via the [Stash Bridge extension](extension/) |

---

## How It Works

### IPC Bridge

The Vue frontend talks to the Rust backend through Tauri's IPC layer. Every
action that touches native OS APIs is a `tauri::command` invoked with
`invoke()` from `@tauri-apps/api/core`:

| Frontend call | Rust command | What it does |
|---|---|---|
| `invoke("capture_snapshot")` | `commands::capture_snapshot` | Enumerates open windows, resolves VS Code paths and terminal directories, and attaches browser tab state |
| `invoke("restore_workspace", { apps })` | `commands::restore_workspace` | Launches each app, then claims and positions one window per app |
| `invoke("close_workspace", { apps, force })` | `commands::close_workspace` | Gracefully closes a workspace's windows; optionally escalates to termination |
| `invoke("bridge_status")` | `commands::bridge_status` | Pairing token and live connection state for the browser extension |
| `invoke("browser_state")` | `commands::browser_state` | The extension's latest view of every browser window and tab |
| `invoke("get_screen_size")` | `commands::get_screen_size` | Primary monitor dimensions (diagnostic) |
| `invoke("write_file" / "read_file")` | `commands::write_file` / `read_file` | Workspace export and import |
| `invoke("show_window")` | `commands::show_window` | Show and focus the main window |

Rust → JS events: **`palette-open`**, emitted by the `Ctrl+Shift+S` global shortcut.

### JSON Persistence

All data is persisted via `tauri-plugin-store` to `workspaces.json` in the app's
data directory. Four keys: `saved_groups`, `project_templates`, `group_order`,
`template_order`. Workspaces and templates are keyed by name.

### Window Capture (`src-tauri/src/capture.rs`)

1. Calls `x_win::get_open_windows()` to enumerate all visible windows.
2. Filters system noise (Program Manager, taskbar, Windows Input Experience, …).
3. Skips Stash itself, including `target/debug` dev builds.
4. Deduplicates by `exe_path + title`, so multi-process apps appear once.
5. Classifies each window as VS Code / browser / terminal / regular.
6. **VS Code** — parses the folder name out of the title, then resolves it to a
   real path (see below).
7. **Terminals** — reads the shell's working directory (see below).
8. **Browsers** — matches the window against the extension's report by geometry
   and attaches the full tab list.
9. Converts absolute pixel coordinates to fractions of the monitor the window
   sits on, and records that monitor's index.

### Dynamic Rescaling

Positions are never stored as raw pixels:

```
relative_x = (window_x - monitor_x) / monitor_width
relative_y = (window_y - monitor_y) / monitor_height
relative_w =  window_w              / monitor_width
relative_h =  window_h              / monitor_height
```

On restore the inverse is applied against the *current* monitor, so a layout
snapped on a 4K display restores sensibly on a 1080p laptop. A window saved on a
monitor that is no longer connected falls back to the primary.

### Window Restore (`src-tauri/src/restore.rs` + `position.rs`)

1. Browser windows are handed to the extension, which rebuilds them with their
   tabs and groups. Everything else is launched as a process.
2. Applications are **spawned directly** — `Command::new(exe_path)` — rather than
   through `cmd /C start`. GUI apps get `CREATE_NO_WINDOW`; terminals get
   `CREATE_NEW_CONSOLE`. This is what removed the console flashes and orphaned
   shell processes.
3. VS Code is always launched with `--new-window <absolute path>`, so two
   projects never collapse into one window.
4. Each app then **claims exactly one window** via `WindowClaimer`:
   - windows that existed *before* the restore are recorded and are off-limits,
   - once an app claims a window, no other app can take it,
   - matching prefers the spawned PID, then the executable path.

   This is what fixes two VS Code windows (or three Chrome windows) both
   resolving to the same window — previously one got positioned twice and the
   other not at all.
5. Positioning runs on a background thread, so the UI returns immediately.

### Closing a Workspace (`commands::close_workspace`)

Windows are asked to close the way clicking ✕ would — `WM_CLOSE` on Windows,
`quit` via AppleScript on macOS, `SIGTERM` on Linux — so applications can prompt
about unsaved work. This replaced a blanket `taskkill /F /IM <exe>`, which
force-killed *every* process sharing an image name and discarded unsaved work
without asking. Passing `force: true` escalates to termination after a 2.5 s
grace period; the scheduled Hard Freeze uses it, the Close Apps button does not.

### VS Code Project Resolution (`src-tauri/src/vscode_state.rs`)

A VS Code title only ever contains the folder's **base name**, never its path, so
launching `code my-app` made VS Code create an empty *file* called `my-app`.
Stash instead cross-references VS Code's own `workspaceStorage`: it reads each
`workspace.json`, decodes the `file://` URI (percent-decoding plus the Windows
drive-letter form `file:///c%3A/…` → `C:\…`), matches on base name, and
tie-breaks on directory mtime to pick the most recently used project.

### Terminal Working Directories (`src-tauri/src/procinfo.rs`)

A terminal that reopens in `System32` is useless. Stash reads the directory
directly rather than asking the shell:

- **Windows** — `NtQueryInformationProcess` → PEB → `RTL_USER_PROCESS_PARAMETERS`
  → `CurrentDirectory.DosPath`, read with `ReadProcessMemory`.
- **Linux** — the `/proc/<pid>/cwd` symlink.
- **macOS** — `lsof -a -p <pid> -d cwd -Fn`.

Terminal *hosts* (`WindowsTerminal.exe`, `gnome-terminal`) sit in their own
install directory, so Stash walks up to three levels of child processes to find
the shell (`pwsh`, `bash`, …) and prefers its directory. Boring defaults like
`C:\Windows\System32` are rejected.

On restore the directory is passed as a launch argument — `wt -d <dir>`,
`cmd /K cd /d <dir>`, `pwsh -NoExit -Command Set-Location …`,
`alacritty --working-directory <dir>` — falling back to setting the child
process's working directory for terminals with no such flag.

### Win32 Bindings (`src-tauri/src/win32.rs`)

Hand-written FFI declarations for the `user32` / `kernel32` / `ntdll` / `dwmapi`
entry points no crate in the tree exposes: `EnumWindows`,
`GetWindowThreadProcessId`, `PostMessageW(WM_CLOSE)`, `SetWindowPos`,
`ShowWindow`, `CreateToolhelp32Snapshot`, `NtQueryInformationProcess`,
`ReadProcessMemory`, and `DwmGetWindowAttribute` for detecting cloaked windows.
Declared by hand rather than pulling in `windows-sys`, keeping the dependency
tree and release binary small.

`src-tauri/src/native.rs` is the cross-platform facade over this; nothing above
it is platform-aware.

---

## Directory Structure

```
stash/
├── index.html                         Entry HTML
├── package.json                       Bun/Node dependencies
├── vite.config.ts                     Vite build config
├── docs/
│   ├── BROWSER-EXTENSION.md           Extension install + pairing walkthrough
│   └── STASH-CONTEXT.md               Full project reference
├── extension/                         Chrome MV3 extension ("Stash Bridge")
│   ├── manifest.json
│   ├── background.js                  Service worker: pushes state, polls commands
│   ├── options.html / options.js      Pairing UI
│   ├── popup.html / popup.js          Status popup
│   └── icons/
├── scripts/
│   ├── arch-check.ts                  Architecture drift checker
│   └── validate.ts                    Runs all 7 quality gates
├── src/                               Vue 3 frontend
│   ├── main.ts                        Vue entry point
│   ├── App.vue                        Root component — state and orchestration
│   ├── style.css                      Global CSS design system (no Tailwind)
│   ├── types.ts                       Shared TypeScript interfaces
│   ├── store.ts                       tauri-plugin-store helpers
│   ├── layout.ts                      autoLayout() grid generator
│   ├── paths.ts                       Path / URL / tab-summary helpers
│   ├── appicons.ts                    Inline-SVG app icon registry
│   ├── components/
│   │   ├── GroupCard.vue              Saved workspace card
│   │   ├── TemplateCard.vue           Project template card
│   │   ├── SnapshotModal.vue          Capture modal
│   │   ├── TemplateModal.vue          Template create/edit
│   │   ├── EditGroupModal.vue         Workspace rename / edit
│   │   ├── HardFreezeModal.vue        Scheduled close configuration
│   │   ├── BrowserPanel.vue           Extension pairing + live tab view
│   │   ├── CommandPalette.vue         Ctrl+Shift+S quick launcher
│   │   ├── RestoreModal.vue           Restore results
│   │   ├── KebabMenu.vue              ⋮ dropdown
│   │   └── AppIcon.vue                SVG icon wrapper
│   └── pages/                         About / FAQ / Contact
└── src-tauri/                         Tauri/Rust backend
    ├── Cargo.toml
    ├── capabilities/default.json      IPC permission grants
    └── src/
        ├── main.rs                    Binary entry point
        ├── lib.rs                     Builder, plugins, tray, shortcuts, bridge
        ├── commands.rs                Tauri IPC command handlers
        ├── models.rs                  Serde data models
        ├── capture.rs                 Window enumeration and classification
        ├── restore.rs                 Launching + restore orchestration
        ├── position.rs                WindowClaimer — one window per app
        ├── native.rs                  Cross-platform window/process actions
        ├── win32.rs                   Hand-written Win32 FFI (Windows only)
        ├── procinfo.rs                Terminal working-directory resolution
        ├── vscode_state.rs            VS Code workspace path resolution
        ├── browser.rs                 Browser state model + window correlation
        └── bridge.rs                  Loopback HTTP server for the extension
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
- Terminal working-directory reading requires a 64-bit build (the default).

### Linux
- `libwebkit2gtk-4.1-dev` (Debian/Ubuntu) or equivalent
- `libayatana-appindicator3-dev` (optional, for tray support)
- `xdotool` for window positioning
- x-win uses X11; Wayland is supported via XWayland.

### macOS
- Coming Soon!!

---

## Setup & Running

```bash
bun install          # 1. Install frontend dependencies
bun tauri dev        # 2. Dev server (hot-reload frontend + Rust recompile)
bun tauri build      # 3. Release bundle
```

### Browser tabs

Tab capture needs the companion Chrome extension.
See **[entension](extension/)** 

---

## Quality Gates

```bash
bun run validate     # all 7 gates in sequence
```

1. ESLint · 2. `vue-tsc --noEmit` · 3. Vitest · 4. arch-check ·
5. `cargo clippy -- -D warnings` · 6. `cargo test` · 7. `vite build`

Individually:

```bash
bun run lint         bun run typecheck      bun run test
bun run arch-check   cd src-tauri && cargo test
```

**arch-check** (`scripts/arch-check.ts`) catches drift between the Rust and
TypeScript halves:

1. Every `#[tauri::command]` is registered in `generate_handler!`, and vice versa.
2. Every relative `.vue` import resolves to a real file.
3. Every type exported from `types.ts` is used somewhere in `src/`.
4. Every capabilities JSON file parses.
5. Every registered command is actually invoked from the frontend (with an
   explicit allowlist for deliberately-unused ones).
6. Every `mod` declared in `lib.rs` has a source file.

---

## Feature Notes

### Multiple windows of the same app
Two VS Code windows, three Chrome windows and two terminals in one workspace all
restore to their own positions. `WindowClaimer` guarantees exclusivity — see
*Window Restore* above.

### Window positioning fallback
Repositioning is best-effort. If an app takes longer than ~12 seconds to show its
first window, Stash skips positioning for it; the app still launches. If the app
was already running and reused an existing window, Stash positions that window
instead of doing nothing.

### Browser memory
Restored background tabs are **discarded immediately** by the extension — they
appear in the tab strip but consume no memory until clicked. Reopening a 40-tab
workspace does not load 40 pages.

### Incognito / Private Browsing
The extension never captures incognito windows.

### Global shortcut conflicts
If another application (or a second copy of Stash) already owns `Ctrl+Shift+S`,
registration fails and is logged — Stash still starts, and the palette remains
reachable from the header button.

### Name conflicts
Saving a workspace with the name of an existing one overwrites it. Choose unique
names.

---

## Releasing

Releases are cut by pushing a tag. `.github/workflows/release.yml` then builds on
both platforms in parallel and collects the installers into a **draft** GitHub
Release for review.

```bash
# bump the version in package.json, src-tauri/tauri.conf.json and src-tauri/Cargo.toml
git tag v1.0.0
git push origin v1.0.0
```

Tauri cannot cross-compile — Windows installers must be built on Windows and Linux
packages on Linux — which is why this runs as a CI matrix rather than locally. The
Linux job pins `ubuntu-22.04` deliberately: binaries link against the build machine's
glibc, so building on a newer runner produces packages that will not start on older
distributions.

Building locally with `bun tauri build` produces artifacts for **your** platform only,
under `src-tauri/target/release/bundle/`.

## Website

The landing page lives in [`website/`](website/) as plain HTML with no build step,
and is hosted on Vercel with the project's **Root Directory** set to `website`.
Download buttons find the installers in the latest GitHub Release at page load, so
cutting a release doesn't require redeploying the site. Vercel skips deploys for
commits that don't touch `website/`.

## Contributing

Bug reports and pull requests are welcome. Run `bun run validate` before opening a PR —
it runs the same seven gates CI does.

## License

MIT © 2026 Collin Fiske. See [LICENSE](LICENSE).

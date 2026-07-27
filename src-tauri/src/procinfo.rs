//! Terminal working-directory resolution.
//!
//! A terminal window that reopens in `C:\Windows\System32` is useless — the whole
//! point of restoring it is to land back in the project you were working in.
//!
//! Terminals do not advertise their directory anywhere queryable, and the
//! terminal *host* process (`WindowsTerminal.exe`, `gnome-terminal`, `iTerm2`)
//! usually sits in its own install directory while the interesting path belongs
//! to the **shell running inside it** (`pwsh.exe`, `bash`, …). So resolution is:
//!
//!   1. read the host process's own working directory,
//!   2. walk its child processes looking for a shell,
//!   3. prefer the shell's directory, since that is what the user sees at the prompt.
//!
//! Reading it directly is strictly better than the `cd`-before-close trick: it
//! requires no cooperation from the shell, works on a terminal sitting at a
//! half-typed command, and cannot corrupt the user's input line.

/// Executables that host a terminal window but are not themselves the shell.
const TERMINAL_HOSTS: &[&str] = &[
    "windowsterminal",
    "wt",
    "conhost",
    "openconsole",
    "alacritty",
    "wezterm-gui",
    "wezterm",
    "kitty",
    "hyper",
    "tabby",
    "conemu",
    "conemu64",
    "mintty",
    "gnome-terminal-server",
    "gnome-terminal",
    "konsole",
    "xfce4-terminal",
    "xterm",
    "terminator",
    "iterm2",
    "terminal",
];

/// Shells whose working directory is the one the user actually cares about.
const SHELLS: &[&str] = &[
    "pwsh",
    "powershell",
    "cmd",
    "bash",
    "zsh",
    "fish",
    "sh",
    "nu",
    "git-bash",
    "busybox",
];

/// True when this executable is a terminal emulator or a shell.
pub fn is_terminal(name: &str, path: &str) -> bool {
    let stem = exe_stem(if path.is_empty() { name } else { path });
    TERMINAL_HOSTS.contains(&stem.as_str()) || SHELLS.contains(&stem.as_str())
}

/// Lower-cased file stem of an executable path (`C:\…\pwsh.exe` → `pwsh`).
pub fn exe_stem(path: &str) -> String {
    let normalised = path.replace('\\', "/");
    let base = normalised.rsplit('/').next().unwrap_or(&normalised);
    base.trim_end_matches(".exe")
        .trim_end_matches(".EXE")
        .trim_end_matches(".app")
        .to_lowercase()
}

/// Best-effort current working directory for a terminal window's process.
///
/// Returns `None` when the directory cannot be read (protected process, an
/// unsupported platform, or a path that no longer exists).
pub fn terminal_cwd(pid: u32) -> Option<String> {
    let own = read_cwd(pid);

    // Prefer a shell running inside the terminal host.
    for (child_pid, stem) in descendants(pid, 3) {
        if SHELLS.contains(&stem.as_str()) {
            if let Some(dir) = read_cwd(child_pid) {
                if is_useful_dir(&dir) {
                    return Some(dir);
                }
            }
        }
    }

    own.filter(|d| is_useful_dir(d))
}

/// Reject directories that carry no intent — system folders and install roots
/// are where terminals start by default, not where the user was working.
fn is_useful_dir(dir: &str) -> bool {
    if dir.trim().is_empty() {
        return false;
    }
    if !std::path::Path::new(dir).is_dir() {
        return false;
    }
    let lower = dir.to_lowercase().replace('\\', "/");
    const BORING: &[&str] = &[
        "c:/windows/system32",
        "c:/windows",
        "c:/program files",
        "c:/program files (x86)",
        "/usr/bin",
        "/bin",
        "/",
    ];
    !BORING.contains(&lower.trim_end_matches('/'))
}

// ── Platform: read one process's CWD ──────────────────────────────────────────

#[cfg(target_os = "windows")]
fn read_cwd(pid: u32) -> Option<String> {
    crate::win32::process_cwd(pid)
}

#[cfg(target_os = "linux")]
fn read_cwd(pid: u32) -> Option<String> {
    // Linux exposes it directly as a symlink.
    std::fs::read_link(format!("/proc/{pid}/cwd"))
        .ok()
        .map(|p| p.to_string_lossy().into_owned())
}

#[cfg(target_os = "macos")]
fn read_cwd(pid: u32) -> Option<String> {
    // `lsof -d cwd` is the only supported way to read another process's cwd
    // without private APIs. `-Fn` prints machine-readable `n<path>` lines.
    let out = std::process::Command::new("lsof")
        .args(["-a", "-p", &pid.to_string(), "-d", "cwd", "-Fn"])
        .output()
        .ok()?;
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .find_map(|l| l.strip_prefix('n').map(|s| s.to_string()))
}

// ── Platform: child process discovery ─────────────────────────────────────────

/// `(pid, exe_stem)` for descendants of `root`, up to `max_depth` levels deep.
#[cfg(target_os = "windows")]
fn descendants(root: u32, max_depth: usize) -> Vec<(u32, String)> {
    let all = crate::win32::process_list();
    let mut out = Vec::new();
    let mut frontier = vec![root];
    for _ in 0..max_depth {
        let mut next = Vec::new();
        for (pid, parent, name) in &all {
            if frontier.contains(parent) && *pid != *parent {
                out.push((*pid, exe_stem(name)));
                next.push(*pid);
            }
        }
        if next.is_empty() {
            break;
        }
        frontier = next;
    }
    out
}

#[cfg(not(target_os = "windows"))]
fn descendants(root: u32, max_depth: usize) -> Vec<(u32, String)> {
    // `ps -eo pid=,ppid=,comm=` is available on both macOS and Linux.
    let Ok(out) = std::process::Command::new("ps")
        .args(["-eo", "pid=,ppid=,comm="])
        .output()
    else {
        return Vec::new();
    };
    let text = String::from_utf8_lossy(&out.stdout);
    let rows: Vec<(u32, u32, String)> = text
        .lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let pid = parts.next()?.parse().ok()?;
            let ppid = parts.next()?.parse().ok()?;
            let comm = parts.next()?.to_string();
            Some((pid, ppid, comm))
        })
        .collect();

    let mut result = Vec::new();
    let mut frontier = vec![root];
    for _ in 0..max_depth {
        let mut next = Vec::new();
        for (pid, ppid, comm) in &rows {
            if frontier.contains(ppid) && pid != ppid {
                result.push((*pid, exe_stem(comm)));
                next.push(*pid);
            }
        }
        if next.is_empty() {
            break;
        }
        frontier = next;
    }
    result
}

// ── Relaunch argument construction ────────────────────────────────────────────

/// Build the argument list that reopens a terminal *in* `cwd`.
///
/// Returns `None` when the terminal takes no directory argument — in that case
/// the caller sets the child process's working directory instead, which most
/// shells inherit correctly.
pub fn terminal_launch_args(exe_path: &str, cwd: &str) -> Option<Vec<String>> {
    let stem = exe_stem(exe_path);
    let args: Vec<String> = match stem.as_str() {
        // Windows Terminal takes a starting-directory flag.
        "wt" | "windowsterminal" => vec!["-d".into(), cwd.into()],
        // `/K` keeps the shell open after running the `cd`.
        "cmd" => vec!["/K".into(), format!("cd /d \"{cwd}\"")],
        "powershell" | "pwsh" => vec![
            "-NoExit".into(),
            "-Command".into(),
            format!("Set-Location -LiteralPath '{}'", cwd.replace('\'', "''")),
        ],
        "alacritty" => vec!["--working-directory".into(), cwd.into()],
        "wezterm" | "wezterm-gui" => vec!["start".into(), "--cwd".into(), cwd.into()],
        "kitty" => vec!["--directory".into(), cwd.into()],
        "gnome-terminal" => vec![format!("--working-directory={cwd}")],
        "konsole" => vec!["--workdir".into(), cwd.into()],
        "xfce4-terminal" => vec![format!("--working-directory={cwd}")],
        "terminator" => vec!["--working-directory".into(), cwd.into()],
        _ => return None,
    };
    Some(args)
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exe_stem_strips_path_and_extension() {
        assert_eq!(exe_stem("C:\\Windows\\System32\\cmd.exe"), "cmd");
        assert_eq!(exe_stem("/usr/bin/bash"), "bash");
        assert_eq!(exe_stem("pwsh.exe"), "pwsh");
        assert_eq!(exe_stem("/Applications/iTerm2.app"), "iterm2");
    }

    #[test]
    fn detects_terminals_and_shells() {
        assert!(is_terminal("cmd", "C:\\Windows\\System32\\cmd.exe"));
        assert!(is_terminal("pwsh", "C:\\Program Files\\PowerShell\\7\\pwsh.exe"));
        assert!(is_terminal(
            "WindowsTerminal",
            "C:\\Program Files\\WindowsApps\\WindowsTerminal.exe"
        ));
        assert!(is_terminal("bash", "/usr/bin/bash"));
        assert!(!is_terminal("Code", "C:\\Code.exe"));
        assert!(!is_terminal("chrome", "chrome.exe"));
    }

    #[test]
    fn cmd_relaunch_args_cd_into_directory() {
        let args = terminal_launch_args("C:\\Windows\\System32\\cmd.exe", "C:\\Projects\\app")
            .expect("cmd should produce args");
        assert_eq!(args[0], "/K");
        assert!(args[1].contains("cd /d"));
        assert!(args[1].contains("C:\\Projects\\app"));
    }

    #[test]
    fn powershell_relaunch_args_escape_single_quotes() {
        let args = terminal_launch_args("pwsh.exe", "C:\\it's here").expect("pwsh args");
        assert_eq!(args[0], "-NoExit");
        // A literal apostrophe must be doubled for PowerShell single-quoted strings.
        assert!(args[2].contains("it''s here"), "got {:?}", args[2]);
    }

    #[test]
    fn windows_terminal_uses_directory_flag() {
        let args = terminal_launch_args("WindowsTerminal.exe", "D:\\work").unwrap();
        assert_eq!(args, vec!["-d".to_string(), "D:\\work".to_string()]);
    }

    #[test]
    fn unknown_terminal_has_no_special_args() {
        assert!(terminal_launch_args("some-random-app.exe", "C:\\x").is_none());
    }

    #[test]
    fn boring_system_directories_are_rejected() {
        assert!(!is_useful_dir(""));
        assert!(!is_useful_dir("C:\\Windows\\System32"));
        assert!(!is_useful_dir("/bin"));
    }
}

// ── Live-system smoke test ────────────────────────────────────────────────────

#[cfg(test)]
mod live_tests {
    use super::*;

    /// Reads this test process's own working directory through the same PEB path
    /// used for terminals. If the offsets or the ReadProcessMemory plumbing were
    /// wrong, this returns None or garbage.
    #[test]
    fn reads_our_own_working_directory() {
        let pid = std::process::id();
        let got = read_cwd(pid);

        // On Windows the detailed variant names the step that failed, which is
        // far more useful than a bare None when this regresses.
        #[cfg(all(target_os = "windows", target_pointer_width = "64"))]
        if got.is_none() {
            panic!(
                "process_cwd returned None for our own pid: {:?}",
                crate::win32::process_cwd_detailed(pid)
            );
        }

        // Elsewhere a missing /proc or lsof is a legitimate skip.
        let Some(dir) = got else { return };

        let expected = std::env::current_dir().expect("cwd");
        let resolved = std::path::Path::new(&dir);
        assert!(resolved.is_dir(), "returned path must exist: {dir}");
        assert_eq!(
            resolved.canonicalize().ok(),
            expected.canonicalize().ok(),
            "PEB-derived cwd ({dir}) must match std::env::current_dir()"
        );
    }

    /// The real scenario: reading the working directory out of a *different*
    /// process. Spawns a long-lived child in a known directory and reads it back.
    #[test]
    fn reads_another_processes_working_directory() {
        let target_dir = std::env::temp_dir().join(format!("stash-cwd-{}", std::process::id()));
        std::fs::create_dir_all(&target_dir).expect("create target dir");

        // A process that stays alive long enough to be inspected.
        #[cfg(target_os = "windows")]
        let mut child = std::process::Command::new("cmd")
            .args(["/C", "ping -n 20 127.0.0.1 > nul"])
            .current_dir(&target_dir)
            .spawn()
            .expect("spawn child");
        #[cfg(not(target_os = "windows"))]
        let mut child = std::process::Command::new("sleep")
            .arg("20")
            .current_dir(&target_dir)
            .spawn()
            .expect("spawn child");

        // Give the child a moment to finish initialising its PEB.
        std::thread::sleep(std::time::Duration::from_millis(600));

        let got = read_cwd(child.id());
        let _ = child.kill();
        let _ = child.wait();

        let Some(dir) = got else {
            let _ = std::fs::remove_dir_all(&target_dir);
            panic!("could not read the working directory of child pid {}", child.id());
        };

        let canonical_got = std::path::Path::new(&dir).canonicalize().ok();
        let canonical_want = target_dir.canonicalize().ok();
        let _ = std::fs::remove_dir_all(&target_dir);

        assert_eq!(
            canonical_got, canonical_want,
            "child cwd read from another process must match where it was spawned"
        );
    }

    /// `terminal_cwd` must reject a directory that carries no intent even when
    /// the raw read succeeds.
    #[test]
    fn terminal_cwd_filters_boring_directories() {
        // Our own process runs in the crate root, which is a real project dir.
        let dir = terminal_cwd(std::process::id());
        if let Some(d) = dir {
            assert!(is_useful_dir(&d), "terminal_cwd must only return useful dirs");
        }
    }
}

//! Resolves a VS Code window title's workspace name to a real filesystem path
//! by cross-referencing VS Code's own `workspaceStorage` history. The window
//! title only ever contains the folder's base name, never the full path, so
//! launching straight off the title (e.g. `code my-app`) makes VS Code create
//! a blank file called "my-app" instead of opening the folder.

use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Deserialize)]
struct WorkspaceJson {
    folder: Option<String>,
    workspace: Option<String>,
}

/// Find the absolute folder (or `.code-workspace` file) path whose base name
/// matches `candidate_name`, preferring the most recently touched entry when
/// several folders share the same base name.
pub fn resolve_workspace_path(candidate_name: &str, exe_path: &str) -> Option<String> {
    let storage_dir = workspace_storage_dir(exe_path)?;
    let entries = std::fs::read_dir(&storage_dir).ok()?;

    let mut best: Option<(std::time::SystemTime, String)> = None;

    for entry in entries.flatten() {
        let dir_path = entry.path();
        let json_path = dir_path.join("workspace.json");
        let Ok(raw) = std::fs::read_to_string(&json_path) else {
            continue;
        };
        let Ok(parsed) = serde_json::from_str::<WorkspaceJson>(&raw) else {
            continue;
        };

        let resolved = if let Some(folder_uri) = parsed.folder {
            decode_file_uri(&folder_uri)
        } else if let Some(workspace_uri) = parsed.workspace {
            decode_file_uri(&workspace_uri)
        } else {
            None
        };

        let Some(resolved_path) = resolved else {
            continue;
        };

        let base_matches = Path::new(&resolved_path)
            .file_stem()
            .map(|s| s.to_string_lossy().eq_ignore_ascii_case(candidate_name))
            .unwrap_or(false);
        if !base_matches {
            continue;
        }

        let mtime = std::fs::metadata(&dir_path)
            .and_then(|m| m.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);

        if best.as_ref().map(|(t, _)| mtime > *t).unwrap_or(true) {
            best = Some((mtime, resolved_path));
        }
    }

    best.map(|(_, path)| path)
}

/// Locate `<user data dir>/User/workspaceStorage` for the VS Code variant at `exe_path`.
fn workspace_storage_dir(exe_path: &str) -> Option<PathBuf> {
    let variant = if exe_path.to_lowercase().contains("insiders") {
        "Code - Insiders"
    } else {
        "Code"
    };

    let base: PathBuf = if cfg!(target_os = "windows") {
        PathBuf::from(std::env::var("APPDATA").ok()?)
    } else if cfg!(target_os = "macos") {
        PathBuf::from(std::env::var("HOME").ok()?).join("Library/Application Support")
    } else {
        PathBuf::from(std::env::var("HOME").ok()?).join(".config")
    };

    Some(base.join(variant).join("User").join("workspaceStorage"))
}

/// Decode a `file://` URI (as stored by VS Code) into a native filesystem path.
/// Handles percent-encoding and the leading-slash Windows drive-letter form
/// (`file:///c%3A/Users/...` → `C:\Users\...`).
fn decode_file_uri(uri: &str) -> Option<String> {
    let stripped = uri.strip_prefix("file://")?;
    let decoded = percent_decode(stripped);

    if cfg!(target_os = "windows") {
        let trimmed = decoded.strip_prefix('/').unwrap_or(&decoded);
        let mut chars = trimmed.chars();
        let native = match (chars.next(), chars.next()) {
            (Some(drive), Some(':')) => {
                format!("{}{}", drive.to_ascii_uppercase(), &trimmed[drive.len_utf8()..])
            }
            _ => trimmed.to_string(),
        };
        Some(native.replace('/', "\\"))
    } else {
        Some(decoded)
    }
}

fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(hex) = std::str::from_utf8(&bytes[i + 1..i + 3]) {
                if let Ok(byte) = u8::from_str_radix(hex, 16) {
                    out.push(byte);
                    i += 3;
                    continue;
                }
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_windows_drive_uri() {
        if cfg!(target_os = "windows") {
            let decoded = decode_file_uri("file:///c%3A/Users/cfisk/OneDrive/Documents/prjs/stash");
            assert_eq!(
                decoded,
                Some("C:\\Users\\cfisk\\OneDrive\\Documents\\prjs\\stash".to_string())
            );
        }
    }

    #[test]
    fn decode_uri_with_spaces() {
        if cfg!(target_os = "windows") {
            let decoded = decode_file_uri("file:///c%3A/Users/cfisk/My%20Project");
            assert_eq!(decoded, Some("C:\\Users\\cfisk\\My Project".to_string()));
        }
    }

    #[test]
    fn percent_decode_basic() {
        assert_eq!(percent_decode("hello%20world"), "hello world");
        assert_eq!(percent_decode("no-escapes"), "no-escapes");
        assert_eq!(percent_decode("c%3A"), "c:");
    }
}

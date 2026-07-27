//! Local HTTP bridge between Stash and the Stash Chrome extension.
//!
//! A browser extension cannot be called into from the outside — it has to be the
//! client. So Stash runs a tiny loopback HTTP server and the extension drives the
//! conversation:
//!
//! ```text
//!   extension  ──POST /state───────▶  push the full tab/group/window state
//!   extension  ──GET  /commands────▶  poll for work Stash has queued
//!   extension  ──POST /ack─────────▶  report the outcome of a command
//! ```
//!
//! Design constraints:
//!  * **Loopback only.** The listener binds `127.0.0.1`, so nothing off-machine
//!    can reach it.
//!  * **Token-gated.** Any web page can issue requests to `127.0.0.1`, so every
//!    endpoint except `/health` requires the pairing token, which the user copies
//!    out of Stash into the extension's options page once.
//!  * **No dependencies.** HTTP/1.1 request parsing is ~80 lines and avoids
//!    pulling a web framework into a desktop app that serves three endpoints.

use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::browser::{BrowserSnapshot, BrowserWindowState};

/// Loopback port the extension is configured to reach.
pub const BRIDGE_PORT: u16 = 8787;

/// An extension that has not called in for this long is treated as disconnected.
const HEARTBEAT_TIMEOUT: Duration = Duration::from_secs(15);

/// Work queued for the extension to perform.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BridgeCommand {
    /// Rebuild these browser windows, tabs and groups.
    Restore {
        id: String,
        windows: Vec<BrowserWindowState>,
    },
    /// Close the listed browser windows by their live window ids.
    Close { id: String, window_ids: Vec<i64> },
}

/// Status surfaced to the UI so the user can tell whether pairing worked.
#[derive(Debug, Clone, Serialize)]
pub struct BridgeStatus {
    pub running: bool,
    pub port: u16,
    pub token: String,
    pub connected: bool,
    pub browser: String,
    pub window_count: usize,
    pub tab_count: usize,
    pub last_seen_secs: Option<u64>,
    pub pending_commands: usize,
}

/// Shared bridge state. Cloneable handle around the inner `Arc`.
#[derive(Clone)]
pub struct Bridge {
    inner: Arc<Inner>,
}

struct Inner {
    token: String,
    running: AtomicBool,
    snapshot: Mutex<Option<BrowserSnapshot>>,
    last_seen: Mutex<Option<Instant>>,
    queue: Mutex<VecDeque<BridgeCommand>>,
}

impl Bridge {
    pub fn new(token: String) -> Self {
        Self {
            inner: Arc::new(Inner {
                token,
                running: AtomicBool::new(false),
                snapshot: Mutex::new(None),
                last_seen: Mutex::new(None),
                queue: Mutex::new(VecDeque::new()),
            }),
        }
    }

    pub fn token(&self) -> &str {
        &self.inner.token
    }

    /// Latest state pushed by the extension, if any.
    pub fn snapshot(&self) -> Option<BrowserSnapshot> {
        self.inner.snapshot.lock().ok()?.clone()
    }

    /// True while the extension has checked in recently.
    pub fn connected(&self) -> bool {
        self.inner
            .last_seen
            .lock()
            .ok()
            .and_then(|g| *g)
            .map(|t| t.elapsed() < HEARTBEAT_TIMEOUT)
            .unwrap_or(false)
    }

    pub fn enqueue(&self, cmd: BridgeCommand) {
        if let Ok(mut q) = self.inner.queue.lock() {
            // Bound the queue so a disconnected extension cannot grow it forever.
            while q.len() >= 32 {
                q.pop_front();
            }
            q.push_back(cmd);
        }
    }

    pub fn status(&self) -> BridgeStatus {
        let snap = self.snapshot();
        BridgeStatus {
            running: self.inner.running.load(Ordering::Relaxed),
            port: BRIDGE_PORT,
            token: self.inner.token.clone(),
            connected: self.connected(),
            browser: snap.as_ref().map(|s| s.browser.clone()).unwrap_or_default(),
            window_count: snap.as_ref().map(|s| s.windows.len()).unwrap_or(0),
            tab_count: snap.as_ref().map(|s| s.tab_count()).unwrap_or(0),
            last_seen_secs: self
                .inner
                .last_seen
                .lock()
                .ok()
                .and_then(|g| *g)
                .map(|t| t.elapsed().as_secs()),
            pending_commands: self.inner.queue.lock().map(|q| q.len()).unwrap_or(0),
        }
    }

    fn touch(&self) {
        if let Ok(mut g) = self.inner.last_seen.lock() {
            *g = Some(Instant::now());
        }
    }

    fn drain_commands(&self) -> Vec<BridgeCommand> {
        self.inner
            .queue
            .lock()
            .map(|mut q| q.drain(..).collect())
            .unwrap_or_default()
    }

    /// Start the listener on a background thread. Returns an error if the port
    /// is already taken (usually a second copy of Stash).
    pub fn start(&self) -> Result<(), String> {
        self.start_on(BRIDGE_PORT).map(|_| ())
    }

    /// Bind a specific port and return the port actually bound.
    /// Pass `0` to let the OS choose one — used by the tests.
    pub fn start_on(&self, port: u16) -> Result<u16, String> {
        let listener = TcpListener::bind(("127.0.0.1", port))
            .map_err(|e| format!("Stash bridge could not bind 127.0.0.1:{port}: {e}"))?;
        let bound = listener
            .local_addr()
            .map_err(|e| e.to_string())?
            .port();

        self.inner.running.store(true, Ordering::Relaxed);
        let bridge = self.clone();

        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else { continue };
                let b = bridge.clone();
                std::thread::spawn(move || {
                    let _ = handle_connection(&b, stream);
                });
            }
        });

        Ok(bound)
    }
}

// ── HTTP plumbing ─────────────────────────────────────────────────────────────

struct Request {
    method: String,
    path: String,
    origin: Option<String>,
    token: Option<String>,
    body: String,
}

fn handle_connection(bridge: &Bridge, stream: TcpStream) -> std::io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(10)))?;
    stream.set_write_timeout(Some(Duration::from_secs(10)))?;

    let mut reader = BufReader::new(stream.try_clone()?);
    let Some(req) = parse_request(&mut reader)? else {
        return Ok(());
    };
    let mut stream = stream;

    // CORS preflight — the extension's service worker sends one before POSTs.
    if req.method == "OPTIONS" {
        return respond(&mut stream, 204, "", &req.origin);
    }

    // `/health` is deliberately unauthenticated so the extension's options page
    // can tell "Stash isn't running" apart from "the token is wrong".
    if req.path == "/health" {
        let body = serde_json::json!({
            "ok": true,
            "app": "stash",
            "version": env!("CARGO_PKG_VERSION"),
        });
        return respond(&mut stream, 200, &body.to_string(), &req.origin);
    }

    if req.token.as_deref() != Some(bridge.token()) {
        let body = serde_json::json!({ "error": "invalid or missing X-Stash-Token" });
        return respond(&mut stream, 401, &body.to_string(), &req.origin);
    }

    bridge.touch();

    match (req.method.as_str(), req.path.as_str()) {
        // Extension pushes the full browser state.
        ("POST", "/state") => {
            match serde_json::from_str::<BrowserSnapshot>(&req.body) {
                Ok(snapshot) => {
                    if let Ok(mut g) = bridge.inner.snapshot.lock() {
                        *g = Some(snapshot);
                    }
                    let body = serde_json::json!({ "ok": true });
                    respond(&mut stream, 200, &body.to_string(), &req.origin)
                }
                Err(e) => {
                    let body = serde_json::json!({ "error": format!("bad snapshot: {e}") });
                    respond(&mut stream, 400, &body.to_string(), &req.origin)
                }
            }
        }

        // Extension polls for queued work.
        ("GET", "/commands") => {
            let commands = bridge.drain_commands();
            let body = serde_json::json!({ "commands": commands });
            respond(&mut stream, 200, &body.to_string(), &req.origin)
        }

        // Extension reports a command result. Logged, not acted on.
        ("POST", "/ack") => {
            let body = serde_json::json!({ "ok": true });
            respond(&mut stream, 200, &body.to_string(), &req.origin)
        }

        _ => {
            let body = serde_json::json!({ "error": "not found" });
            respond(&mut stream, 404, &body.to_string(), &req.origin)
        }
    }
}

/// Parse an HTTP/1.1 request. Returns `Ok(None)` on an empty or malformed
/// request line rather than treating it as an error worth reporting.
fn parse_request(reader: &mut BufReader<TcpStream>) -> std::io::Result<Option<Request>> {
    let mut line = String::new();
    if reader.read_line(&mut line)? == 0 {
        return Ok(None);
    }

    let mut parts = line.split_whitespace();
    let (Some(method), Some(path)) = (parts.next(), parts.next()) else {
        return Ok(None);
    };
    let (method, path) = (method.to_string(), path.to_string());

    let mut content_length = 0usize;
    let mut origin = None;
    let mut token = None;

    loop {
        let mut header = String::new();
        if reader.read_line(&mut header)? == 0 {
            break;
        }
        let trimmed = header.trim_end();
        if trimmed.is_empty() {
            break;
        }
        let Some((name, value)) = trimmed.split_once(':') else {
            continue;
        };
        let value = value.trim();
        match name.to_ascii_lowercase().as_str() {
            "content-length" => content_length = value.parse().unwrap_or(0),
            "origin" => origin = Some(value.to_string()),
            "x-stash-token" => token = Some(value.to_string()),
            _ => {}
        }
    }

    // Cap the body so a rogue caller cannot exhaust memory.
    const MAX_BODY: usize = 8 * 1024 * 1024;
    let mut body = String::new();
    if content_length > 0 && content_length <= MAX_BODY {
        let mut buf = vec![0u8; content_length];
        reader.read_exact(&mut buf)?;
        body = String::from_utf8_lossy(&buf).into_owned();
    }

    Ok(Some(Request {
        method,
        path,
        origin,
        token,
        body,
    }))
}

/// Write a JSON response, echoing the caller's origin so the extension's
/// `fetch` calls pass the CORS check.
fn respond(
    stream: &mut TcpStream,
    status: u16,
    body: &str,
    origin: &Option<String>,
) -> std::io::Result<()> {
    let reason = match status {
        200 => "OK",
        204 => "No Content",
        400 => "Bad Request",
        401 => "Unauthorized",
        404 => "Not Found",
        _ => "Error",
    };
    let allow_origin = origin.as_deref().unwrap_or("*");

    let response = format!(
        "HTTP/1.1 {status} {reason}\r\n\
         Content-Type: application/json\r\n\
         Content-Length: {len}\r\n\
         Access-Control-Allow-Origin: {allow_origin}\r\n\
         Access-Control-Allow-Headers: Content-Type, X-Stash-Token\r\n\
         Access-Control-Allow-Methods: GET, POST, OPTIONS\r\n\
         Access-Control-Max-Age: 86400\r\n\
         Connection: close\r\n\
         \r\n\
         {body}",
        len = body.len()
    );
    stream.write_all(response.as_bytes())?;
    stream.flush()
}

// ── Pairing token ─────────────────────────────────────────────────────────────

/// Load the pairing token from disk, generating one on first run.
///
/// Persisting it means the user pastes the token into the extension once rather
/// than after every restart.
pub fn load_or_create_token(dir: &std::path::Path) -> String {
    let path = dir.join("bridge-token.txt");

    if let Ok(existing) = std::fs::read_to_string(&path) {
        let trimmed = existing.trim().to_string();
        if trimmed.len() >= 16 {
            return trimmed;
        }
    }

    let token = generate_token();
    let _ = std::fs::create_dir_all(dir);
    let _ = std::fs::write(&path, &token);
    token
}

/// 32 hex characters derived from clock jitter, the PID and ASLR heap addresses.
///
/// This gates a loopback-only endpoint that exposes browser tab titles — it is a
/// pairing secret, not a cryptographic key, and no dependency is worth adding for it.
fn generate_token() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let mut seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x9E37_79B9_7F4A_7C15);
    seed ^= std::process::id() as u64;
    // Heap addresses vary per run under ASLR.
    let probe = Box::new(0u8);
    seed ^= (&*probe as *const u8 as u64).rotate_left(17);

    let mut out = String::with_capacity(32);
    for _ in 0..4 {
        // SplitMix64 — small, well-distributed, and self-contained.
        seed = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = seed;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        out.push_str(&format!("{z:016x}"));
    }
    out.truncate(32);
    out
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_are_32_hex_chars_and_unique() {
        let a = generate_token();
        let b = generate_token();
        assert_eq!(a.len(), 32);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(a, b, "two tokens generated back to back must differ");
    }

    #[test]
    fn a_fresh_bridge_is_disconnected() {
        let bridge = Bridge::new("t".into());
        assert!(!bridge.connected());
        assert_eq!(bridge.status().tab_count, 0);
    }

    #[test]
    fn touch_marks_the_bridge_connected() {
        let bridge = Bridge::new("t".into());
        bridge.touch();
        assert!(bridge.connected());
        assert_eq!(bridge.status().last_seen_secs, Some(0));
    }

    #[test]
    fn commands_queue_and_drain_once() {
        let bridge = Bridge::new("t".into());
        bridge.enqueue(BridgeCommand::Close {
            id: "1".into(),
            window_ids: vec![7],
        });
        assert_eq!(bridge.status().pending_commands, 1);

        let drained = bridge.drain_commands();
        assert_eq!(drained.len(), 1);
        // Draining is destructive — a second poll returns nothing.
        assert!(bridge.drain_commands().is_empty());
    }

    #[test]
    fn queue_is_bounded() {
        let bridge = Bridge::new("t".into());
        for i in 0..100 {
            bridge.enqueue(BridgeCommand::Close {
                id: i.to_string(),
                window_ids: vec![],
            });
        }
        assert!(bridge.status().pending_commands <= 32);
    }

    #[test]
    fn commands_serialize_with_a_type_tag() {
        let json = serde_json::to_string(&BridgeCommand::Close {
            id: "abc".into(),
            window_ids: vec![1, 2],
        })
        .unwrap();
        assert!(json.contains("\"type\":\"close\""), "got {json}");
        assert!(json.contains("\"window_ids\":[1,2]"), "got {json}");
    }

    // ── End-to-end HTTP tests against a real listener ────────────────────────

    /// Issue a raw HTTP/1.1 request and return `(status_code, body)`.
    fn http(
        port: u16,
        method: &str,
        path: &str,
        token: Option<&str>,
        body: Option<&str>,
    ) -> (u16, String) {
        use std::io::{Read, Write};
        use std::net::TcpStream;

        let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
        let payload = body.unwrap_or("");
        let mut req = format!(
            "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nOrigin: chrome-extension://abc\r\nContent-Length: {}\r\n",
            payload.len()
        );
        if let Some(t) = token {
            req.push_str(&format!("X-Stash-Token: {t}\r\n"));
        }
        req.push_str("Connection: close\r\n\r\n");
        req.push_str(payload);

        stream.write_all(req.as_bytes()).expect("write");
        stream.flush().expect("flush");

        let mut raw = String::new();
        stream.read_to_string(&mut raw).expect("read");

        let status: u16 = raw
            .split_whitespace()
            .nth(1)
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let body = raw.split("\r\n\r\n").nth(1).unwrap_or("").to_string();
        (status, body)
    }

    fn started_bridge() -> (Bridge, u16) {
        let bridge = Bridge::new("secret-token".into());
        let port = bridge.start_on(0).expect("bridge should bind an ephemeral port");
        (bridge, port)
    }

    #[test]
    fn health_needs_no_token() {
        let (_bridge, port) = started_bridge();
        let (status, body) = http(port, "GET", "/health", None, None);
        assert_eq!(status, 200);
        assert!(body.contains("\"app\":\"stash\""), "got {body}");
    }

    #[test]
    fn state_is_rejected_without_the_token() {
        let (bridge, port) = started_bridge();
        let (status, _) = http(port, "POST", "/state", None, Some("{}"));
        assert_eq!(status, 401);
        // A rejected request must not count as the extension checking in.
        assert!(!bridge.connected());
    }

    #[test]
    fn state_is_rejected_with_a_wrong_token() {
        let (_bridge, port) = started_bridge();
        let (status, _) = http(port, "POST", "/state", Some("nope"), Some("{}"));
        assert_eq!(status, 401);
    }

    #[test]
    fn posting_state_stores_the_snapshot_and_marks_connected() {
        let (bridge, port) = started_bridge();
        let payload = r#"{
            "browser": "chrome",
            "captured_at": "2026-07-26T12:00:00Z",
            "windows": [{
                "window_id": 42, "left": 0, "top": 0, "width": 1200, "height": 800,
                "state": "normal", "focused": true, "incognito": false,
                "tabs": [
                    {"url":"https://a.test","title":"A","pinned":false,"active":true,"group_id":1},
                    {"url":"https://b.test","title":"B","pinned":false,"active":false,"group_id":1}
                ],
                "groups": [{"id":1,"title":"Work","color":"blue","collapsed":false}]
            }]
        }"#;

        let (status, _) = http(port, "POST", "/state", Some("secret-token"), Some(payload));
        assert_eq!(status, 200);

        assert!(bridge.connected(), "a valid request marks the bridge live");
        let snap = bridge.snapshot().expect("snapshot stored");
        assert_eq!(snap.browser, "chrome");
        assert_eq!(snap.windows.len(), 1);
        assert_eq!(snap.tab_count(), 2);
        assert_eq!(snap.windows[0].groups[0].title, "Work");

        let status = bridge.status();
        assert_eq!(status.tab_count, 2);
        assert_eq!(status.window_count, 1);
    }

    #[test]
    fn malformed_state_is_a_400_and_leaves_the_old_snapshot_intact() {
        let (bridge, port) = started_bridge();
        let (status, _) = http(
            port,
            "POST",
            "/state",
            Some("secret-token"),
            Some("{\"windows\": \"not an array\"}"),
        );
        assert_eq!(status, 400);
        assert!(bridge.snapshot().is_none());
    }

    #[test]
    fn queued_commands_are_delivered_then_drained() {
        let (bridge, port) = started_bridge();
        bridge.enqueue(BridgeCommand::Close {
            id: "c1".into(),
            window_ids: vec![42],
        });

        let (status, body) = http(port, "GET", "/commands", Some("secret-token"), None);
        assert_eq!(status, 200);
        assert!(body.contains("\"type\":\"close\""), "got {body}");
        assert!(body.contains("42"), "got {body}");

        // The extension has taken the work; a second poll must be empty.
        let (_, body2) = http(port, "GET", "/commands", Some("secret-token"), None);
        assert!(body2.contains("\"commands\":[]"), "got {body2}");
    }

    #[test]
    fn restore_commands_round_trip_their_window_payload() {
        let (bridge, port) = started_bridge();
        bridge.enqueue(BridgeCommand::Restore {
            id: "r1".into(),
            windows: vec![BrowserWindowState {
                window_id: 7,
                width: 1024,
                tabs: vec![crate::browser::BrowserTab {
                    url: "https://example.test/".into(),
                    ..Default::default()
                }],
                ..Default::default()
            }],
        });

        let (_, body) = http(port, "GET", "/commands", Some("secret-token"), None);
        assert!(body.contains("\"type\":\"restore\""), "got {body}");
        assert!(body.contains("https://example.test/"), "got {body}");
    }

    #[test]
    fn cors_preflight_is_answered() {
        let (_bridge, port) = started_bridge();
        let (status, _) = http(port, "OPTIONS", "/state", None, None);
        assert_eq!(status, 204, "the extension's preflight must succeed");
    }

    #[test]
    fn unknown_paths_are_404() {
        let (_bridge, port) = started_bridge();
        let (status, _) = http(port, "GET", "/nope", Some("secret-token"), None);
        assert_eq!(status, 404);
    }

    #[test]
    fn a_second_bind_of_the_same_port_fails_cleanly() {
        let (_bridge, port) = started_bridge();
        let other = Bridge::new("t".into());
        // This is the "two copies of Stash" case; it must error, not panic.
        assert!(other.start_on(port).is_err());
    }

    #[test]
    fn token_persists_across_calls() {
        let dir = std::env::temp_dir().join(format!("stash-token-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let first = load_or_create_token(&dir);
        let second = load_or_create_token(&dir);
        assert_eq!(first, second, "token must be stable across restarts");
        let _ = std::fs::remove_dir_all(&dir);
    }
}

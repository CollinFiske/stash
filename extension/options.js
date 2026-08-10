const $ = (id) => document.getElementById(id);

const DEFAULTS = {
  host: "127.0.0.1",
  port: 8787,
  token: "",
  enabled: true,
  discardRestoredTabs: true,
};

function setStatus(message, kind) {
  const el = $("status");
  el.textContent = message;
  el.className = `status ${kind}`;
}

async function load() {
  const cfg = await chrome.storage.local.get(DEFAULTS);
  $("token").value = cfg.token || "";
  $("host").value = cfg.host || DEFAULTS.host;
  $("port").value = cfg.port || DEFAULTS.port;
  $("discard").checked = cfg.discardRestoredTabs !== false;
}

// Applies immediately — it only affects the next restore, so there is nothing
// to save alongside the connection settings.
$("discard").addEventListener("change", async () => {
  await chrome.storage.local.set({ discardRestoredTabs: $("discard").checked });
});

async function save() {
  const token = $("token").value.trim();
  const host = $("host").value.trim() || DEFAULTS.host;
  const port = Number($("port").value) || DEFAULTS.port;

  if (!token) {
    setStatus("Enter the pairing token from the Stash app first.", "err");
    return false;
  }

  await chrome.storage.local.set({ token, host, port, enabled: true });
  await chrome.runtime.sendMessage({ type: "restart" });
  return true;
}

$("save").addEventListener("click", async () => {
  if (!(await save())) return;
  setStatus("Saved. Checking connection…", "ok");
  await test();
});

$("test").addEventListener("click", () => test());

async function test() {
  try {
    const res = await chrome.runtime.sendMessage({ type: "test" });
    if (res && res.ok) {
      setStatus(
        `Connected to Stash ${res.health?.version ?? ""} — browser state sent.`.trim(),
        "ok"
      );
    } else {
      const error = res?.error || "unknown error";
      // Distinguish the three failure modes the user can actually act on.
      if (/401/.test(error)) {
        setStatus("Stash is running but rejected the token. Copy it again from Stash → Browser.", "err");
      } else if (/Failed to fetch|not running/i.test(error)) {
        setStatus("Could not reach Stash. Is the desktop app open?", "err");
      } else {
        setStatus(error, "err");
      }
    }
  } catch (e) {
    setStatus(String(e), "err");
  }
}

load();

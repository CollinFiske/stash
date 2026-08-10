/**
 * Stash Bridge — MV3 service worker.
 *
 * The desktop app cannot call into a browser extension, so the extension drives
 * the conversation against Stash's loopback HTTP server:
 *
 *   push   POST /state     the full window / tab / tab-group picture
 *   poll   GET  /commands  work Stash has queued (restore, close)
 *   ack    POST /ack       what happened
 *
 * MV3 service workers are killed when idle. Two mechanisms keep this alive:
 *   1. an in-worker interval loop, which the fetch traffic itself keeps warm;
 *   2. a `chrome.alarms` tick that restarts the loop if the worker was evicted.
 */

const DEFAULTS = {
  host: "127.0.0.1",
  port: 8787,
  token: "",
  enabled: true,
  /**
   * Unload restored background tabs once they have loaded, to reclaim memory.
   * Turn off to leave every restored tab loaded and ready.
   */
  discardRestoredTabs: true,
};

/** How often to poll Stash for queued commands. */
const POLL_MS = 2000;
/** How often to push browser state even when nothing has changed. */
const PUSH_MS = 5000;
/** Debounce for state pushes triggered by tab activity. */
const CHANGE_DEBOUNCE_MS = 800;

let timers = { poll: null, push: null };
let changeTimer = null;
let lastError = null;
let lastPushAt = 0;

// ── Configuration ─────────────────────────────────────────────────────────────

async function getConfig() {
  const stored = await chrome.storage.local.get(DEFAULTS);
  return { ...DEFAULTS, ...stored };
}

function endpoint(cfg, path) {
  return `http://${cfg.host}:${cfg.port}${path}`;
}

// ── Talking to Stash ──────────────────────────────────────────────────────────

async function callStash(cfg, path, { method = "GET", body } = {}) {
  const res = await fetch(endpoint(cfg, path), {
    method,
    headers: {
      "Content-Type": "application/json",
      "X-Stash-Token": cfg.token,
    },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  if (!res.ok) {
    const detail = await res.text().catch(() => "");
    throw new Error(`${res.status} ${res.statusText} ${detail}`.trim());
  }
  return res.json().catch(() => ({}));
}

/** Health check used by the options page. Deliberately unauthenticated. */
async function checkHealth(cfg) {
  const res = await fetch(endpoint(cfg, "/health"), { method: "GET" });
  if (!res.ok) throw new Error(`Stash replied ${res.status}`);
  return res.json();
}

// ── Reading browser state ─────────────────────────────────────────────────────

/** URLs that cannot be recreated by `chrome.tabs.create`. */
function isRestorable(url) {
  if (!url) return false;
  return !/^(devtools|chrome-untrusted|view-source):/i.test(url);
}

/**
 * Snapshot every normal browser window, its tabs, and its tab groups.
 *
 * Incognito windows are deliberately excluded — capturing private browsing into
 * a file on disk is not a decision an extension should make for the user.
 */
async function collectState() {
  const windows = await chrome.windows.getAll({ populate: true });

  // Tab groups are queried separately; MV3 has no populate option for them.
  let allGroups = [];
  try {
    allGroups = await chrome.tabGroups.query({});
  } catch {
    allGroups = []; // older Chrome, or tabGroups unavailable
  }

  const result = [];

  for (const win of windows) {
    if (win.incognito) continue;
    if (win.type !== "normal") continue; // skip popups and devtools windows

    const tabs = (win.tabs || [])
      .filter((t) => isRestorable(t.url))
      .sort((a, b) => a.index - b.index)
      .map((t) => ({
        url: t.url,
        title: t.title || "",
        pinned: !!t.pinned,
        active: !!t.active,
        group_id: t.groupId != null && t.groupId >= 0 ? t.groupId : null,
      }));

    if (tabs.length === 0) continue;

    const usedGroupIds = new Set(tabs.map((t) => t.group_id).filter((g) => g != null));
    const groups = allGroups
      .filter((g) => g.windowId === win.id && usedGroupIds.has(g.id))
      .map((g) => ({
        id: g.id,
        title: g.title || "",
        color: g.color || "grey",
        collapsed: !!g.collapsed,
      }));

    result.push({
      window_id: win.id,
      focused: !!win.focused,
      incognito: false,
      state: win.state || "normal",
      left: win.left ?? 0,
      top: win.top ?? 0,
      width: win.width ?? 0,
      height: win.height ?? 0,
      tabs,
      groups,
    });
  }

  return {
    browser: detectBrowser(),
    captured_at: new Date().toISOString(),
    windows: result,
  };
}

function detectBrowser() {
  const ua = navigator.userAgent;
  if (/Edg\//.test(ua)) return "edge";
  if (/OPR\//.test(ua)) return "opera";
  if (/Brave/.test(ua)) return "brave";
  if (/Vivaldi/.test(ua)) return "vivaldi";
  return "chrome";
}

async function pushState(cfg) {
  const state = await collectState();
  await callStash(cfg, "/state", { method: "POST", body: state });
  lastPushAt = Date.now();
  lastError = null;
  await chrome.storage.local.set({
    lastPushAt,
    lastWindowCount: state.windows.length,
    lastTabCount: state.windows.reduce((n, w) => n + w.tabs.length, 0),
    lastError: null,
  });
}

// ── Executing Stash's commands ────────────────────────────────────────────────

/**
 * Rebuild browser windows from a saved snapshot.
 *
 * Background tabs are discarded to keep a restored workspace cheap — but only
 * *after* each tab's navigation has committed. See [`waitForCommit`] for why
 * that ordering is not optional.
 */
async function restoreWindows(windowStates, cfg) {
  let restored = 0;

  for (const ws of windowStates || []) {
    const tabs = (ws.tabs || []).filter((t) => isRestorable(t.url));
    if (tabs.length === 0) continue;

    const activeIndex = Math.max(0, tabs.findIndex((t) => t.active));

    // Create the window on its first tab, then fill in the rest in order.
    const created = await chrome.windows.create({
      url: tabs[0].url,
      left: ws.left || undefined,
      top: ws.top || undefined,
      width: ws.width || undefined,
      height: ws.height || undefined,
      focused: false,
    });

    const windowId = created.id;
    const tabIds = [created.tabs[0].id];

    // Every tab is created unpinned and pinned afterwards. Creating a tab with
    // `pinned: true` makes Chrome move it into the pinned strip immediately,
    // which shifts every index after it and scrambles the restored tab order.
    for (let i = 1; i < tabs.length; i++) {
      const t = await chrome.tabs.create({
        windowId,
        url: tabs[i].url,
        index: i,
        active: false,
      });
      tabIds.push(t.id);
    }

    // Pin left-to-right so the pinned tabs keep their relative order.
    for (let i = 0; i < tabs.length; i++) {
      if (!tabs[i].pinned) continue;
      await chrome.tabs.update(tabIds[i], { pinned: true }).catch(() => {});
    }

    await regroupTabs(windowId, tabs, tabIds, ws.groups || []);

    // Maximised / fullscreen cannot be set at the same time as bounds.
    if (ws.state && ws.state !== "normal" && ws.state !== "minimized") {
      await chrome.windows.update(windowId, { state: ws.state }).catch(() => {});
    }

    await chrome.tabs.update(tabIds[activeIndex], { active: true }).catch(() => {});

    if (cfg?.discardRestoredTabs !== false) {
      await discardInactiveTabs(tabIds, tabIds[activeIndex]);
    }

    restored++;
  }

  return restored;
}

/**
 * Resolve once `tabId` has *committed* its navigation, or the timeout expires.
 *
 * `chrome.tabs.create({url})` resolves as soon as the tab object exists — long
 * before the page is fetched. Until the navigation commits, Chrome reports the
 * target in `tab.pendingUrl` and leaves `tab.url` empty, and the tab's session
 * history is still empty.
 *
 * Discarding a tab in that state throws away the only record of where it was
 * pointed, so it comes back as a blank New Tab page instead of the saved URL.
 * That is exactly what happened to every restored background tab before this
 * check existed.
 *
 * Returns true when the tab committed and is therefore safe to discard.
 */
async function waitForCommit(tabId, timeoutMs = 15000) {
  const deadline = Date.now() + timeoutMs;

  while (Date.now() < deadline) {
    let tab;
    try {
      tab = await chrome.tabs.get(tabId);
    } catch {
      return false; // tab closed underneath us
    }

    if (tab.discarded) return false; // already unloaded; nothing to do
    // A committed URL is the signal. `pendingUrl` set with an empty `url`
    // means the navigation is still in flight.
    if (tab.url && tab.url !== "chrome://newtab/") return true;
    if (tab.status === "complete" && !tab.pendingUrl) return true;

    await sleep(150);
  }
  return false;
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

/** Recreate the tab groups a window had, preserving title, colour and collapse. */
async function regroupTabs(windowId, tabs, tabIds, groups) {
  const byOriginalGroup = new Map();
  tabs.forEach((t, i) => {
    if (t.group_id == null) return;
    if (!byOriginalGroup.has(t.group_id)) byOriginalGroup.set(t.group_id, []);
    byOriginalGroup.get(t.group_id).push(tabIds[i]);
  });

  for (const [originalId, ids] of byOriginalGroup) {
    try {
      const newGroupId = await chrome.tabs.group({
        tabIds: ids,
        createProperties: { windowId },
      });
      const meta = groups.find((g) => g.id === originalId);
      if (meta) {
        await chrome.tabGroups.update(newGroupId, {
          title: meta.title || "",
          color: meta.color || "grey",
          collapsed: !!meta.collapsed,
        });
      }
    } catch (e) {
      console.warn("[stash] could not regroup tabs", e);
    }
  }
}

/**
 * Unload every tab except the active one, so a restored workspace costs little RAM.
 *
 * A tab is only discarded once its navigation has committed — discarding earlier
 * loses the URL. Tabs that never commit (offline, very slow, or a page that hangs)
 * are deliberately left loaded: a loaded tab is a far better outcome than a blank one.
 */
async function discardInactiveTabs(tabIds, activeTabId) {
  const candidates = tabIds.filter((id) => id !== activeTabId);

  // Wait in parallel — Chrome throttles concurrent loads on its own, and doing
  // this serially would take the sum of every page's load time.
  const commits = await Promise.all(candidates.map((id) => waitForCommit(id)));

  for (let i = 0; i < candidates.length; i++) {
    if (!commits[i]) continue; // never committed — leave it alone
    try {
      await chrome.tabs.discard(candidates[i]);
    } catch {
      // Active, already discarded, or closed. Harmless.
    }
  }
}

async function closeWindows(windowIds) {
  let closed = 0;
  for (const id of windowIds || []) {
    try {
      await chrome.windows.remove(id);
      closed++;
    } catch {
      // Window was already gone.
    }
  }
  return closed;
}

async function handleCommand(cfg, cmd) {
  let result = { id: cmd.id, type: cmd.type, ok: true };
  try {
    if (cmd.type === "restore") {
      result.restored = await restoreWindows(cmd.windows, cfg);
    } else if (cmd.type === "close") {
      result.closed = await closeWindows(cmd.window_ids);
    } else {
      result = { id: cmd.id, type: cmd.type, ok: false, error: "unknown command" };
    }
  } catch (e) {
    result = { id: cmd.id, type: cmd.type, ok: false, error: String(e) };
    console.error("[stash] command failed", cmd.type, e);
  }
  await callStash(cfg, "/ack", { method: "POST", body: result }).catch(() => {});
  // Reflect the new reality straight away rather than waiting for the heartbeat.
  await pushState(cfg).catch(() => {});
}

// ── Loops ─────────────────────────────────────────────────────────────────────

async function pollOnce() {
  const cfg = await getConfig();
  if (!cfg.enabled || !cfg.token) return;

  try {
    const { commands } = await callStash(cfg, "/commands");
    for (const cmd of commands || []) {
      await handleCommand(cfg, cmd);
    }
    lastError = null;
  } catch (e) {
    await noteError(e);
  }
}

async function pushOnce() {
  const cfg = await getConfig();
  if (!cfg.enabled || !cfg.token) return;
  try {
    await pushState(cfg);
  } catch (e) {
    await noteError(e);
  }
}

async function noteError(e) {
  const message = String(e && e.message ? e.message : e);
  // "Failed to fetch" just means Stash is not running — not worth surfacing.
  lastError = /Failed to fetch|NetworkError/i.test(message)
    ? "Stash is not running"
    : message;
  await chrome.storage.local.set({ lastError });
}

function startLoops() {
  stopLoops();
  timers.poll = setInterval(pollOnce, POLL_MS);
  timers.push = setInterval(pushOnce, PUSH_MS);
  pollOnce();
  pushOnce();
}

function stopLoops() {
  if (timers.poll) clearInterval(timers.poll);
  if (timers.push) clearInterval(timers.push);
  timers = { poll: null, push: null };
}

/** Push immediately when the user changes something, coalescing bursts. */
function onBrowserChanged() {
  if (changeTimer) clearTimeout(changeTimer);
  changeTimer = setTimeout(() => {
    changeTimer = null;
    pushOnce();
  }, CHANGE_DEBOUNCE_MS);
}

// ── Wiring ────────────────────────────────────────────────────────────────────

for (const event of [
  chrome.tabs.onCreated,
  chrome.tabs.onRemoved,
  chrome.tabs.onUpdated,
  chrome.tabs.onMoved,
  chrome.tabs.onAttached,
  chrome.windows.onCreated,
  chrome.windows.onRemoved,
]) {
  event.addListener(onBrowserChanged);
}

if (chrome.tabGroups) {
  chrome.tabGroups.onCreated.addListener(onBrowserChanged);
  chrome.tabGroups.onUpdated.addListener(onBrowserChanged);
  chrome.tabGroups.onRemoved.addListener(onBrowserChanged);
}

// The alarm resurrects the loops if the service worker was evicted while idle.
chrome.alarms.create("stash-keepalive", { periodInMinutes: 0.5 });
chrome.alarms.onAlarm.addListener((alarm) => {
  if (alarm.name === "stash-keepalive") startLoops();
});

chrome.runtime.onStartup.addListener(startLoops);
chrome.runtime.onInstalled.addListener(startLoops);

// Popup and options page ask for status / trigger actions.
chrome.runtime.onMessage.addListener((msg, _sender, sendResponse) => {
  (async () => {
    const cfg = await getConfig();
    try {
      if (msg.type === "status") {
        const stored = await chrome.storage.local.get([
          "lastPushAt",
          "lastWindowCount",
          "lastTabCount",
          "lastError",
        ]);
        sendResponse({ ok: true, config: cfg, ...stored });
      } else if (msg.type === "test") {
        const health = await checkHealth(cfg);
        // A successful authenticated push proves the token is right too.
        await pushState(cfg);
        sendResponse({ ok: true, health });
      } else if (msg.type === "pushNow") {
        await pushState(cfg);
        sendResponse({ ok: true });
      } else if (msg.type === "restart") {
        startLoops();
        sendResponse({ ok: true });
      } else {
        sendResponse({ ok: false, error: "unknown message" });
      }
    } catch (e) {
      sendResponse({ ok: false, error: String(e && e.message ? e.message : e) });
    }
  })();
  return true; // keep the channel open for the async reply
});

startLoops();

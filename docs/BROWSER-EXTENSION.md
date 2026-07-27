# Stash Bridge — Browser Extension Setup

The Stash desktop app can see that Chrome is open and where its windows sit. It
cannot see a single tab — no operating system exposes that. **Stash Bridge** is a
small Chrome extension that fills the gap, so a snapshot captures *what you were
reading*, not just *that a browser was running*.

With it installed, Stash can:

- capture every window, tab, pinned tab and **tab group** (name, colour, collapsed state),
- close a workspace's browser windows without touching your other browsing,
- reopen them exactly as they were — with background tabs left **unloaded**, so
  restoring a 40-tab workspace costs almost no memory until you click a tab.

---

## Part 1 — Install the extension (2 minutes)

1. **Start Stash.** The bridge only listens while the desktop app is running.

2. **Open the extensions page.** In Chrome, go to `chrome://extensions`.
   *(Edge: `edge://extensions` · Brave: `brave://extensions`)*

3. **Turn on Developer mode** using the toggle in the top-right corner.

4. **Click "Load unpacked"** and select the `extension/` folder inside the Stash
   repository:

   ```
   c:\Users\cfisk\OneDrive\Documents\prjs\stash\extension
   ```

   Select the **folder itself** — not a file inside it.

5. Chrome adds **Stash Bridge** to the list. Pin it to the toolbar if you like:
   click the puzzle-piece icon → pin.

---

## Part 2 — Pair it with Stash (1 minute)

The bridge is protected by a pairing token. Anything on your machine can reach
`127.0.0.1`, so without a token any web page you visit could ask Stash for your
tab list. The token stops that.

1. In **Stash**, click the **Browser** button in the header.
   The panel opens showing a **Pairing Token** and a **Copy** button.

2. Click **Copy**.

3. Back in Chrome, open the extension's settings:
   - right-click the Stash Bridge icon → **Options**, or
   - click the icon → **Settings**.

4. Paste the token into **Pairing token** and press **Save & Connect**.

5. You should see **"Connected to Stash — browser state sent."**

Switch back to Stash. The **Browser** button now shows a tab count, and the panel
turns green with a live list of your browser windows and tab groups.

> The token is stored in Stash's app-data folder, so it stays the same across
> restarts. You only do this once.

---

## Part 3 — Using it

### Capturing tabs

Take a snapshot as normal (**Snapshot** in the header). Browser windows now show
what they contain:

```
Google Chrome                          tabs saved
12 tabs · Research, Docs
```

Each Chrome window on screen is matched to its tab list by comparing window
geometry, so three Chrome windows become three separate entries with the right
tabs in each — not one lump.

### Restoring tabs

Press **Launch** on a saved workspace. Instead of starting `chrome.exe` on a
single URL, Stash hands the whole window description to the extension, which
rebuilds it: correct window position and size, tabs in their original order,
pinned tabs still pinned, and tab groups restored with their names and colours.

Background tabs are **discarded immediately** — they appear in the tab strip but
consume no memory until you click one.

### Closing tabs

**Close Apps** on an active workspace closes exactly the browser windows in that
workspace through the extension. Chrome windows that belong to a different
workspace are left alone.

---

## What is shared, and what is not

**Sent to Stash** (over `127.0.0.1` only, never off your machine):

- window position, size and state
- tab URLs, titles, pinned flag, active flag
- tab group names, colours and collapsed state

**Never captured:**

- **Incognito windows** — excluded outright by the extension
- page contents, cookies, passwords, form data, history
- `devtools://`, `chrome-untrusted://` and `view-source:` tabs, which cannot be recreated anyway

There is no outbound network traffic. The extension talks to `127.0.0.1:8787`
and nothing else — that is the only host in its `host_permissions`.

---

## How it works

A Chrome extension cannot be called into from outside the browser, so the
extension is the client and Stash is the server:

```
┌──────────────────────┐                    ┌────────────────────────┐
│  Stash Bridge        │                    │  Stash (Rust)          │
│  service worker      │                    │  127.0.0.1:8787        │
│                      │                    │                        │
│  every ~5s ──────────┼── POST /state ────▶│  stores the snapshot   │
│  on tab change ──────┼── POST /state ────▶│  (debounced 800ms)     │
│  every ~2s ──────────┼── GET /commands ──▶│  drains the work queue │
│                      │◀── restore/close ──┤                        │
│  runs the command    ├── POST /ack ──────▶│                        │
└──────────────────────┘                    └────────────────────────┘
```

| Endpoint | Auth | Purpose |
|---|---|---|
| `GET /health` | none | Lets the options page distinguish "Stash isn't running" from "wrong token" |
| `POST /state` | token | The extension pushes its full window/tab/group picture |
| `GET /commands` | token | The extension collects queued work; the queue drains on read |
| `POST /ack` | token | The extension reports what happened |

MV3 service workers get evicted when idle, so the extension keeps two safety
nets: an in-worker interval loop (kept warm by its own traffic) and a
`chrome.alarms` tick every 30 seconds that restarts the loop if Chrome killed it.

---

## Troubleshooting

**The Browser panel says "Offline"**
Port 8787 is already in use — almost always a second copy of Stash. Quit the
other instance (tray icon → Quit Stash) and restart.

**The panel says "Waiting" and never turns green**
Stash is listening but the extension has not checked in. Check, in order:
1. Is the extension enabled at `chrome://extensions`?
2. Did you press **Save & Connect** after pasting the token?
3. Click the extension icon — the popup shows the exact error.

**"Stash is running but rejected the token"**
The token was copied incompletely or Stash's app-data folder was cleared. Copy
it again from Stash → **Browser** and re-save.

**"Could not reach Stash"**
The desktop app is not running, or a firewall is blocking loopback connections
for Chrome. Stash must be open for the bridge to answer.

**Tabs restore but tab groups do not**
`chrome.tabGroups` needs Chrome 89+. On older builds tabs still restore in the
right order and window; only the coloured group pills are lost.

**Some tabs are missing after a restore**
`devtools://` and `view-source:` tabs are filtered out at capture time — Chrome
does not allow an extension to recreate them.

**The extension stops syncing after Chrome has been idle for a long time**
Click the extension icon and press **Sync now**; this also restarts the loop.
The 30-second alarm normally recovers this on its own.

---

## Updating the extension

After pulling changes to `extension/`, go to `chrome://extensions` and click the
**reload** (↻) icon on the Stash Bridge card. The token is preserved.

## Other browsers

Anything Chromium-based works with the same folder: **Edge**, **Brave**,
**Opera**, **Vivaldi** and **Arc**. Load it from that browser's own extensions
page — each browser keeps its own extension list and its own copy of the setting,
so paste the same token into each one.

**Firefox is not supported.** It has no `chrome.tabGroups` API, and MV3 support
differs enough that the service worker would need rewriting.

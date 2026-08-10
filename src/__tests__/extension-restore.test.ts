/**
 * Regression tests for the Stash Bridge extension's restore path.
 *
 * These load `extension/background.js` into a sandbox with a fake `chrome` API
 * that models Chrome's real tab lifecycle: a tab created with a URL does not
 * have that URL yet. Chrome reports the target in `pendingUrl` and leaves `url`
 * empty until the navigation *commits*.
 *
 * That distinction is the whole point. Discarding a tab before it commits throws
 * away the only record of where it pointed, and it comes back as a blank New Tab
 * page — which is exactly what happened to restored background tabs.
 */

import { describe, it, expect, beforeEach, vi } from "vitest";
// Loaded as a string via Vite's `?raw`, so the extension's real source is what
// gets exercised — not a copy that could drift away from it.
import backgroundSource from "../../extension/background.js?raw";

// ── Fake Chrome ───────────────────────────────────────────────────────────────

interface FakeTab {
  id: number;
  windowId: number;
  index: number;
  url: string;
  pendingUrl?: string;
  status: "loading" | "complete";
  pinned: boolean;
  active: boolean;
  discarded: boolean;
  groupId: number;
}

/** How many `chrome.tabs.get` polls a tab takes before its navigation commits. */
const POLLS_UNTIL_COMMIT = 3;

class FakeChrome {
  tabs: FakeTab[] = [];
  nextTabId = 1;
  nextWindowId = 100;
  nextGroupId = 900;

  /** Remaining polls before each tab commits. */
  private pollsLeft = new Map<number, number>();

  /** Every discard attempt, with whether the tab had committed at that moment. */
  discardLog: { tabId: number; hadCommitted: boolean }[] = [];

  groupUpdates: { groupId: number; props: Record<string, unknown> }[] = [];

  private get(tabId: number): FakeTab | undefined {
    return this.tabs.find((t) => t.id === tabId);
  }

  private makeTab(windowId: number, url: string, index: number, active: boolean): FakeTab {
    const tab: FakeTab = {
      id: this.nextTabId++,
      windowId,
      index,
      // Chrome has NOT committed the navigation yet.
      url: "",
      pendingUrl: url,
      status: "loading",
      pinned: false,
      active,
      discarded: false,
      groupId: -1,
    };
    this.tabs.push(tab);
    this.pollsLeft.set(tab.id, POLLS_UNTIL_COMMIT);
    return tab;
  }

  api = {
    tabs: {
      create: async (props: Record<string, unknown>): Promise<FakeTab> =>
        this.makeTab(
          props.windowId as number,
          props.url as string,
          props.index as number,
          !!props.active
        ),

      get: async (tabId: number): Promise<FakeTab> => {
        const tab = this.get(tabId);
        if (!tab) throw new Error(`no tab ${tabId}`);
        // Each poll advances the tab toward committing, the way a real page load does.
        const left = (this.pollsLeft.get(tabId) ?? 0) - 1;
        this.pollsLeft.set(tabId, left);
        if (left <= 0 && tab.pendingUrl) {
          tab.url = tab.pendingUrl;
          delete tab.pendingUrl;
          tab.status = "complete";
        }
        return { ...tab };
      },

      update: async (tabId: number, props: Record<string, unknown>): Promise<FakeTab> => {
        const tab = this.get(tabId);
        if (!tab) throw new Error(`no tab ${tabId}`);
        Object.assign(tab, props);
        return { ...tab };
      },

      discard: async (tabId: number): Promise<void> => {
        const tab = this.get(tabId);
        if (!tab) throw new Error(`no tab ${tabId}`);
        this.discardLog.push({ tabId, hadCommitted: tab.url !== "" });
        tab.discarded = true;
        // Chrome keeps a discarded tab's committed URL — and only that.
        // Anything discarded pre-commit loses its destination for good.
        if (tab.url === "") delete tab.pendingUrl;
      },

      group: async (props: { tabIds: number[] }): Promise<number> => {
        const groupId = this.nextGroupId++;
        for (const id of props.tabIds) {
          const tab = this.get(id);
          if (tab) tab.groupId = groupId;
        }
        return groupId;
      },

      onCreated: { addListener: () => {} },
      onRemoved: { addListener: () => {} },
      onUpdated: { addListener: () => {} },
      onMoved: { addListener: () => {} },
      onAttached: { addListener: () => {} },
    },

    tabGroups: {
      query: async () => [],
      update: async (groupId: number, props: Record<string, unknown>) => {
        this.groupUpdates.push({ groupId, props });
        return { id: groupId, ...props };
      },
      onCreated: { addListener: () => {} },
      onUpdated: { addListener: () => {} },
      onRemoved: { addListener: () => {} },
    },

    windows: {
      create: async (props: Record<string, unknown>) => {
        const windowId = this.nextWindowId++;
        const first = this.makeTab(windowId, props.url as string, 0, false);
        return { id: windowId, tabs: [first] };
      },
      update: async () => {},
      remove: async () => {},
      getAll: async () => [],
      onCreated: { addListener: () => {} },
      onRemoved: { addListener: () => {} },
    },

    storage: {
      local: {
        get: async (defaults: Record<string, unknown>) => ({ ...defaults, token: "test-token" }),
        set: async () => {},
      },
    },

    alarms: { create: () => {}, onAlarm: { addListener: () => {} } },
    runtime: {
      onStartup: { addListener: () => {} },
      onInstalled: { addListener: () => {} },
      onMessage: { addListener: () => {} },
      openOptionsPage: () => {},
    },
  };
}

/** Load background.js in a sandbox and hand back the functions under test. */
function loadBackground(chrome: FakeChrome) {
  const factory = new Function(
    "chrome",
    "navigator",
    "fetch",
    "console",
    `${backgroundSource}
     return { restoreWindows, discardInactiveTabs, waitForCommit, isRestorable };`
  );
  return factory(
    chrome.api,
    { userAgent: "Chrome/120" },
    // The loops start on load; keep them from touching the network.
    () => Promise.reject(new Error("offline in tests")),
    { log: () => {}, warn: () => {}, error: () => {} }
  );
}

function windowState(urls: string[], opts: Partial<{ activeIndex: number; pinned: number[]; groupOf: (number | null)[]; groups: unknown[] }> = {}) {
  const activeIndex = opts.activeIndex ?? 0;
  return {
    window_id: 1,
    focused: true,
    incognito: false,
    state: "normal",
    left: 0,
    top: 0,
    width: 1200,
    height: 800,
    tabs: urls.map((url, i) => ({
      url,
      title: `Tab ${i}`,
      pinned: (opts.pinned ?? []).includes(i),
      active: i === activeIndex,
      group_id: opts.groupOf ? opts.groupOf[i] : null,
    })),
    groups: opts.groups ?? [],
  };
}

// ── Tests ─────────────────────────────────────────────────────────────────────

describe("extension restore — tab URLs survive", () => {
  let chrome: FakeChrome;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  let bg: any;

  beforeEach(() => {
    vi.stubGlobal("setInterval", () => 0);
    vi.stubGlobal("clearInterval", () => {});
    chrome = new FakeChrome();
    bg = loadBackground(chrome);
  });

  /**
   * The reported bug: three tabs, Gmail active. After restore only Gmail had a
   * URL; Docs and Slides came back as blank New Tab pages.
   */
  it("never discards a tab before its navigation commits", async () => {
    const ws = windowState(
      [
        "https://docs.google.com/document/d/abc",
        "https://docs.google.com/presentation/d/xyz",
        "https://mail.google.com/mail/u/0",
      ],
      { activeIndex: 2 }
    );

    await bg.restoreWindows([ws], { discardRestoredTabs: true });

    expect(chrome.discardLog.length).toBeGreaterThan(0);
    for (const entry of chrome.discardLog) {
      expect(
        entry.hadCommitted,
        `tab ${entry.tabId} was discarded before its navigation committed`
      ).toBe(true);
    }
  });

  it("leaves every restored tab pointing at its saved URL", async () => {
    const urls = [
      "https://docs.google.com/document/d/abc",
      "https://docs.google.com/presentation/d/xyz",
      "https://mail.google.com/mail/u/0",
    ];
    await bg.restoreWindows([windowState(urls, { activeIndex: 2 })], {
      discardRestoredTabs: true,
    });

    const restored = chrome.tabs.map((t) => t.url || t.pendingUrl || "");
    expect(restored).toEqual(urls);
    // Nothing may be left blank — a blank tab is the failure the user reported.
    expect(restored.filter((u) => u === "")).toHaveLength(0);
  });

  it("still discards background tabs, so memory is actually reclaimed", async () => {
    await bg.restoreWindows(
      [windowState(["https://a.test/", "https://b.test/", "https://c.test/"], { activeIndex: 0 })],
      { discardRestoredTabs: true }
    );

    const discarded = chrome.tabs.filter((t) => t.discarded);
    expect(discarded).toHaveLength(2); // the two background tabs
    expect(chrome.tabs.find((t) => t.active)?.discarded).toBe(false);
  });

  it("honours the opt-out and leaves tabs loaded", async () => {
    await bg.restoreWindows(
      [windowState(["https://a.test/", "https://b.test/"], { activeIndex: 0 })],
      { discardRestoredTabs: false }
    );

    expect(chrome.discardLog).toHaveLength(0);
    expect(chrome.tabs.every((t) => !t.discarded)).toBe(true);
  });

  it("defaults to discarding when no config is supplied", async () => {
    await bg.restoreWindows([windowState(["https://a.test/", "https://b.test/"])], undefined);
    expect(chrome.discardLog.length).toBeGreaterThan(0);
  });
});

describe("extension restore — ordering and grouping", () => {
  let chrome: FakeChrome;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  let bg: any;

  beforeEach(() => {
    vi.stubGlobal("setInterval", () => 0);
    vi.stubGlobal("clearInterval", () => {});
    chrome = new FakeChrome();
    bg = loadBackground(chrome);
  });

  it("keeps tab order when a later tab is pinned", async () => {
    // Creating a tab with pinned:true moves it into the pinned strip immediately,
    // which shifts every index after it. Pinning must happen after creation.
    const urls = ["https://a.test/", "https://b.test/", "https://c.test/"];
    await bg.restoreWindows([windowState(urls, { pinned: [2] })], {
      discardRestoredTabs: false,
    });

    const order = chrome.tabs.map((t) => t.url || t.pendingUrl);
    expect(order).toEqual(urls);
    expect(chrome.tabs[2].pinned).toBe(true);
    expect(chrome.tabs[0].pinned).toBe(false);
  });

  it("restores a tab group with its title and colour", async () => {
    const ws = windowState(["https://a.test/", "https://b.test/", "https://c.test/"], {
      groupOf: [7, 7, null],
      groups: [{ id: 7, title: "Work", color: "blue", collapsed: false }],
    });

    await bg.restoreWindows([ws], { discardRestoredTabs: false });

    expect(chrome.groupUpdates).toHaveLength(1);
    expect(chrome.groupUpdates[0].props).toMatchObject({ title: "Work", color: "blue" });
    // The two grouped tabs share a group; the third does not.
    expect(chrome.tabs[0].groupId).toBe(chrome.tabs[1].groupId);
    expect(chrome.tabs[2].groupId).toBe(-1);
  });

  it("skips tabs Chrome cannot recreate", async () => {
    expect(bg.isRestorable("https://example.test/")).toBe(true);
    expect(bg.isRestorable("devtools://devtools/bundled/x.html")).toBe(false);
    expect(bg.isRestorable("view-source:https://example.test/")).toBe(false);
    expect(bg.isRestorable("")).toBe(false);
  });
});

import { describe, it, expect } from "vitest";
import { baseName, exeBase, hostOf, tabSummary, groupTitles } from "../paths";
import type { BrowserWindowState } from "../types";

describe("baseName", () => {
  it("takes the last segment of a Windows path", () => {
    expect(baseName("C:\\Users\\me\\Projects\\stash")).toBe("stash");
  });

  it("takes the last segment of a POSIX path", () => {
    expect(baseName("/home/me/projects/stash")).toBe("stash");
  });

  it("ignores trailing separators", () => {
    expect(baseName("C:\\Projects\\app\\")).toBe("app");
    expect(baseName("/home/me/app/")).toBe("app");
  });

  it("returns the input when there is no separator", () => {
    expect(baseName("stash")).toBe("stash");
  });

  it("handles a bare drive root without crashing", () => {
    expect(baseName("C:\\")).toBe("C:");
  });
});

describe("exeBase", () => {
  it("strips the directory and the .exe extension", () => {
    expect(exeBase("C:\\Program Files\\Microsoft VS Code\\Code.exe")).toBe("code");
  });

  it("is case insensitive", () => {
    expect(exeBase("C:\\CHROME.EXE")).toBe("chrome");
  });

  it("leaves extensionless POSIX binaries alone", () => {
    expect(exeBase("/usr/bin/firefox")).toBe("firefox");
  });

  it("passes through a bare command name", () => {
    // Templates let users type `code` instead of a full path.
    expect(exeBase("code")).toBe("code");
  });

  it("matches an absolute path against a bare command", () => {
    // This equivalence is what makes active-detection work for templates.
    expect(exeBase("C:\\Apps\\Code.exe")).toBe(exeBase("code"));
  });
});

describe("hostOf", () => {
  it("extracts the hostname", () => {
    expect(hostOf("https://github.com/anthropics/stash/pulls")).toBe("github.com");
  });

  it("returns the input for an unparseable string", () => {
    expect(hostOf("not a url")).toBe("not a url");
  });

  it("handles localhost with a port", () => {
    expect(hostOf("http://localhost:1420/index.html")).toBe("localhost");
  });
});

// ── Browser window summaries ──────────────────────────────────────────────────

function makeWindow(tabCount: number, groupTitleList: string[] = []): BrowserWindowState {
  return {
    window_id: 1,
    focused: false,
    incognito: false,
    state: "normal",
    left: 0,
    top: 0,
    width: 1200,
    height: 800,
    tabs: Array.from({ length: tabCount }, (_, i) => ({
      url: `https://example.com/${i}`,
      title: `Tab ${i}`,
      pinned: false,
      active: i === 0,
      group_id: null,
    })),
    groups: groupTitleList.map((title, i) => ({
      id: i,
      title,
      color: "blue",
      collapsed: false,
    })),
  };
}

describe("tabSummary", () => {
  it("uses the singular for one tab", () => {
    expect(tabSummary(makeWindow(1))).toBe("1 tab");
  });

  it("uses the plural for several tabs", () => {
    expect(tabSummary(makeWindow(12))).toBe("12 tabs");
  });

  it("uses the plural for zero tabs", () => {
    expect(tabSummary(makeWindow(0))).toBe("0 tabs");
  });
});

describe("groupTitles", () => {
  it("lists named groups", () => {
    expect(groupTitles(makeWindow(3, ["Docs", "PRs"]))).toEqual(["Docs", "PRs"]);
  });

  it("drops untitled groups so the UI does not render blank pills", () => {
    expect(groupTitles(makeWindow(3, ["Docs", "", "   "]))).toEqual(["Docs"]);
  });

  it("returns an empty list when there are no groups", () => {
    expect(groupTitles(makeWindow(2))).toEqual([]);
  });
});

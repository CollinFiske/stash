import { describe, it, expect } from "vitest";
import { getAppIcon } from "../appicons";

// Every returned value must be a valid SVG string
function isSvg(s: string) {
  return s.startsWith("<svg") && s.includes("</svg>");
}

describe("getAppIcon – always returns SVG", () => {
  const cases: [string, string, boolean, boolean][] = [
    ["Code",       "C:\\Code.exe",     true,  false],
    ["chrome",     "chrome.exe",       false, true ],
    ["firefox",    "firefox.exe",      false, true ],
    ["msedge",     "msedge.exe",       false, true ],
    ["Slack",      "slack.exe",        false, false],
    ["Spotify",    "spotify.exe",      false, false],
    ["Discord",    "discord.exe",      false, false],
    ["Notion",     "notion.exe",       false, false],
    ["Figma",      "figma.exe",        false, false],
    ["Zoom",       "zoom.exe",         false, false],
    ["Outlook",    "outlook.exe",      false, false],
    ["MATLAB",     "matlab.exe",       false, false],
    ["UnknownApp", "random_app.exe",   false, false],
    ["",           "",                 false, false], // edge: empty strings
  ];

  for (const [name, path, isVscode, isBrowser] of cases) {
    it(`returns SVG for ${name || "(empty)"}`, () => {
      const svg = getAppIcon(name, path, isVscode, isBrowser);
      expect(isSvg(svg)).toBe(true);
    });
  }
});

describe("getAppIcon – brand-specific icons", () => {
  it("VS Code icon contains #007ACC (brand blue)", () => {
    const svg = getAppIcon("Code", "Code.exe", true, false);
    expect(svg).toContain("#007ACC");
  });

  it("Chrome icon contains four brand colors", () => {
    const svg = getAppIcon("chrome", "chrome.exe", false, true);
    expect(svg).toContain("#EA4335"); // red
    expect(svg).toContain("#FBBC05"); // yellow
    expect(svg).toContain("#34A853"); // green
    expect(svg).toContain("#4285F4"); // blue
  });

  it("Slack icon contains #E01E5A (brand red-pink)", () => {
    const svg = getAppIcon("Slack", "slack.exe", false, false);
    expect(svg).toContain("#E01E5A");
  });

  it("Spotify icon contains #1DB954 (brand green)", () => {
    const svg = getAppIcon("Spotify", "spotify.exe", false, false);
    expect(svg).toContain("#1DB954");
  });

  it("Figma icon contains multiple brand colors", () => {
    const svg = getAppIcon("Figma", "figma.exe", false, false);
    expect(svg).toContain("#F24E1E");
  });

  it("Edge icon is different from Chrome icon", () => {
    const chrome = getAppIcon("chrome", "chrome.exe", false, true);
    const edge = getAppIcon("msedge", "msedge.exe", false, true);
    expect(chrome).not.toBe(edge);
  });

  it("path-based detection: Chrome path → Chrome icon", () => {
    const svg = getAppIcon("Google Chrome", "C:/Program Files/Google/Chrome/Application/chrome.exe", false, true);
    expect(svg).toContain("#4285F4");
  });

  it("fallback uses first letter as uppercase", () => {
    const svg = getAppIcon("zypherx", "zypherx.exe", false, false);
    expect(svg).toContain("Z");
  });
});

describe("getAppIcon – path-based detection", () => {
  it("detects VS Code from full Windows path", () => {
    const svg = getAppIcon("Code", "C:\\Users\\user\\AppData\\Local\\Programs\\Microsoft VS Code\\Code.exe", true, false);
    expect(isSvg(svg)).toBe(true);
    expect(svg).toContain("#007ACC");
  });

  it("detects Slack from path", () => {
    const svg = getAppIcon("Slack", "C:\\Users\\user\\AppData\\Local\\slack\\slack.exe", false, false);
    expect(svg).toContain("#E01E5A");
  });

  it("detects Discord from path", () => {
    const svg = getAppIcon("Update", "C:\\Users\\user\\AppData\\Local\\Discord\\discord.exe", false, false);
    expect(svg).toContain("#5865F2");
  });
});

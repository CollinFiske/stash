function letter(text: string, bg: string, fg = "#fff"): string {
  const multi = text.length > 1;
  const safe = text.replace(/</g, "&lt;").replace(/>/g, "&gt;");
  return `<svg viewBox="0 0 24 24"><rect width="24" height="24" rx="5" fill="${bg}"/><text x="12" y="${multi ? "15.5" : "16.5"}" text-anchor="middle" font-size="${multi ? "9" : "12"}" font-weight="700" font-family="system-ui,sans-serif" fill="${fg}">${safe}</text></svg>`;
}

const ICONS: Record<string, string> = {
  vscode: `<svg viewBox="0 0 24 24"><path d="M17.58 1.5 9.39 8.82 5.27 5.7a.71.71 0 0 0-.9.04L2.76 7.17a.71.71 0 0 0 0 1.01L5.92 12l-3.16 3.82a.71.71 0 0 0 0 1.01l1.61 1.43c.24.22.62.22.9.04l4.12-3.12 8.19 7.32c.36.33.9.31 1.24.01l3.46-1.67A1.1 1.1 0 0 0 23 19.5v-15a1.1 1.1 0 0 0-.63-1l-3.46-1.67a1.1 1.1 0 0 0-1.33.17zm-1.08 13.08-6.14-4.58 6.14-4.57v9.15z" fill="#007ACC"/></svg>`,
  chrome: `<svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="11.5" fill="white"/><path d="M12 3A9 9 0 0 1 19.8 16.5L16.3 14.5A5 5 0 0 0 12 7Z" fill="#EA4335"/><path d="M19.8 16.5A9 9 0 0 1 4.2 16.5L7.7 14.5A5 5 0 0 0 16.3 14.5Z" fill="#FBBC05"/><path d="M4.2 16.5A9 9 0 0 1 12 3L12 7A5 5 0 0 0 7.7 14.5Z" fill="#34A853"/><circle cx="12" cy="12" r="4" fill="white"/><circle cx="12" cy="12" r="3" fill="#4285F4"/></svg>`,
  firefox: `<svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="9" fill="#0064B9"/><path d="M21 9c-1-4-5-7-9-7-1 0-2 .1-3 .4 3.5.6 6 3 6.5 6l.5-.4c2 1.5 3 4 2.5 6.5A9 9 0 0 0 21 9z" fill="#FF7139"/><path d="M15.5 8.4C15 5.4 12.5 3 9 2.4 6 3.2 3.7 5.5 3 8.5c.5-.3 1-.5 2-.5 3 0 5 2 5 5 0 1.5-.5 3-1.5 4 1-.3 2-.8 3-1.5l.5-.4c2-1.5 3-4 3-6.5l-.5-.2z" fill="#FF9500"/></svg>`,
  safari: `<svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="11" fill="#007AFF"/><polygon points="12,4 14,11.5 12,13 10,11.5" fill="#FF3B30"/><polygon points="12,20 10,12.5 12,11 14,12.5" fill="white"/></svg>`,
  edge: `<svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="11" fill="#0078D4"/><path d="M5 12a7 7 0 0 1 7-7c3 0 5.5 1.8 6.5 4.5H7a5 5 0 0 0 5 7c2 0 3.5-.9 4.5-2.2C15 16.1 13 17.5 10.5 17.5A7.5 7.5 0 0 1 5 12z" fill="#50E6FF"/></svg>`,
  brave: `<svg viewBox="0 0 24 24"><rect width="24" height="24" rx="5" fill="#F85B2A"/><text x="12" y="16.5" text-anchor="middle" font-size="12" font-weight="700" font-family="system-ui,sans-serif" fill="white">B</text></svg>`,
  opera: `<svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="11" fill="#FF1B2D"/><ellipse cx="12" cy="12" rx="4.5" ry="7" fill="none" stroke="white" stroke-width="2"/></svg>`,
  slack: `<svg viewBox="0 0 24 24"><path d="M5.8 15.4a2 2 0 1 1-2-2h2v2zm1 0a2 2 0 0 1 4 0v5a2 2 0 0 1-4 0v-5z" fill="#E01E5A"/><path d="M8.6 5.8a2 2 0 1 1 2-2v2h-2zm0 1a2 2 0 0 1 0 4H3.6a2 2 0 0 1 0-4h5z" fill="#36C5F0"/><path d="M18.2 8.6a2 2 0 1 1 2 2h-2v-2zm-1 0a2 2 0 0 1-4 0V3.6a2 2 0 0 1 4 0v5z" fill="#2EB67D"/><path d="M15.4 18.2a2 2 0 1 1-2 2v-2h2zm0-1a2 2 0 0 1 0-4h5a2 2 0 0 1 0 4h-5z" fill="#ECB22E"/></svg>`,
  spotify: `<svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="11" fill="#1DB954"/><path d="M7 9.5c3.5-1 8-.5 11 1.5" stroke="white" stroke-width="1.8" stroke-linecap="round" fill="none"/><path d="M7.5 12.5c3-1 6.5-.5 9 1.2" stroke="white" stroke-width="1.6" stroke-linecap="round" fill="none"/><path d="M8 15.5c2.3-.7 5-.3 7 1" stroke="white" stroke-width="1.4" stroke-linecap="round" fill="none"/></svg>`,
  discord: `<svg viewBox="0 0 24 24"><path d="M20.3 4A17.5 17.5 0 0 0 15.6 2.5c-.3.4-.5.9-.7 1.3a16.3 16.3 0 0 0-5.7 0 12 12 0 0 0-.7-1.3A17.5 17.5 0 0 0 3.8 4 18.7 18.7 0 0 0 .7 18.5a17.6 17.6 0 0 0 5.4 2.7 13.1 13.1 0 0 0 1.1-1.8 11.4 11.4 0 0 1-1.8-.9l.4-.3a12.6 12.6 0 0 0 10.7 0l.4.3c-.6.3-1.2.7-1.8.9.3.6.7 1.2 1.1 1.8a17.6 17.6 0 0 0 5.4-2.7A18.7 18.7 0 0 0 20.3 4zM8.1 15.6c-1.2 0-2.2-1.1-2.2-2.5s1-2.5 2.2-2.5 2.2 1.1 2.2 2.5-1 2.5-2.2 2.5zm7.8 0c-1.2 0-2.2-1.1-2.2-2.5s1-2.5 2.2-2.5 2.2 1.1 2.2 2.5-1 2.5-2.2 2.5z" fill="#5865F2"/></svg>`,
  whatsapp: `<svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="11" fill="#25D366"/><path d="M7.5 7.5a6.5 6.5 0 0 1 9.2 9.2l.8 2.8-2.8-.8A6.5 6.5 0 0 1 7.5 7.5z" fill="white"/><path d="M9.5 10.5c.3.7.8 1.5 1.5 2.3.7.7 1.6 1.2 2.3 1.5l1-1c.2-.2.5-.2.7 0l1.3 1.3a.5.5 0 0 1 0 .7l-1 1c-.5.3-1 .3-1.5 0-2-1-4-3-4.5-5-.3-.5-.3-1 0-1.5l1-1a.5.5 0 0 1 .7 0l1.3 1.3c.2.2.2.5 0 .7l-1 .7z" fill="#25D366"/></svg>`,
  figma: `<svg viewBox="0 0 24 24"><rect x="5" y="2" width="7" height="7" rx="3.5" fill="#F24E1E"/><rect x="12" y="2" width="7" height="7" rx="3.5" fill="#FF7262"/><rect x="5" y="9" width="7" height="6" fill="#A259FF"/><circle cx="15.5" cy="12" r="3.5" fill="#1ABCFE"/><rect x="5" y="15" width="7" height="7" rx="3.5" fill="#0ACF83"/></svg>`,
  vlc: `<svg viewBox="0 0 24 24"><path d="M12 2L4 20h16L12 2z" fill="#FF8800"/><path d="M9 16l-1.5 4h9L15 16H9z" fill="white" opacity="0.4"/><path d="M10 12l-1 4h6l-1-4h-4z" fill="white" opacity="0.4"/></svg>`,
  folder: `<svg viewBox="0 0 24 24"><path d="M2 6a2 2 0 0 1 2-2h5l2 2h9a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V6z" fill="#FFC107"/><path d="M2 10h20v8a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2v-8z" fill="#FFD54F"/></svg>`,
  finder: `<svg viewBox="0 0 24 24"><rect width="24" height="24" rx="5" fill="#3D8AF7"/><circle cx="9" cy="10" r="2.2" fill="white"/><circle cx="15" cy="10" r="2.2" fill="white"/><path d="M8 15.5q4 3.5 8 0" stroke="white" stroke-width="1.5" fill="none" stroke-linecap="round"/></svg>`,
  obs: `<svg viewBox="0 0 24 24"><rect width="24" height="24" rx="5" fill="#302E31"/><circle cx="12" cy="12" r="7" fill="none" stroke="#A78BFA" stroke-width="2"/><circle cx="12" cy="12" r="3.5" fill="#7C3AED"/><circle cx="12" cy="12" r="1.5" fill="white"/></svg>`,
  terminal: `<svg viewBox="0 0 24 24"><rect width="24" height="24" rx="4" fill="#1E1E2E"/><path d="M5 8.5l5 3.5-5 3.5" stroke="#6EE7B7" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" fill="none"/><line x1="13" y1="16" x2="19" y2="16" stroke="#6EE7B7" stroke-width="1.8" stroke-linecap="round"/></svg>`,
  powershell: `<svg viewBox="0 0 24 24"><rect width="24" height="24" rx="4" fill="#012456"/><path d="M4 8l5 4-5 4" stroke="#00BFFF" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" fill="none"/><line x1="12" y1="16" x2="20" y2="16" stroke="#00BFFF" stroke-width="1.8" stroke-linecap="round"/></svg>`,

  // Letter-based icons — brand color + initial(s)
  "visual-studio": letter("VS", "#7B2FBE"),
  gmail:           letter("M",  "#EA4335"),
  outlook:         letter("O",  "#0078D4"),
  chatgpt:         letter("AI", "#10A37F"),
  claude:          letter("C",  "#CC5500"),
  matlab:          letter("M",  "#0076A8"),
  "system-settings": letter("S", "#888"),
  "activity-monitor": letter("A", "#F44336"),
  "task-manager":  letter("T",  "#0078D4"),
  garageband:      letter("G",  "#E07020"),
  "fl-studio":     letter("FL", "#E85E00"),
  autocad:         letter("A",  "#CC0000"),
  solidworks:      letter("S",  "#CC0000"),
  excel:           letter("X",  "#217346"),
  word:            letter("W",  "#2B579A"),
  powerpoint:      letter("P",  "#D24726"),
  access:          letter("A",  "#A4373A"),
  pages:           letter("P",  "#F7A133"),
  notion:          letter("N",  "#191919"),
  obsidian:        letter("O",  "#7C3AED"),
  onenote:         letter("N",  "#7719AA"),
  "apple-notes":   letter("N",  "#FFDB4D", "#1a1a1a"),
  teams:           letter("T",  "#5059C9"),
  zoom:            letter("Z",  "#2D8CFF"),
  skype:           letter("S",  "#00AFF0"),
  telegram:        letter("T",  "#2CA5E0"),
  photoshop:       letter("Ps", "#001E36"),
  illustrator:     letter("Ai", "#FF7C00"),
  premiere:        letter("Pr", "#2B0040"),
  "github-desktop": letter("G", "#24292E"),
  intellij:        letter("IJ", "#FE315D"),
  webstorm:        letter("WS", "#00C4B3"),
  pycharm:         letter("PC", "#21D789", "#1a1a1a"),
  cursor:          letter("C",  "#111"),
  neovim:          letter("NV", "#57A143"),
  vim:             letter("V",  "#019733"),
  xcode:           letter("X",  "#147EFB"),
  iterm2:          letter(">_", "#191919"),
};

const FALLBACK_COLORS = [
  "#6366F1","#EC4899","#F59E0B","#10B981","#3B82F6","#EF4444","#8B5CF6","#06B6D4"
];

export function getAppIcon(
  name: string,
  path: string,
  isVscode: boolean,
  isBrowser: boolean
): string {
  if (isVscode) return ICONS.vscode;

  const p = path.toLowerCase().replace(/\\/g, "/");
  const n = name.toLowerCase();

  if (isBrowser) {
    if (p.includes("chrome") && !p.includes("chromium")) return ICONS.chrome;
    if (p.includes("msedge") || p.includes("/edge/") || p.includes("microsoftedge")) return ICONS.edge;
    if (p.includes("firefox")) return ICONS.firefox;
    if (p.includes("safari")) return ICONS.safari;
    if (p.includes("brave")) return ICONS.brave;
    if (p.includes("opera")) return ICONS.opera;
    return ICONS.chrome;
  }

  // Dev editors / IDEs
  if (n === "cursor" || p.includes("/cursor/") || p.includes("\\cursor\\")) return ICONS.cursor;
  if (n.includes("intellij") || p.includes("intellij")) return ICONS.intellij;
  if (n.includes("webstorm") || p.includes("webstorm")) return ICONS.webstorm;
  if (n.includes("pycharm") || p.includes("pycharm")) return ICONS.pycharm;
  if (n.includes("xcode") || p.includes("xcode")) return ICONS.xcode;
  if (n.includes("github desktop") || p.includes("githubdesktop") || p.includes("github desktop")) return ICONS["github-desktop"];
  if (n === "nvim" || (n.includes("neovim") && !n.includes("vim"))) return ICONS.neovim;
  if (n === "vim" || p.endsWith("/vim") || p.endsWith("\\vim.exe")) return ICONS.vim;
  if (n.includes("visual studio") || p.includes("/devenv") || p.includes("\\devenv")) return ICONS["visual-studio"];

  // Communication
  if (n.includes("slack") || p.includes("slack")) return ICONS.slack;
  if (n.includes("discord") || p.includes("discord")) return ICONS.discord;
  if (n.includes("teams") || p.includes("/teams/") || p.includes("\\teams\\")) return ICONS.teams;
  if (n.includes("zoom") || p.includes("/zoom/") || p.includes("\\zoom\\")) return ICONS.zoom;
  if (n.includes("skype") || p.includes("skype")) return ICONS.skype;
  if (n.includes("telegram") || p.includes("telegram")) return ICONS.telegram;
  if (n.includes("whatsapp") || p.includes("whatsapp")) return ICONS.whatsapp;

  // Email
  if (n.includes("outlook") || p.includes("outlook")) return ICONS.outlook;
  if (n.includes("gmail")) return ICONS.gmail;

  // Media / creative
  if (n.includes("spotify") || p.includes("spotify")) return ICONS.spotify;
  if (n.includes("vlc") || p.includes("vlc")) return ICONS.vlc;
  if (n.includes("garageband") || p.includes("garageband")) return ICONS.garageband;
  if (n.includes("fl studio") || n.includes("fruityloops") || p.includes("fl studio")) return ICONS["fl-studio"];
  if (n.includes("obs") || p.includes("/obs/") || p.includes("obs64") || p.includes("obs32")) return ICONS.obs;
  if (n.includes("photoshop") || p.includes("photoshop")) return ICONS.photoshop;
  if (n.includes("illustrator") || p.includes("illustrator")) return ICONS.illustrator;
  if (n.includes("premiere") || p.includes("premiere")) return ICONS.premiere;
  if (n.includes("figma") || p.includes("figma")) return ICONS.figma;

  // Productivity / notes
  if (n.includes("notion") || p.includes("notion")) return ICONS.notion;
  if (n.includes("obsidian") || p.includes("obsidian")) return ICONS.obsidian;
  if (n.includes("onenote") || p.includes("onenote")) return ICONS.onenote;
  if ((n.includes("notes") && (n.includes("apple") || p.includes("/notes.app"))) || p.endsWith("/notes")) return ICONS["apple-notes"];

  // Microsoft Office
  if (n.includes("excel") || p.includes("excel")) return ICONS.excel;
  if (n.includes("word") || p.includes("winword")) return ICONS.word;
  if (n.includes("powerpoint") || p.includes("powerpnt")) return ICONS.powerpoint;
  if ((n.includes("access") && n.includes("microsoft")) || p.includes("msaccess")) return ICONS.access;
  if (n.includes("pages") || p.includes("/pages.app")) return ICONS.pages;

  // AI tools
  if (n.includes("chatgpt") || p.includes("chatgpt")) return ICONS.chatgpt;
  if (n.includes("claude") || p.includes("claude")) return ICONS.claude;

  // Engineering
  if (n.includes("matlab") || p.includes("matlab")) return ICONS.matlab;
  if (n.includes("autocad") || p.includes("autocad")) return ICONS.autocad;
  if (n.includes("solidworks") || p.includes("solidworks")) return ICONS.solidworks;

  // System / terminal
  if (n.includes("finder") || (p.includes("finder") && !p.includes("pathfinder"))) return ICONS.finder;
  if (p.includes("explorer.exe") || n === "file explorer" || n === "windows explorer") return ICONS.folder;
  if (n.includes("system settings") || n.includes("system preferences") || n.includes("control panel")) return ICONS["system-settings"];
  if (n.includes("activity monitor") || p.includes("activitymonitor")) return ICONS["activity-monitor"];
  if (n.includes("task manager") || p.includes("taskmgr")) return ICONS["task-manager"];
  if (n.includes("powershell") || p.includes("powershell")) return ICONS.powershell;
  if (n.includes("cmd") && (p.endsWith("/cmd.exe") || p.endsWith("\\cmd.exe"))) return ICONS.terminal;
  if (n.includes("windows terminal") || p.includes("windowsterminal")) return ICONS.terminal;
  if (n.includes("iterm") || p.includes("iterm")) return ICONS.iterm2;
  if (n.includes("terminal") || p.includes("terminal")) return ICONS.terminal;

  // Fallback: colored initial from name
  const idx = (n.charCodeAt(0) || 0) % FALLBACK_COLORS.length;
  return letter(name.charAt(0).toUpperCase() || "?", FALLBACK_COLORS[idx]);
}

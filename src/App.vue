<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open, save } from "@tauri-apps/plugin-dialog";
import GroupCard from "./components/GroupCard.vue";
import TemplateCard from "./components/TemplateCard.vue";
import SnapshotModal from "./components/SnapshotModal.vue";
import TemplateModal from "./components/TemplateModal.vue";
import RestoreModal from "./components/RestoreModal.vue";
import EditGroupModal from "./components/EditGroupModal.vue";
import CommandPalette from "./components/CommandPalette.vue";
import BrowserPanel from "./components/BrowserPanel.vue";
import AboutPage from "./pages/AboutPage.vue";
import FaqPage from "./pages/FaqPage.vue";
import ContactPage from "./pages/ContactPage.vue";
import {
  loadGroups, saveGroup, deleteGroup,
  loadTemplates, saveTemplate, deleteTemplate,
  updateGroup, updateTemplate,
  loadGroupOrder, saveGroupOrder,
  loadTemplateOrder, saveTemplateOrder,
} from "./store";
import { autoLayout } from "./layout";
import { exeBase } from "./paths";
import HardFreezeModal from "./components/HardFreezeModal.vue";
import type {
  CapturedApp, SavedGroup, ProjectTemplate,
  RestoreSummary, HardFreeze, StashExport,
  BridgeStatus, BrowserSnapshot, CloseSummary,
} from "./types";

// ── State ─────────────────────────────────────────────────────────────────────
const groups = ref<Record<string, SavedGroup>>({});
const templates = ref<Record<string, ProjectTemplate>>({});
const capturedApps = ref<CapturedApp[]>([]);
const restoreSummary = ref<RestoreSummary | null>(null);
const openApps = ref<CapturedApp[]>([]);

// Browser extension bridge
const bridgeStatus = ref<BridgeStatus | null>(null);
const browserSnapshot = ref<BrowserSnapshot | null>(null);

// Ordering
const groupOrder = ref<string[]>([]);
const templateOrder = ref<string[]>([]);

// Drag
const dragSrcGroup = ref<string | null>(null);
const dragOverGroup = ref<string | null>(null);
const dragSrcTemplate = ref<string | null>(null);
const dragOverTemplate = ref<string | null>(null);

// Search & filter
const searchQuery = ref("");
const sortKey = ref<"created_at" | "last_used" | "launch_count" | "app_count" | "name">("created_at");
const sortDir = ref<"desc" | "asc">("desc");
const tagFilter = ref<string[]>([]);

// UI toggles
const showSnapshot = ref(false);
const showTemplate = ref(false);
const showRestore = ref(false);
const showPalette = ref(false);
const showBrowser = ref(false);
const capturingLoading = ref(false);
const savingTemplate = ref(false);
const launchingGroup = ref<string | null>(null);
const launchingTemplate = ref<string | null>(null);

// Edit state
const editingGroup = ref<SavedGroup | null>(null);
const editingTemplate = ref<ProjectTemplate | null>(null);

// Hamburger / page nav
type Page = "home" | "about" | "faq" | "contact";
const currentPage = ref<Page>("home");
const showHamburger = ref(false);
const hamburgerRef = ref<HTMLElement | null>(null);

// Toast with optional undo action
const toast = ref<{
  message: string;
  type: "success" | "error";
  action?: { label: string; fn: () => void };
} | null>(null);

// Undo delete
const undoDelete = ref<{
  type: "group" | "template";
  name: string;
  item: SavedGroup | ProjectTemplate;
  timer: ReturnType<typeof setTimeout>;
} | null>(null);

// Freeze
const freezeTarget = ref<{ type: "group" | "template"; name: string } | null>(null);
const freezeWarning = ref<{ name: string; secondsLeft: number } | null>(null);
const triggeredToday = new Set<string>();

let toastTimer: ReturnType<typeof setTimeout> | null = null;
let pollTimer: ReturnType<typeof setInterval> | null = null;
let freezeTimer: ReturnType<typeof setInterval> | null = null;
let unlistenPalette: (() => void) | null = null;

// ── Computed: all tags across groups+templates ─────────────────────────────
const allTags = computed(() => {
  const set = new Set<string>();
  for (const g of Object.values(groups.value)) g.tags?.forEach((t) => set.add(t));
  for (const t of Object.values(templates.value)) t.tags?.forEach((t2) => set.add(t2));
  return [...set].sort();
});

// ── Computed: filtered + sorted lists ─────────────────────────────────────
function matchesSearch<T extends SavedGroup | ProjectTemplate>(item: T): boolean {
  const q = searchQuery.value.trim().toLowerCase();
  if (!q) return true;
  if (item.name.toLowerCase().includes(q)) return true;
  if (item.tags?.some((t) => t.includes(q))) return true;
  if ("apps" in item) {
    return item.apps.some((a) => exeBase(a.exe_path).includes(q) || a.name.toLowerCase().includes(q));
  } else {
    return item.entries.some((e) => exeBase(e.exe_path).includes(q) || e.name.toLowerCase().includes(q));
  }
}

function matchesTags<T extends SavedGroup | ProjectTemplate>(item: T): boolean {
  if (tagFilter.value.length === 0) return true;
  return tagFilter.value.every((ft) => item.tags?.includes(ft));
}

function sortItems<T extends SavedGroup | ProjectTemplate>(items: T[]): T[] {
  return [...items].sort((a, b) => {
    let cmp: number;
    if (sortKey.value === "name") {
      cmp = a.name.localeCompare(b.name);
    } else if (sortKey.value === "last_used") {
      const la = a.last_used ?? "";
      const lb = b.last_used ?? "";
      cmp = la.localeCompare(lb);
    } else if (sortKey.value === "launch_count") {
      cmp = (a.launch_count ?? 0) - (b.launch_count ?? 0);
    } else if (sortKey.value === "app_count") {
      const ac = "apps" in a ? a.apps.length : a.entries.length;
      const bc = "apps" in b ? b.apps.length : b.entries.length;
      cmp = ac - bc;
    } else {
      cmp = a.created_at.localeCompare(b.created_at);
    }
    return sortDir.value === "desc" ? -cmp : cmp;
  });
}

function applyOrder<T extends SavedGroup | ProjectTemplate>(items: T[], order: string[]): T[] {
  if (order.length === 0 || searchQuery.value.trim() || tagFilter.value.length > 0) return items;
  const nameMap = new Map(items.map((i) => [i.name, i]));
  const ordered: T[] = [];
  for (const n of order) {
    const item = nameMap.get(n);
    if (item) { ordered.push(item); nameMap.delete(n); }
  }
  // Append any newly added items not yet in order
  for (const item of nameMap.values()) ordered.push(item);
  return ordered;
}

const filteredGroups = computed(() => {
  const all = Object.values(groups.value).filter((g) => matchesSearch(g) && matchesTags(g));
  const sorted = sortItems(all);
  return applyOrder(sorted, groupOrder.value);
});

const filteredTemplates = computed(() => {
  const all = Object.values(templates.value).filter((t) => matchesSearch(t) && matchesTags(t));
  const sorted = sortItems(all);
  return applyOrder(sorted, templateOrder.value);
});

const hasAnything = computed(
  () => Object.keys(groups.value).length > 0 || Object.keys(templates.value).length > 0
);

const hasResults = computed(
  () => filteredGroups.value.length > 0 || filteredTemplates.value.length > 0
);

// ── Active detection ──────────────────────────────────────────────────────────
function isAppOpen(exePath: string, isVscode: boolean, contextPath: string | null): boolean {
  const name = exeBase(exePath);
  if (isVscode && contextPath) {
    return openApps.value.some(
      (a) => exeBase(a.exe_path) === name && a.context_path === contextPath
    );
  }
  return openApps.value.some((a) => exeBase(a.exe_path) === name);
}

function isGroupActive(group: SavedGroup): boolean {
  return group.apps.length > 0 && group.apps.every((a) =>
    isAppOpen(a.exe_path, a.is_vscode, a.context_path)
  );
}

function isTemplateActive(tmpl: ProjectTemplate): boolean {
  return tmpl.entries.length > 0 && tmpl.entries.every((e) =>
    isAppOpen(e.exe_path, e.is_vscode, e.context_path)
  );
}

async function refreshOpenApps() {
  try {
    const apps = await invoke<CapturedApp[]>("capture_snapshot");
    openApps.value = apps;
  } catch {
    // silently ignore
  }
}

// ── Browser bridge ────────────────────────────────────────────────────────────
async function refreshBridge() {
  try {
    bridgeStatus.value = await invoke<BridgeStatus>("bridge_status");
    browserSnapshot.value = await invoke<BrowserSnapshot | null>("browser_state");
  } catch {
    bridgeStatus.value = null;
  }
}

// ── Hard Freeze ───────────────────────────────────────────────────────────────
function openFreezeModal(type: "group" | "template", name: string) {
  freezeTarget.value = { type, name };
}

function currentFreezeConfig(): HardFreeze | undefined {
  if (!freezeTarget.value) return undefined;
  if (freezeTarget.value.type === "group") return groups.value[freezeTarget.value.name]?.hard_freeze;
  return templates.value[freezeTarget.value.name]?.hard_freeze;
}

function currentFreezeAppCount(): number {
  if (!freezeTarget.value) return 0;
  if (freezeTarget.value.type === "group") return groups.value[freezeTarget.value.name]?.apps.length ?? 0;
  return templates.value[freezeTarget.value.name]?.entries.length ?? 0;
}

async function handleSaveFreeze(freeze: HardFreeze) {
  if (!freezeTarget.value) return;
  try {
    if (freezeTarget.value.type === "group") {
      const g = groups.value[freezeTarget.value.name];
      if (g) await updateGroup(g.name, { ...g, hard_freeze: freeze });
      groups.value = await loadGroups();
    } else {
      const t = templates.value[freezeTarget.value.name];
      if (t) await updateTemplate(t.name, { ...t, hard_freeze: freeze });
      templates.value = await loadTemplates();
    }
    showToast("Hard Freeze saved", "success");
  } catch (err) {
    showToast(`Save failed: ${String(err)}`, "error");
  } finally {
    freezeTarget.value = null;
  }
}

async function handleRemoveFreeze() {
  if (!freezeTarget.value) return;
  try {
    if (freezeTarget.value.type === "group") {
      const g = groups.value[freezeTarget.value.name];
      if (g) {
        const { hard_freeze: _, ...rest } = g;
        await updateGroup(g.name, rest as SavedGroup);
      }
      groups.value = await loadGroups();
    } else {
      const t = templates.value[freezeTarget.value.name];
      if (t) {
        const { hard_freeze: _, ...rest } = t;
        await updateTemplate(t.name, rest as ProjectTemplate);
      }
      templates.value = await loadTemplates();
    }
    showToast("Hard Freeze removed");
  } catch (err) {
    showToast(`Remove failed: ${String(err)}`, "error");
  } finally {
    freezeTarget.value = null;
  }
}

function checkFreezes() {
  const now = new Date();
  const today = now.toISOString().slice(0, 10);
  const currentDay = now.getDay();
  const currentSecs = now.getHours() * 3600 + now.getMinutes() * 60 + now.getSeconds();

  type FreezeItem = { key: string; name: string; freeze: HardFreeze; apps: CapturedApp[] };
  const all: FreezeItem[] = [];

  for (const g of Object.values(groups.value)) {
    if (g.hard_freeze?.enabled) {
      all.push({ key: `g-${g.name}-${today}`, name: g.name, freeze: g.hard_freeze, apps: g.apps });
    }
  }
  for (const t of Object.values(templates.value)) {
    if (t.hard_freeze?.enabled) {
      all.push({ key: `t-${t.name}-${today}`, name: t.name, freeze: t.hard_freeze, apps: entriesToApps(t) });
    }
  }

  let nextWarning: typeof freezeWarning.value = null;

  for (const item of all) {
    if (triggeredToday.has(item.key)) continue;
    const { freeze } = item;
    if (freeze.days.length > 0 && !freeze.days.includes(currentDay)) continue;

    const [fh, fm] = freeze.time.split(":").map(Number);
    const freezeSecs = fh * 3600 + fm * 60;
    const until = freezeSecs - currentSecs;

    if (until <= 0 && until > -30) {
      triggeredToday.add(item.key);
      freezeWarning.value = null;
      // A scheduled freeze escalates to a forced close: the user asked for the
      // workspace to be gone at this time, and nobody is present to answer a
      // "save your work?" prompt.
      invoke("close_workspace", { apps: item.apps, force: true }).catch(() => {});
      showToast(`❄ Hard Freeze: "${item.name}" closed`, "success");
      refreshOpenApps();
      return;
    } else if (until > 0 && until <= freeze.warning_seconds) {
      nextWarning = { name: item.name, secondsLeft: until };
    }
  }

  freezeWarning.value = nextWarning;
}

// ── Init ──────────────────────────────────────────────────────────────────────
onMounted(async () => {
  [groups.value, templates.value, groupOrder.value, templateOrder.value] = await Promise.all([
    loadGroups(), loadTemplates(), loadGroupOrder(), loadTemplateOrder(),
  ]);
  await Promise.all([refreshOpenApps(), refreshBridge()]);
  pollTimer = setInterval(() => {
    refreshOpenApps();
    refreshBridge();
  }, 8000);
  freezeTimer = setInterval(checkFreezes, 5000);

  // Listen for global shortcut palette-open event from Rust
  unlistenPalette = await listen("palette-open", () => {
    showPalette.value = true;
  });
});

onUnmounted(() => {
  if (pollTimer) clearInterval(pollTimer);
  if (freezeTimer) clearInterval(freezeTimer);
  if (unlistenPalette) unlistenPalette();
  document.removeEventListener("click", handleHamburgerOutside);
});

// ── Hamburger outside-click ───────────────────────────────────────────────────
function handleHamburgerOutside(e: MouseEvent) {
  if (hamburgerRef.value && !hamburgerRef.value.contains(e.target as Node)) {
    showHamburger.value = false;
  }
}

watch(showHamburger, (val) => {
  if (val) document.addEventListener("click", handleHamburgerOutside);
  else document.removeEventListener("click", handleHamburgerOutside);
});

function toggleHamburger(e: MouseEvent) {
  e.stopPropagation();
  showHamburger.value = !showHamburger.value;
}

function navigate(page: Page) {
  currentPage.value = page;
  showHamburger.value = false;
}

// ── Toast ─────────────────────────────────────────────────────────────────────
function showToast(
  message: string,
  type: "success" | "error" = "success",
  action?: { label: string; fn: () => void }
) {
  if (toastTimer) clearTimeout(toastTimer);
  toast.value = { message, type, action };
  if (!action) {
    toastTimer = setTimeout(() => { toast.value = null; }, 3500);
  } else {
    // Longer display when there's an action
    toastTimer = setTimeout(() => { toast.value = null; }, 5500);
  }
}

// ── Snapshot flow ─────────────────────────────────────────────────────────────
async function openSnapshot() {
  showSnapshot.value = true;
  capturingLoading.value = true;
  capturedApps.value = [];
  try {
    capturedApps.value = await invoke<CapturedApp[]>("capture_snapshot");
  } catch (err) {
    showToast(String(err), "error");
    showSnapshot.value = false;
  } finally {
    capturingLoading.value = false;
  }
}

async function handleSave(name: string, apps: CapturedApp[]) {
  const group: SavedGroup = {
    name,
    created_at: new Date().toISOString(),
    apps,
  };
  try {
    await saveGroup(group);
    showSnapshot.value = false;
    capturedApps.value = [];
    showToast(`"${name}" saved`, "success");
    groups.value = await loadGroups();
    // Append to order if not already present
    if (!groupOrder.value.includes(name)) {
      groupOrder.value = [...groupOrder.value, name];
      await saveGroupOrder(groupOrder.value);
    }
  } catch (err) {
    showToast(`Save failed: ${String(err)}`, "error");
  }
}

/**
 * Save a fresh capture as a reusable template rather than a workspace.
 *
 * Templates carry no geometry — `autoLayout` assigns each entry a grid slot at
 * launch time — so the captured rectangles are deliberately dropped here.
 */
async function handleSaveAsTemplate(name: string, apps: CapturedApp[]) {
  const template: ProjectTemplate = {
    name,
    created_at: new Date().toISOString(),
    entries: apps.map((a) => ({
      name: a.name,
      exe_path: a.exe_path,
      is_vscode: a.is_vscode,
      is_browser: a.is_browser,
      is_terminal: a.is_terminal ?? false,
      context_path: a.context_path,
      target_url: a.target_url,
      working_dir: a.working_dir ?? null,
    })),
  };
  try {
    await saveTemplate(template);
    showSnapshot.value = false;
    capturedApps.value = [];
    showToast(`Template "${name}" saved`, "success");
    templates.value = await loadTemplates();
    if (!templateOrder.value.includes(name)) {
      templateOrder.value = [...templateOrder.value, name];
      await saveTemplateOrder(templateOrder.value);
    }
  } catch (err) {
    showToast(`Save failed: ${String(err)}`, "error");
  }
}

function cancelSnapshot() {
  showSnapshot.value = false;
  capturedApps.value = [];
}

// ── Workspace restore ─────────────────────────────────────────────────────────
async function handleLaunch(group: SavedGroup) {
  launchingGroup.value = group.name;
  try {
    const summary = await invoke<RestoreSummary>("restore_workspace", { apps: group.apps });
    restoreSummary.value = summary;
    showRestore.value = true;
    // Update launch stats
    const updated: SavedGroup = {
      ...group,
      launch_count: (group.launch_count ?? 0) + 1,
      last_used: new Date().toISOString(),
    };
    await updateGroup(group.name, updated);
    groups.value = await loadGroups();
  } catch (err) {
    showToast(String(err), "error");
  } finally {
    launchingGroup.value = null;
  }
}

// ── Undo delete helpers ───────────────────────────────────────────────────────
async function finalizePendingDelete() {
  if (!undoDelete.value) return;
  clearTimeout(undoDelete.value.timer);
  const { type, name } = undoDelete.value;
  undoDelete.value = null;
  if (type === "group") await deleteGroup(name).catch(() => {});
  else await deleteTemplate(name).catch(() => {});
}

function handleUndoDelete() {
  if (!undoDelete.value) return;
  clearTimeout(undoDelete.value.timer);
  const { type, name, item } = undoDelete.value;
  undoDelete.value = null;
  if (type === "group") {
    groups.value = { ...groups.value, [name]: item as SavedGroup };
  } else {
    templates.value = { ...templates.value, [name]: item as ProjectTemplate };
  }
  toast.value = null;
}

async function handleDelete(name: string) {
  const item = groups.value[name];
  if (!item) return;

  await finalizePendingDelete();

  // Soft-delete from local state
  const newGroups = { ...groups.value };
  delete newGroups[name];
  groups.value = newGroups;

  const timer = setTimeout(async () => {
    undoDelete.value = null;
    await deleteGroup(name);
    groupOrder.value = groupOrder.value.filter((n) => n !== name);
    await saveGroupOrder(groupOrder.value);
  }, 5000);

  undoDelete.value = { type: "group", name, item, timer };
  showToast(`"${name}" deleted`, "success", { label: "Undo", fn: handleUndoDelete });
}

// ── Edit group ────────────────────────────────────────────────────────────────
function openEditGroup(group: SavedGroup) {
  editingGroup.value = group;
}

async function handleSaveEditGroup(updated: SavedGroup) {
  if (!editingGroup.value) return;
  try {
    const oldName = editingGroup.value.name;
    await updateGroup(oldName, updated);
    groups.value = await loadGroups();
    // Update order if renamed
    if (oldName !== updated.name) {
      groupOrder.value = groupOrder.value.map((n) => (n === oldName ? updated.name : n));
      await saveGroupOrder(groupOrder.value);
    }
    showToast(`"${updated.name}" updated`, "success");
  } catch (err) {
    showToast(`Update failed: ${String(err)}`, "error");
  } finally {
    editingGroup.value = null;
  }
}

// ── Close apps ────────────────────────────────────────────────────────────────
/**
 * Materialise a template's entries as CapturedApp records.
 *
 * Templates store no geometry, so `autoLayout` assigns a grid slot to each entry
 * at launch time. The same shape is what `close_workspace` expects.
 */
function entriesToApps(tmpl: ProjectTemplate): CapturedApp[] {
  const positions = autoLayout(tmpl.entries.length);
  return tmpl.entries.map((entry, i) => ({
    name: entry.name,
    exe_path: entry.exe_path,
    title: entry.name,
    is_vscode: entry.is_vscode,
    is_browser: entry.is_browser,
    is_terminal: entry.is_terminal ?? false,
    context_path: entry.context_path,
    target_url: entry.target_url,
    working_dir: entry.working_dir ?? null,
    relative_position: positions[i],
    browser_window: null,
  }));
}

/** Summarise a close so the user knows what actually happened. */
function describeClose(summary: CloseSummary, name: string): string {
  const parts: string[] = [];
  if (summary.closed > 0) parts.push(`${summary.closed} window(s) closed`);
  if (summary.browser_windows_closed > 0) {
    parts.push(`${summary.browser_windows_closed} browser window(s)`);
  }
  if (summary.force_killed > 0) parts.push(`${summary.force_killed} force-closed`);
  if (parts.length === 0) return `Nothing was running in "${name}"`;
  return `"${name}": ${parts.join(", ")}`;
}

async function handleCloseGroupApps(group: SavedGroup) {
  try {
    const summary = await invoke<CloseSummary>("close_workspace", {
      apps: group.apps,
      force: false,
    });
    showToast(describeClose(summary, group.name), "success");
    // Applications may show a save prompt, so the window list settles a moment later.
    setTimeout(refreshOpenApps, 1500);
  } catch (err) {
    showToast(String(err), "error");
  }
}

async function handleCloseTemplateApps(tmpl: ProjectTemplate) {
  try {
    const summary = await invoke<CloseSummary>("close_workspace", {
      apps: entriesToApps(tmpl),
      force: false,
    });
    showToast(describeClose(summary, tmpl.name), "success");
    setTimeout(refreshOpenApps, 1500);
  } catch (err) {
    showToast(String(err), "error");
  }
}

// ── Template create / edit ────────────────────────────────────────────────────
function openNewTemplate() {
  editingTemplate.value = null;
  showTemplate.value = true;
}

function openEditTemplate(tmpl: ProjectTemplate) {
  editingTemplate.value = tmpl;
  showTemplate.value = true;
}

async function handleSaveTemplate(template: ProjectTemplate) {
  savingTemplate.value = true;
  try {
    if (editingTemplate.value) {
      const oldName = editingTemplate.value.name;
      await updateTemplate(oldName, template);
      templates.value = await loadTemplates();
      if (oldName !== template.name) {
        templateOrder.value = templateOrder.value.map((n) => (n === oldName ? template.name : n));
        await saveTemplateOrder(templateOrder.value);
      }
      showToast(`Template "${template.name}" updated`, "success");
    } else {
      await saveTemplate(template);
      templates.value = await loadTemplates();
      if (!templateOrder.value.includes(template.name)) {
        templateOrder.value = [...templateOrder.value, template.name];
        await saveTemplateOrder(templateOrder.value);
      }
      showToast(`Template "${template.name}" saved`, "success");
    }
    showTemplate.value = false;
    editingTemplate.value = null;
  } catch (err) {
    showToast(`Save failed: ${String(err)}`, "error");
  } finally {
    savingTemplate.value = false;
  }
}

// ── Template launch ───────────────────────────────────────────────────────────
async function handleLaunchTemplate(template: ProjectTemplate) {
  launchingTemplate.value = template.name;
  try {
    const apps = entriesToApps(template);
    const summary = await invoke<RestoreSummary>("restore_workspace", { apps });
    restoreSummary.value = summary;
    showRestore.value = true;
    // Update launch stats
    const updated: ProjectTemplate = {
      ...template,
      launch_count: (template.launch_count ?? 0) + 1,
      last_used: new Date().toISOString(),
    };
    await updateTemplate(template.name, updated);
    templates.value = await loadTemplates();
  } catch (err) {
    showToast(String(err), "error");
  } finally {
    launchingTemplate.value = null;
  }
}

async function handleDeleteTemplate(name: string) {
  const item = templates.value[name];
  if (!item) return;

  await finalizePendingDelete();

  const newTemplates = { ...templates.value };
  delete newTemplates[name];
  templates.value = newTemplates;

  const timer = setTimeout(async () => {
    undoDelete.value = null;
    await deleteTemplate(name);
    templateOrder.value = templateOrder.value.filter((n) => n !== name);
    await saveTemplateOrder(templateOrder.value);
  }, 5000);

  undoDelete.value = { type: "template", name, item, timer };
  showToast(`Template "${name}" deleted`, "success", { label: "Undo", fn: handleUndoDelete });
}

// ── Drag to reorder ───────────────────────────────────────────────────────────
function handleGroupDragStart(name: string, e: DragEvent) {
  dragSrcGroup.value = name;
  if (e.dataTransfer) e.dataTransfer.effectAllowed = "move";
}

function handleGroupDragOver(name: string) {
  dragOverGroup.value = name;
}

async function handleGroupDrop(targetName: string) {
  const src = dragSrcGroup.value;
  if (!src || src === targetName) {
    dragSrcGroup.value = null;
    dragOverGroup.value = null;
    return;
  }
  const names = filteredGroups.value.map((g) => g.name);
  const srcIdx = names.indexOf(src);
  const tgtIdx = names.indexOf(targetName);
  if (srcIdx < 0 || tgtIdx < 0) return;
  const newOrder = [...names];
  newOrder.splice(srcIdx, 1);
  newOrder.splice(tgtIdx, 0, src);
  groupOrder.value = newOrder;
  await saveGroupOrder(newOrder);
  dragSrcGroup.value = null;
  dragOverGroup.value = null;
}

function handleGroupDragEnd() {
  dragSrcGroup.value = null;
  dragOverGroup.value = null;
}

function handleTemplateDragStart(name: string, e: DragEvent) {
  dragSrcTemplate.value = name;
  if (e.dataTransfer) e.dataTransfer.effectAllowed = "move";
}

function handleTemplateDragOver(name: string) {
  dragOverTemplate.value = name;
}

async function handleTemplateDrop(targetName: string) {
  const src = dragSrcTemplate.value;
  if (!src || src === targetName) {
    dragSrcTemplate.value = null;
    dragOverTemplate.value = null;
    return;
  }
  const names = filteredTemplates.value.map((t) => t.name);
  const srcIdx = names.indexOf(src);
  const tgtIdx = names.indexOf(targetName);
  if (srcIdx < 0 || tgtIdx < 0) return;
  const newOrder = [...names];
  newOrder.splice(srcIdx, 1);
  newOrder.splice(tgtIdx, 0, src);
  templateOrder.value = newOrder;
  await saveTemplateOrder(newOrder);
  dragSrcTemplate.value = null;
  dragOverTemplate.value = null;
}

function handleTemplateDragEnd() {
  dragSrcTemplate.value = null;
  dragOverTemplate.value = null;
}

// ── Import / Export ───────────────────────────────────────────────────────────
async function handleExport() {
  try {
    const filePath = await save({
      title: "Export Stash Workspaces",
      defaultPath: "stash-export.stash",
      filters: [{ name: "Stash Export", extensions: ["stash", "json"] }],
    });
    if (!filePath) return;
    const data: StashExport = {
      version: 1,
      exported_at: new Date().toISOString(),
      groups: groups.value,
      templates: templates.value,
    };
    await invoke("write_file", { path: filePath, content: JSON.stringify(data, null, 2) });
    showToast("Workspaces exported", "success");
  } catch (err) {
    showToast(`Export failed: ${String(err)}`, "error");
  }
}

async function handleImport() {
  try {
    const filePath = await open({
      title: "Import Stash Workspaces",
      multiple: false,
      filters: [{ name: "Stash Export", extensions: ["stash", "json"] }],
    });
    if (typeof filePath !== "string" || !filePath) return;
    const raw = await invoke<string>("read_file", { path: filePath });
    const data = JSON.parse(raw) as StashExport;
    if (!data.groups || !data.templates) throw new Error("Invalid export file");
    let imported = 0;
    for (const g of Object.values(data.groups)) {
      await saveGroup(g);
      imported++;
    }
    for (const t of Object.values(data.templates)) {
      await saveTemplate(t);
      imported++;
    }
    groups.value = await loadGroups();
    templates.value = await loadTemplates();
    showToast(`Imported ${imported} item(s)`, "success");
  } catch (err) {
    showToast(`Import failed: ${String(err)}`, "error");
  }
}

// ── Tag filter toggle ─────────────────────────────────────────────────────────
function toggleTagFilter(tag: string) {
  const idx = tagFilter.value.indexOf(tag);
  if (idx >= 0) tagFilter.value.splice(idx, 1);
  else tagFilter.value.push(tag);
}

// ── Command palette ───────────────────────────────────────────────────────────
function handlePaletteLaunchGroup(name: string) {
  showPalette.value = false;
  const g = groups.value[name];
  if (g) handleLaunch(g);
}

function handlePaletteLaunchTemplate(name: string) {
  showPalette.value = false;
  const t = templates.value[name];
  if (t) handleLaunchTemplate(t);
}
</script>

<template>
  <div id="app-root">
    <!-- Header -->
    <header class="header">
      <div class="header-brand">
        <button
          v-if="currentPage !== 'home'"
          class="btn btn-ghost"
          style="padding:5px 10px;margin-right:4px;"
          @click="navigate('home')"
          aria-label="Back to home"
        >Home</button>
        <template v-else>
          <div class="header-logo" aria-hidden="true">📦</div>
          <h1>Stash</h1>
        </template>
      </div>
      <div class="header-actions">
        <template v-if="currentPage === 'home'">
          <button class="btn btn-ghost btn-icon-label" title="Import workspaces from file" @click="handleImport">
            <svg class="btn-svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
              <polyline points="7 10 12 15 17 10"/>
              <line x1="12" y1="15" x2="12" y2="3"/>
            </svg>
            <span>Import</span>
          </button>
          <button class="btn btn-ghost btn-icon-label" title="Export workspaces to file" @click="handleExport">
            <svg class="btn-svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
              <polyline points="17 8 12 3 7 8"/>
              <line x1="12" y1="3" x2="12" y2="15"/>
            </svg>
            <span>Export</span>
          </button>
          <button class="btn btn-ghost btn-icon-label" @click="openNewTemplate" title="Create a new project template">
            <svg class="btn-svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/>
              <polyline points="14 2 14 8 20 8"/>
              <line x1="12" y1="11" x2="12" y2="17"/>
              <line x1="9" y1="14" x2="15" y2="14"/>
            </svg>
            <span>Template</span>
          </button>
          <button class="btn btn-primary btn-icon-label" @click="openSnapshot" title="Capture current open windows as a workspace">
            <svg class="btn-svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M23 19a2 2 0 0 1-2 2H3a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h4l2-3h6l2 3h4a2 2 0 0 1 2 2z"/>
              <circle cx="12" cy="13" r="4"/>
            </svg>
            <span>Snapshot</span>
          </button>
          <button
            class="btn btn-ghost browser-trigger-btn"
            :class="{ 'browser-connected': bridgeStatus?.connected }"
            @click="showBrowser = true; refreshBridge()"
            :aria-label="bridgeStatus?.connected ? 'Browser connected' : 'Connect browser extension'"
            :title="bridgeStatus?.connected
              ? `Browser connected — ${bridgeStatus.tab_count} tabs across ${bridgeStatus.window_count} windows`
              : 'Connect the Stash Bridge extension to capture browser tabs'"
          >
            <svg class="btn-svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <circle cx="12" cy="12" r="10"/>
              <line x1="2" y1="12" x2="22" y2="12"/>
              <path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"/>
            </svg>
            <span v-if="bridgeStatus?.connected" class="browser-tab-badge">{{ bridgeStatus.tab_count }}</span>
          </button>
          <button class="btn btn-ghost palette-trigger-btn" @click="showPalette = true" title="Quick launch (Ctrl+Shift+S)">
            <svg class="btn-svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2"/>
            </svg>
          </button>
        </template>
        <div class="hamburger-wrap" ref="hamburgerRef">
          <button class="hamburger-btn" @click="toggleHamburger" aria-label="Menu" :aria-expanded="showHamburger">
            ☰
          </button>
          <div v-if="showHamburger" class="hamburger-menu" role="menu">
            <button class="hamburger-item" :class="{ 'active-nav': currentPage === 'about' }" @click="navigate('about')" role="menuitem">About</button>
            <button class="hamburger-item" :class="{ 'active-nav': currentPage === 'faq' }" @click="navigate('faq')" role="menuitem">FAQ</button>
            <button class="hamburger-item" :class="{ 'active-nav': currentPage === 'contact' }" @click="navigate('contact')" role="menuitem">Contact</button>
          </div>
        </div>
      </div>
    </header>

    <!-- Pages -->
    <main class="main">
      <!-- About / FAQ / Contact -->
      <template v-if="currentPage === 'about'"><AboutPage /></template>
      <template v-else-if="currentPage === 'faq'"><FaqPage /></template>
      <template v-else-if="currentPage === 'contact'"><ContactPage /></template>

      <!-- Home -->
      <template v-else>

        <!-- ── Search & Filter bar ──────────────────────────────────────── -->
        <div v-if="hasAnything" class="search-filter-bar">
          <div class="search-input-wrap">
            <svg class="search-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <circle cx="11" cy="11" r="8"/>
              <path d="m21 21-4.35-4.35"/>
            </svg>
            <input
              class="search-input"
              v-model="searchQuery"
              placeholder="Search by name, app, or tag…"
              aria-label="Search workspaces"
            />
            <button v-if="searchQuery" class="search-clear-btn" @click="searchQuery = ''" aria-label="Clear search">✕</button>
          </div>
          <div class="sort-controls">
            <select class="sort-select form-select" v-model="sortKey" aria-label="Sort by">
              <option value="created_at">Date Created</option>
              <option value="last_used">Last Used</option>
              <option value="launch_count">Most Used</option>
              <option value="app_count">Most Apps</option>
              <option value="name">Name</option>
            </select>
            <button
              class="sort-dir-btn"
              @click="sortDir = sortDir === 'desc' ? 'asc' : 'desc'"
              :title="sortDir === 'desc' ? 'Descending' : 'Ascending'"
              aria-label="Toggle sort direction"
            >{{ sortDir === "desc" ? "↓" : "↑" }}</button>
          </div>
        </div>

        <!-- ── Tag filter chips ──────────────────────────────────────────── -->
        <div v-if="allTags.length > 0" class="tag-filter-row">
          <button
            v-for="tag in allTags"
            :key="tag"
            class="tag-filter-chip"
            :class="{ 'tag-filter-chip-active': tagFilter.includes(tag) }"
            @click="toggleTagFilter(tag)"
          >{{ tag }}</button>
          <button v-if="tagFilter.length > 0" class="tag-filter-clear" @click="tagFilter = []">Clear filters</button>
        </div>

        <!-- ── Templates section ──────────────────────────────────────────── -->
        <template v-if="filteredTemplates.length > 0">
          <div class="section-header">
            <h2>Project Templates</h2>
            <span class="section-count">{{ filteredTemplates.length }}</span>
          </div>
          <div class="workspace-grid section-grid">
            <TemplateCard
              v-for="tmpl in filteredTemplates"
              :key="tmpl.name"
              :template="tmpl"
              :launching="launchingTemplate === tmpl.name"
              :active="isTemplateActive(tmpl)"
              :dragging="dragSrcTemplate === tmpl.name"
              :drag-over="dragOverTemplate === tmpl.name"
              @launch="handleLaunchTemplate(tmpl)"
              @delete="handleDeleteTemplate(tmpl.name)"
              @edit="openEditTemplate(tmpl)"
              @close-apps="handleCloseTemplateApps(tmpl)"
              @freeze-click="openFreezeModal('template', tmpl.name)"
              @drag-start="(e) => handleTemplateDragStart(tmpl.name, e)"
              @drag-over="handleTemplateDragOver(tmpl.name)"
              @drop="handleTemplateDrop(tmpl.name)"
              @drag-end="handleTemplateDragEnd"
            />
          </div>
        </template>

        <!-- ── Workspaces section ─────────────────────────────────────────── -->
        <div
          class="section-header"
          :style="filteredTemplates.length > 0 ? 'margin-top:22px' : ''"
        >
          <h2>Saved Workspaces</h2>
          <span class="section-count">{{ filteredGroups.length }}</span>
        </div>

        <div v-if="filteredGroups.length > 0" class="workspace-grid">
          <GroupCard
            v-for="group in filteredGroups"
            :key="group.name"
            :group="group"
            :launching="launchingGroup === group.name"
            :active="isGroupActive(group)"
            :dragging="dragSrcGroup === group.name"
            :drag-over="dragOverGroup === group.name"
            @launch="handleLaunch(group)"
            @delete="handleDelete(group.name)"
            @edit="openEditGroup(group)"
            @close-apps="handleCloseGroupApps(group)"
            @freeze-click="openFreezeModal('group', group.name)"
            @drag-start="(e) => handleGroupDragStart(group.name, e)"
            @drag-over="handleGroupDragOver(group.name)"
            @drop="handleGroupDrop(group.name)"
            @drag-end="handleGroupDragEnd"
          />
        </div>

        <!-- ── No search results (but data exists) ──────────────────────── -->
        <div
          v-if="hasAnything && !hasResults && (searchQuery || tagFilter.length > 0)"
          class="search-no-results"
        >
          <div class="empty-icon" style="font-size:2rem;">🔍</div>
          <p>No workspaces match your search.</p>
          <button class="btn btn-ghost" style="margin-top:8px;" @click="searchQuery = ''; tagFilter = []">Clear filters</button>
        </div>

        <!-- ── Full empty state ────────────────────────────────────────── -->
        <div v-if="!hasAnything" class="empty-state">
          <div class="empty-icon">📦</div>
          <h2 style="text-transform:none;font-size:1.15rem;color:var(--text-primary);font-weight:700;">
            Welcome to Stash
          </h2>
          <p class="empty-desc">
            Stash saves your open apps and window positions as a <strong>workspace</strong>.
            Take a Snapshot to capture what's open right now, or build a
            <strong>Template</strong> to define a reusable launch plan.
          </p>
          <div class="empty-actions">
            <button class="btn btn-ghost" @click="openNewTemplate">+ Add Template</button>
            <button class="btn btn-primary empty-cta" @click="openSnapshot">+ Take Snapshot</button>
          </div>
        </div>

        <!-- Workspaces-only empty (templates exist but no snapshots) -->
        <div v-else-if="filteredGroups.length === 0 && !searchQuery && tagFilter.length === 0 && filteredTemplates.length > 0" class="workspaces-empty">
          No snapshots saved yet —
          <button class="inline-link" @click="openSnapshot">+ Take Snapshot</button>
          to capture your open windows.
        </div>

      </template>
    </main>

    <!-- Snapshot modal -->
    <transition name="fade">
      <SnapshotModal
        v-if="showSnapshot"
        :apps="capturedApps"
        :loading="capturingLoading"
        @save="handleSave"
        @save-template="handleSaveAsTemplate"
        @cancel="cancelSnapshot"
      />
    </transition>

    <!-- Template create / edit modal -->
    <transition name="fade">
      <TemplateModal
        v-if="showTemplate"
        :saving="savingTemplate"
        :initial="editingTemplate ?? undefined"
        @save="handleSaveTemplate"
        @cancel="showTemplate = false; editingTemplate = null"
      />
    </transition>

    <!-- Edit group modal -->
    <transition name="fade">
      <EditGroupModal
        v-if="editingGroup"
        :group="editingGroup"
        @save="handleSaveEditGroup"
        @cancel="editingGroup = null"
      />
    </transition>

    <!-- Restore result modal -->
    <transition name="fade">
      <RestoreModal
        v-if="showRestore && restoreSummary"
        :summary="restoreSummary"
        @close="showRestore = false"
      />
    </transition>

    <!-- Hard Freeze modal -->
    <transition name="fade">
      <HardFreezeModal
        v-if="freezeTarget"
        :name="freezeTarget.name"
        :app-count="currentFreezeAppCount()"
        :existing="currentFreezeConfig()"
        @save="handleSaveFreeze"
        @remove="handleRemoveFreeze"
        @cancel="freezeTarget = null"
      />
    </transition>

    <!-- Browser bridge panel -->
    <transition name="fade">
      <BrowserPanel
        v-if="showBrowser"
        :status="bridgeStatus"
        :snapshot="browserSnapshot"
        @refresh="refreshBridge"
        @close="showBrowser = false"
      />
    </transition>

    <!-- Command Palette -->
    <transition name="fade">
      <CommandPalette
        v-if="showPalette"
        :groups="groups"
        :templates="templates"
        @launch-group="handlePaletteLaunchGroup"
        @launch-template="handlePaletteLaunchTemplate"
        @close="showPalette = false"
      />
    </transition>

    <!-- Hard Freeze warning banner -->
    <transition name="freeze-banner">
      <div v-if="freezeWarning" class="freeze-warning-banner" role="alert">
        <span class="freeze-warning-icon">❄</span>
        <div class="freeze-warning-body">
          <div class="freeze-warning-title">Hard Freeze incoming — "{{ freezeWarning.name }}"</div>
          <div class="freeze-warning-sub">Apps will close in {{ freezeWarning.secondsLeft }}s</div>
          <div class="freeze-warning-track">
            <div class="freeze-warning-bar" :style="{ width: `${Math.min(100, (freezeWarning.secondsLeft / 120) * 100)}%` }" />
          </div>
        </div>
      </div>
    </transition>

    <!-- Toast (with optional undo action) -->
    <transition name="fade">
      <div
        v-if="toast"
        class="toast"
        :class="[toast.type, toast.action ? 'toast-with-action' : '']"
        role="status"
        aria-live="polite"
      >
        <span>{{ toast.message }}</span>
        <button
          v-if="toast.action"
          class="toast-action-btn"
          @click="toast.action.fn()"
        >{{ toast.action.label }}</button>
      </div>
    </transition>
  </div>
</template>

<style scoped>
#app-root {
  height: 100vh;
  display: flex;
  flex-direction: column;
  background: var(--bg-base);
}
</style>

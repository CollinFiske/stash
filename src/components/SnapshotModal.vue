<script setup lang="ts">
import { ref, computed, watch } from "vue";
import { open } from "@tauri-apps/plugin-dialog";
import type { CapturedApp } from "../types";
import AppIcon from "./AppIcon.vue";

const props = defineProps<{
  apps: CapturedApp[];
  loading: boolean;
}>();

const emit = defineEmits<{
  save: [name: string, apps: CapturedApp[]];
  cancel: [];
}>();

const groupName = ref("");
const trimmed = computed(() => groupName.value.trim());
const editMode = ref(false);
const editableApps = ref<CapturedApp[]>([]);

watch(() => props.apps, (newApps) => {
  editableApps.value = [...newApps];
}, { immediate: true });

function removeApp(index: number) {
  editableApps.value.splice(index, 1);
}

// ── Add-app form (edit mode) ──────────────────────────────────────────────────
const newPath = ref("");
const newName = ref("");
const newType = ref<"regular" | "vscode" | "browser" | "terminal">("regular");
const newContext = ref("");
const newUrl = ref("");
const newWorkingDir = ref("");

/** Terminal emulators and shells that accept a starting directory. */
const TERMINALS = [
  "cmd", "powershell", "pwsh", "wt", "windowsterminal", "conhost",
  "bash", "zsh", "fish", "alacritty", "wezterm", "kitty", "hyper",
  "tabby", "mintty", "gnome-terminal", "konsole", "xterm", "iterm",
];

function inferName(path: string): string {
  const parts = path.replace(/\\/g, "/").split("/");
  return parts[parts.length - 1].replace(/\.exe$/i, "").trim() || path;
}

function autoDetect() {
  const p = newPath.value.toLowerCase();
  const stem = inferName(newPath.value).toLowerCase();
  if (p === "code" || p.includes("\\code.exe") || p.includes("code - insiders")) {
    newType.value = "vscode";
  } else if (["chrome", "firefox", "msedge", "brave", "safari", "arc"].some((b) => p.includes(b))) {
    newType.value = "browser";
  } else if (TERMINALS.includes(stem)) {
    newType.value = "terminal";
  } else {
    newType.value = "regular";
  }
  if (!newName.value) newName.value = inferName(newPath.value);
}

async function browseFolder() {
  try {
    const selected = await open({ directory: true, multiple: false, title: "Select Starting Directory" });
    if (typeof selected === "string" && selected) newWorkingDir.value = selected;
  } catch {
    // cancelled
  }
}

async function browseFile() {
  try {
    const selected = await open({
      multiple: false,
      title: "Select Application",
      filters: [
        { name: "Executables", extensions: ["exe", "bat", "cmd", "app"] },
        { name: "All Files", extensions: ["*"] },
      ],
    });
    if (typeof selected === "string" && selected) {
      newPath.value = selected;
      if (!newName.value) newName.value = inferName(selected);
      autoDetect();
    }
  } catch {
    // cancelled
  }
}

function addApp() {
  const path = newPath.value.trim();
  if (!path) return;
  editableApps.value.push({
    name: newName.value.trim() || inferName(path),
    exe_path: path,
    title: newName.value.trim() || inferName(path),
    is_vscode: newType.value === "vscode",
    is_browser: newType.value === "browser",
    is_terminal: newType.value === "terminal",
    context_path: newType.value === "vscode" ? newContext.value.trim() || null : null,
    target_url: newType.value === "browser" ? newUrl.value.trim() || null : null,
    working_dir: newType.value === "terminal" ? newWorkingDir.value.trim() || null : null,
    relative_position: { x: 0.0, y: 0.0, w: 0.5, h: 0.8 },
    browser_window: null,
  });
  newPath.value = "";
  newName.value = "";
  newType.value = "regular";
  newContext.value = "";
  newUrl.value = "";
  newWorkingDir.value = "";
}

function appSublabel(app: CapturedApp): string | null {
  if (app.is_vscode && app.context_path) return app.context_path;
  if (app.is_terminal && app.working_dir) return `cd ${app.working_dir}`;
  // With the extension connected, show what the browser window actually holds.
  if (app.is_browser && app.browser_window) {
    const n = app.browser_window.tabs.length;
    const groups = app.browser_window.groups.map((g) => g.title).filter(Boolean);
    const groupPart = groups.length ? ` · ${groups.join(", ")}` : "";
    return `${n} tab${n !== 1 ? "s" : ""}${groupPart}`;
  }
  if (app.is_browser && app.target_url) {
    try { return new URL(app.target_url).hostname; } catch { return app.target_url; }
  }
  return null;
}

/** Short type badge shown on the right of each captured row. */
function appBadge(app: CapturedApp): string | null {
  if (app.is_vscode) return "VS Code";
  if (app.is_browser) return app.browser_window ? "tabs saved" : "browser";
  if (app.is_terminal) return app.working_dir ? "terminal" : "terminal (no dir)";
  return null;
}

function handleSave() {
  if (trimmed.value && editableApps.value.length > 0) {
    emit("save", trimmed.value, editableApps.value);
  }
}
</script>

<template>
  <div class="modal-overlay" @click.self="emit('cancel')">
    <transition name="slide" appear>
      <div class="modal" role="dialog" aria-modal="true" aria-labelledby="modal-title">
        <div class="modal-header">
          <span class="modal-title" id="modal-title">Save Workspace Snapshot</span>
          <button class="btn-icon" @click="emit('cancel')" aria-label="Close">✕</button>
        </div>

        <div class="modal-body">
          <div class="form-group">
            <label class="form-label" for="group-name">Workspace Name</label>
            <input
              id="group-name"
              class="form-input"
              type="text"
              v-model="groupName"
              placeholder="e.g. Research, Deep Work…"
              @keydown.enter="handleSave"
              autofocus
              :disabled="loading"
            />
          </div>

          <div class="form-group">
            <span class="form-label">
              {{ editMode ? "Edit Windows" : "Captured Windows" }}
              ({{ editableApps.length }})
            </span>

            <div v-if="loading" style="display:flex;align-items:center;gap:10px;padding:10px 0;">
              <span class="spinner"></span>
              <span style="color:var(--text-muted)">Scanning windows…</span>
            </div>
            <div v-else-if="editableApps.length === 0" style="color:var(--text-muted);padding:8px 0;">
              No windows. Add one below or cancel.
            </div>
            <div v-else class="captured-list">
              <div
                v-for="(app, i) in editableApps"
                :key="app.exe_path + app.title + i"
                class="captured-item"
              >
                <span class="captured-icon">
                  <AppIcon :name="app.name" :path="app.exe_path" :is-vscode="app.is_vscode" :is-browser="app.is_browser" :size="20" />
                </span>
                <div class="captured-info">
                  <div class="captured-name">{{ app.name }}</div>
                  <div v-if="appSublabel(app)" class="captured-sub">{{ appSublabel(app) }}</div>
                </div>
                <span v-if="appBadge(app)" class="app-chip-badge">{{ appBadge(app) }}</span>
                <button
                  v-if="editMode"
                  class="btn-icon snapshot-remove-btn"
                  @click="removeApp(i)"
                  aria-label="Remove"
                  title="Remove from snapshot"
                >✕</button>
              </div>
            </div>
          </div>

          <!-- Add-app panel — only visible in edit mode -->
          <div v-if="editMode" class="add-app-panel">
            <span class="form-label" style="display:block;margin-bottom:10px;">Add an App</span>
            <div class="add-app-grid">
              <div class="form-group" style="margin-bottom:0;">
                <label class="form-label add-app-label">Executable path or command</label>
                <div class="browse-row">
                  <input
                    class="form-input"
                    v-model="newPath"
                    placeholder="code, chrome, C:\Path\To\App.exe…"
                    @input="autoDetect"
                    @keydown.enter="addApp"
                  />
                  <button class="btn btn-ghost browse-btn" type="button" @click="browseFile" title="Browse for file">📂</button>
                </div>
              </div>
              <div style="display:grid;grid-template-columns:1fr 1fr;gap:8px;">
                <div class="form-group" style="margin-bottom:0;">
                  <label class="form-label add-app-label">Display name</label>
                  <input class="form-input" v-model="newName" placeholder="auto-detected" />
                </div>
                <div class="form-group" style="margin-bottom:0;">
                  <label class="form-label add-app-label">Type</label>
                  <select class="form-input form-select" v-model="newType">
                    <option value="regular">Regular App</option>
                    <option value="vscode">VS Code</option>
                    <option value="browser">Browser</option>
                    <option value="terminal">Terminal</option>
                  </select>
                </div>
              </div>
              <div v-if="newType === 'vscode'" class="form-group" style="margin-bottom:0;">
                <label class="form-label add-app-label">Workspace folder (optional)</label>
                <input class="form-input" v-model="newContext" placeholder="C:\Projects\my-app" />
              </div>
              <div v-if="newType === 'browser'" class="form-group" style="margin-bottom:0;">
                <label class="form-label add-app-label">URL (optional)</label>
                <input class="form-input" v-model="newUrl" placeholder="https://github.com/pulls" />
              </div>
              <div v-if="newType === 'terminal'" class="form-group" style="margin-bottom:0;">
                <label class="form-label add-app-label">Starting directory (optional)</label>
                <div class="browse-row">
                  <input class="form-input" v-model="newWorkingDir" placeholder="C:\Projects\my-app" />
                  <button class="btn btn-ghost browse-btn" type="button" @click="browseFolder" title="Browse for folder">📂</button>
                </div>
              </div>
            </div>
            <button class="btn btn-ghost" style="margin-top:10px;" @click="addApp" :disabled="!newPath.trim()">
              + Add App
            </button>
          </div>
        </div>

        <div class="modal-footer">
          <button class="btn btn-ghost" @click="emit('cancel')" :disabled="loading">Cancel</button>
          <button
            class="btn btn-ghost"
            @click="editMode = !editMode"
            :disabled="loading"
            style="margin-left:auto;"
          >
            {{ editMode ? "Done Editing" : "Edit" }}
          </button>
          <button
            class="btn btn-primary"
            @click="handleSave"
            :disabled="loading || !trimmed || editableApps.length === 0"
          >
            <span v-if="loading" class="spinner" style="width:12px;height:12px;"></span>
            Save Workspace
          </button>
        </div>
      </div>
    </transition>
  </div>
</template>

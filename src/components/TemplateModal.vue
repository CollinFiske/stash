<script setup lang="ts">
import { ref } from "vue";
import { open } from "@tauri-apps/plugin-dialog";
import type { ProjectTemplate, TemplateEntry } from "../types";
import AppIcon from "./AppIcon.vue";

const props = defineProps<{ saving: boolean; initial?: ProjectTemplate }>();

const emit = defineEmits<{
  save: [template: ProjectTemplate];
  cancel: [];
}>();

const isEdit = !!props.initial;
const templateName = ref(props.initial?.name ?? "");
const entries = ref<TemplateEntry[]>(props.initial ? [...props.initial.entries] : []);
const tags = ref<string[]>([...(props.initial?.tags ?? [])]);
const newTag = ref("");

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

function addEntry() {
  const path = newPath.value.trim();
  if (!path) return;
  entries.value.push({
    name: newName.value.trim() || inferName(path),
    exe_path: path,
    is_vscode: newType.value === "vscode",
    is_browser: newType.value === "browser",
    is_terminal: newType.value === "terminal",
    context_path: newType.value === "vscode" ? newContext.value.trim() || null : null,
    target_url: newType.value === "browser" ? newUrl.value.trim() || null : null,
    working_dir: newType.value === "terminal" ? newWorkingDir.value.trim() || null : null,
  });
  newPath.value = "";
  newName.value = "";
  newType.value = "regular";
  newContext.value = "";
  newUrl.value = "";
  newWorkingDir.value = "";
}

function removeEntry(i: number) {
  entries.value.splice(i, 1);
}

function addTag() {
  const t = newTag.value.trim().toLowerCase();
  if (t && !tags.value.includes(t)) {
    tags.value.push(t);
  }
  newTag.value = "";
}

function removeTag(i: number) {
  tags.value.splice(i, 1);
}

function handleTagKeydown(e: KeyboardEvent) {
  if (e.key === "Enter") {
    e.preventDefault();
    addTag();
  }
}

function handleSave() {
  const name = templateName.value.trim();
  if (!name || entries.value.length === 0) return;
  emit("save", {
    name,
    created_at: props.initial?.created_at ?? new Date().toISOString(),
    entries: [...entries.value],
    tags: tags.value.length > 0 ? [...tags.value] : undefined,
  });
}
</script>

<template>
  <div class="modal-overlay" @click.self="emit('cancel')">
    <transition name="slide" appear>
      <div class="modal" role="dialog" aria-modal="true" aria-labelledby="tmpl-modal-title">
        <div class="modal-header">
          <span class="modal-title" id="tmpl-modal-title">{{ isEdit ? "Edit Template" : "New Project Template" }}</span>
          <button class="btn-icon" @click="emit('cancel')" aria-label="Close">✕</button>
        </div>

        <div class="modal-body">
          <div class="form-group">
            <label class="form-label" for="tmpl-name">Template Name <span style="color: red;">*</span></label>
            <input
              id="tmpl-name"
              class="form-input"
              v-model="templateName"
              placeholder="e.g. Frontend Dev, Code Review, Research…"
              autofocus
            />
          </div>

          <!-- Tags -->
          <div class="form-group">
            <span class="form-label">Tags</span>
            <div v-if="tags.length > 0" class="tag-edit-pills">
              <span v-for="(tag, i) in tags" :key="tag" class="tag-edit-pill">
                {{ tag }}
                <button class="tag-remove-btn" @click="removeTag(i)" aria-label="Remove tag">✕</button>
              </span>
            </div>
            <div class="browse-row" style="margin-top:6px;">
              <input
                class="form-input"
                v-model="newTag"
                placeholder="Add a tag (e.g. work, client, dev)…"
                @keydown="handleTagKeydown"
              />
              <button class="btn btn-ghost browse-btn" @click="addTag" :disabled="!newTag.trim()">Add</button>
            </div>
          </div>

          <div class="form-group" v-if="entries.length > 0">
            <span class="form-label">Apps ({{ entries.length }})</span>
            <div class="template-entries">
              <div v-for="(entry, i) in entries" :key="i" class="template-entry">
                <span class="captured-icon">
                  <AppIcon :name="entry.name" :path="entry.exe_path" :is-vscode="entry.is_vscode" :is-browser="entry.is_browser" :size="20" />
                </span>
                <div class="captured-info">
                  <div class="captured-name">{{ entry.name }}</div>
                  <div v-if="entry.context_path" class="captured-sub tmpl-sub">workspace: {{ entry.context_path }}</div>
                  <div v-else-if="entry.working_dir" class="captured-sub tmpl-sub">cd: {{ entry.working_dir }}</div>
                  <div v-else-if="entry.target_url" class="captured-sub tmpl-sub">{{ entry.target_url }}</div>
                  <div v-else class="captured-sub tmpl-sub" style="opacity:0.5;">{{ entry.exe_path }}</div>
                </div>
                <button class="btn-icon" @click="removeEntry(i)" aria-label="Remove" title="Remove">✕</button>
              </div>
            </div>
          </div>

          <div class="add-app-panel">
            <span class="form-label" style="display:block;margin-bottom:10px;">Add an App</span>
            <div class="add-app-grid">
              <div class="form-group" style="margin-bottom:0;">
                <label class="form-label add-app-label">Executable path or command <span style="color: red;">*</span></label>
                <div class="browse-row">
                  <input
                    class="form-input"
                    v-model="newPath"
                    placeholder="code, chrome, C:\Path\To\App.exe…"
                    @input="autoDetect"
                    @keydown.enter="addEntry"
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
            <button class="btn btn-ghost" style="margin-top:10px;" @click="addEntry" :disabled="!newPath.trim()">
              + Add App
            </button>
          </div>
        </div>

        <div class="modal-footer">
          <button class="btn btn-ghost" @click="emit('cancel')" :disabled="props.saving">Cancel</button>
          <button
            class="btn btn-template-primary"
            @click="handleSave"
            :disabled="props.saving || !templateName.trim() || entries.length === 0"
          >
            <span v-if="props.saving" class="spinner" style="width:12px;height:12px;"></span>
            {{ isEdit ? "Save Changes" : "Save Template" }}
          </button>
        </div>
      </div>
    </transition>
  </div>
</template>

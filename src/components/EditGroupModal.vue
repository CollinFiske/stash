<script setup lang="ts">
import { ref } from "vue";
import type { SavedGroup } from "../types";
import AppIcon from "./AppIcon.vue";

const props = defineProps<{ group: SavedGroup }>();

const emit = defineEmits<{
  save: [updated: SavedGroup];
  cancel: [];
}>();

const name = ref(props.group.name);
const apps = ref(props.group.apps.map((a) => ({ ...a })));
const tags = ref<string[]>([...(props.group.tags ?? [])]);
const newTag = ref("");

function removeApp(i: number) {
  apps.value.splice(i, 1);
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
  const n = name.value.trim();
  if (!n) return;
  emit("save", {
    ...props.group,
    name: n,
    apps: [...apps.value],
    tags: tags.value.length > 0 ? [...tags.value] : undefined,
  });
}
</script>

<template>
  <div class="modal-overlay" @click.self="emit('cancel')">
    <transition name="slide" appear>
      <div class="modal" role="dialog" aria-modal="true" aria-labelledby="edit-group-title">
        <div class="modal-header">
          <span class="modal-title" id="edit-group-title">Edit Workspace</span>
          <button class="btn-icon" @click="emit('cancel')" aria-label="Close">✕</button>
        </div>

        <div class="modal-body">
          <div class="form-group">
            <label class="form-label" for="edit-group-name">Workspace Name</label>
            <input
              id="edit-group-name"
              class="form-input"
              v-model="name"
              placeholder="Workspace name"
              autofocus
              @keydown.enter="handleSave"
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

          <div class="form-group">
            <span class="form-label">Apps ({{ apps.length }})</span>
            <div v-if="apps.length === 0" style="color:var(--text-muted);padding:8px 0;font-size:0.85rem;">
              No apps remaining.
            </div>
            <div v-else class="captured-list">
              <div
                v-for="(app, i) in apps"
                :key="app.exe_path + i"
                class="captured-item"
              >
                <span class="captured-icon">
                  <AppIcon :name="app.name" :path="app.exe_path" :is-vscode="app.is_vscode" :is-browser="app.is_browser" :size="20" />
                </span>
                <div class="captured-info">
                  <div class="captured-name">{{ app.name }}</div>
                  <div v-if="app.is_vscode && app.context_path" class="captured-sub">{{ app.context_path }}</div>
                  <div v-else-if="app.is_browser && app.target_url" class="captured-sub">{{ app.target_url }}</div>
                </div>
                <button class="btn-icon" @click="removeApp(i)" aria-label="Remove" title="Remove">✕</button>
              </div>
            </div>
          </div>
        </div>

        <div class="modal-footer">
          <button class="btn btn-ghost" @click="emit('cancel')">Cancel</button>
          <button
            class="btn btn-primary"
            @click="handleSave"
            :disabled="!name.trim() || apps.length === 0"
          >
            Save Changes
          </button>
        </div>
      </div>
    </transition>
  </div>
</template>

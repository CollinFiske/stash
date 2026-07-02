<script setup lang="ts">
import type { SavedGroup } from "../types";
import AppIcon from "./AppIcon.vue";
import KebabMenu from "./KebabMenu.vue";

defineProps<{
  group: SavedGroup;
  launching: boolean;
  active: boolean;
  dragging?: boolean;
  dragOver?: boolean;
}>();

const emit = defineEmits<{
  launch: [];
  delete: [];
  edit: [];
  "close-apps": [];
  "freeze-click": [];
  "drag-start": [e: DragEvent];
  "drag-over": [e: DragEvent];
  "drop": [e: DragEvent];
  "drag-end": [];
}>();

const TAG_COLORS = [
  { bg: "rgba(99,102,241,0.15)", border: "rgba(99,102,241,0.4)", text: "#818cf8" },
  { bg: "rgba(16,185,129,0.12)", border: "rgba(16,185,129,0.4)", text: "#34d399" },
  { bg: "rgba(245,158,11,0.12)", border: "rgba(245,158,11,0.4)", text: "#fbbf24" },
  { bg: "rgba(239,68,68,0.12)", border: "rgba(239,68,68,0.4)", text: "#f87171" },
  { bg: "rgba(6,182,212,0.12)", border: "rgba(6,182,212,0.4)", text: "#22d3ee" },
  { bg: "rgba(168,85,247,0.12)", border: "rgba(168,85,247,0.4)", text: "#c084fc" },
];

function tagColor(tag: string) {
  let hash = 0;
  for (const ch of tag) hash = (hash * 31 + ch.charCodeAt(0)) | 0;
  return TAG_COLORS[Math.abs(hash) % TAG_COLORS.length];
}

function formatDate(iso: string): string {
  try {
    return new Intl.DateTimeFormat(undefined, {
      month: "short", day: "numeric", year: "numeric",
      hour: "2-digit", minute: "2-digit",
    }).format(new Date(iso));
  } catch {
    return iso;
  }
}

function formatLastUsed(iso: string): string {
  try {
    const diff = Date.now() - new Date(iso).getTime();
    const mins = Math.floor(diff / 60000);
    if (mins < 1) return "just now";
    if (mins < 60) return `${mins}m ago`;
    const hrs = Math.floor(mins / 60);
    if (hrs < 24) return `${hrs}h ago`;
    const days = Math.floor(hrs / 24);
    return `${days}d ago`;
  } catch {
    return "";
  }
}

function chipLabel(app: { name: string; context_path: string | null; target_url: string | null; is_vscode: boolean; is_browser: boolean }): string {
  if (app.is_vscode && app.context_path) {
    const parts = app.context_path.replace(/\\/g, "/").split("/");
    return parts[parts.length - 1] || app.name;
  }
  if (app.is_browser && app.target_url) {
    try { return new URL(app.target_url).hostname; } catch { return app.name; }
  }
  return app.name;
}
</script>

<template>
  <article
    class="group-card"
    :class="{
      'card-active': active,
      'card-dragging': dragging,
      'card-drag-over': dragOver,
    }"
    draggable="true"
    @dragstart="emit('drag-start', $event)"
    @dragover.prevent="emit('drag-over', $event)"
    @dragenter.prevent
    @drop.prevent="emit('drop', $event)"
    @dragend="emit('drag-end')"
  >
    <div class="card-top">
      <div class="card-meta-count">
        <span class="card-count-num">{{ group.apps.length }}</span>
        <span class="card-count-label">app{{ group.apps.length !== 1 ? "s" : "" }}</span>
        <div v-if="active" class="active-badge">● Active</div>
      </div>
      <div class="card-top-center">
        <div class="card-name">{{ group.name }}</div>
        <div class="card-date">{{ formatDate(group.created_at) }}</div>
        <div v-if="group.last_used" class="card-last-used">
          used {{ formatLastUsed(group.last_used) }}
          <span v-if="(group.launch_count ?? 0) > 0"> · {{ group.launch_count }}×</span>
        </div>
      </div>
      <div class="card-top-right">
        <button
          v-if="active"
          class="btn btn-close-apps"
          @click="emit('close-apps')"
          :disabled="launching"
          title="Close all apps in this workspace"
        >✕ Close Apps</button>
        <button
          class="freeze-icon-btn"
          :class="{
            'freeze-icon-active': group.hard_freeze?.enabled,
            'freeze-icon-set': group.hard_freeze && !group.hard_freeze.enabled,
          }"
          @click="emit('freeze-click')"
          :title="group.hard_freeze ? (group.hard_freeze.enabled ? `Hard Freeze at ${group.hard_freeze.time}` : 'Hard Freeze (disabled)') : 'Set Hard Freeze'"
          aria-label="Configure Hard Freeze"
        >
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="12" cy="12" r="9"/>
            <polyline points="12 7 12 12 15 15"/>
          </svg>
        </button>
        <KebabMenu @edit="emit('edit')" @delete="emit('delete')" />
      </div>
    </div>

    <!-- Tags -->
    <div v-if="group.tags && group.tags.length > 0" class="card-tags">
      <span
        v-for="tag in group.tags"
        :key="tag"
        class="tag-pill"
        :style="{
          background: tagColor(tag).bg,
          borderColor: tagColor(tag).border,
          color: tagColor(tag).text,
        }"
      >{{ tag }}</span>
    </div>

    <div class="card-apps">
      <div
        v-for="app in group.apps"
        :key="app.exe_path + app.title"
        class="app-chip"
        :title="app.title"
      >
        <AppIcon :name="app.name" :path="app.exe_path" :is-vscode="app.is_vscode" :is-browser="app.is_browser" :size="16" />
        <span class="app-chip-label">{{ chipLabel(app) }}</span>
        <span v-if="app.is_vscode" class="app-chip-badge">vsc</span>
        <span v-else-if="app.is_browser" class="app-chip-badge">web</span>
      </div>
    </div>

    <div class="card-footer">
      <button
        class="btn btn-launch"
        @click="emit('launch')"
        :disabled="launching"
      >
        <span v-if="launching" class="spinner" style="width:12px;height:12px;"></span>
        <svg v-else class="btn-svg launch-icon" viewBox="0 0 24 24" fill="currentColor" stroke="none">
          <polygon points="5 3 19 12 5 21 5 3"/>
        </svg>
        {{ launching ? "Launching…" : "Launch" }}
      </button>
    </div>
  </article>
</template>

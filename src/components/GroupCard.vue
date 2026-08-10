<script setup lang="ts">
import type { SavedGroup, CapturedApp } from "../types";
import AppIcon from "./AppIcon.vue";
import KebabMenu from "./KebabMenu.vue";
import { baseName, hostOf, tabSummary, groupTitles } from "../paths";
import { tagColor, formatDate, formatLastUsed, freezeLabel, freezeTitle } from "../cardFormat";

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

function chipLabel(app: CapturedApp): string {
  if (app.is_vscode && app.context_path) {
    return baseName(app.context_path) || app.name;
  }
  // A captured browser window carries its whole tab list — that is far more
  // useful on a card than the URL of whichever tab happened to be focused.
  if (app.is_browser && app.browser_window) {
    return tabSummary(app.browser_window);
  }
  if (app.is_browser && app.target_url) {
    return hostOf(app.target_url);
  }
  if (app.is_terminal && app.working_dir) {
    return baseName(app.working_dir) || app.name;
  }
  return app.name;
}

/** Named tab groups inside a captured browser window, for the chip tooltip. */
function chipTitle(app: CapturedApp): string {
  if (app.is_terminal && app.working_dir) return app.working_dir;
  if (app.is_vscode && app.context_path) return app.context_path;
  if (app.is_browser && app.browser_window) {
    const groups = groupTitles(app.browser_window);
    const heading = groups.length ? `Groups: ${groups.join(", ")}\n` : "";
    return heading + app.browser_window.tabs.slice(0, 12).map((t) => t.title || t.url).join("\n");
  }
  return app.title;
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
        <KebabMenu
          :show-close-apps="active"
          :close-apps-disabled="launching"
          show-hard-freeze
          :hard-freeze-label="freezeLabel(group.hard_freeze)"
          :hard-freeze-title="freezeTitle(group.hard_freeze)"
          :hard-freeze-on="group.hard_freeze?.enabled"
          @close-apps="emit('close-apps')"
          @hard-freeze="emit('freeze-click')"
          @edit="emit('edit')"
          @delete="emit('delete')"
        />
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
        :title="chipTitle(app)"
      >
        <AppIcon :name="app.name" :path="app.exe_path" :is-vscode="app.is_vscode" :is-browser="app.is_browser" :size="16" />
        <span class="app-chip-label">{{ chipLabel(app) }}</span>
        <span v-if="app.is_vscode" class="app-chip-badge">vsc</span>
        <span v-else-if="app.is_browser" class="app-chip-badge">web</span>
        <span v-else-if="app.is_terminal" class="app-chip-badge">cwd</span>
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

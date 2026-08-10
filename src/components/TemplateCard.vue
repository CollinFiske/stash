<script setup lang="ts">
import type { ProjectTemplate, TemplateEntry } from "../types";
import AppIcon from "./AppIcon.vue";
import KebabMenu from "./KebabMenu.vue";
import { baseName, hostOf } from "../paths";
import { tagColor, formatDate, formatLastUsed, freezeLabel, freezeTitle } from "../cardFormat";

defineProps<{
  template: ProjectTemplate;
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

function chipLabel(e: TemplateEntry): string {
  if (e.is_vscode && e.context_path) {
    return baseName(e.context_path) || e.name;
  }
  if (e.is_browser && e.target_url) {
    return hostOf(e.target_url);
  }
  if (e.is_terminal && e.working_dir) {
    return baseName(e.working_dir) || e.name;
  }
  return e.name;
}
</script>

<template>
  <article
    class="group-card template-card"
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
        <span class="card-count-num">{{ template.entries.length }}</span>
        <span class="card-count-label">app{{ template.entries.length !== 1 ? "s" : "" }}</span>
        <div v-if="active" class="active-badge active-badge-tmpl">● Live</div>
      </div>
      <div class="card-top-center">
        <div class="card-name">{{ template.name }}</div>
        <div class="card-date">{{ formatDate(template.created_at) }}</div>
        <div v-if="template.last_used" class="card-last-used">
          used {{ formatLastUsed(template.last_used) }}
          <span v-if="(template.launch_count ?? 0) > 0"> · {{ template.launch_count }}×</span>
        </div>
      </div>
      <div class="card-top-right">
        <KebabMenu
          :show-close-apps="active"
          :close-apps-disabled="launching"
          show-hard-freeze
          :hard-freeze-label="freezeLabel(template.hard_freeze)"
          :hard-freeze-title="freezeTitle(template.hard_freeze)"
          :hard-freeze-on="template.hard_freeze?.enabled"
          @close-apps="emit('close-apps')"
          @hard-freeze="emit('freeze-click')"
          @edit="emit('edit')"
          @delete="emit('delete')"
        />
      </div>
    </div>

    <!-- Tags -->
    <div v-if="template.tags && template.tags.length > 0" class="card-tags">
      <span
        v-for="tag in template.tags"
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
        v-for="entry in template.entries"
        :key="entry.exe_path + entry.name"
        class="app-chip"
        :title="entry.exe_path"
      >
        <AppIcon :name="entry.name" :path="entry.exe_path" :is-vscode="entry.is_vscode" :is-browser="entry.is_browser" :size="16" />
        <span class="app-chip-label">{{ chipLabel(entry) }}</span>
        <span v-if="entry.is_vscode" class="app-chip-badge tmpl-badge">vsc</span>
        <span v-else-if="entry.is_browser" class="app-chip-badge tmpl-badge">web</span>
        <span v-else-if="entry.is_terminal" class="app-chip-badge tmpl-badge">cwd</span>
      </div>
    </div>

    <div class="card-footer">
      <button
        class="btn btn-template-launch"
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

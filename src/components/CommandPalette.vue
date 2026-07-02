<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, nextTick } from "vue";
import type { SavedGroup, ProjectTemplate } from "../types";

const props = defineProps<{
  groups: Record<string, SavedGroup>;
  templates: Record<string, ProjectTemplate>;
}>();

const emit = defineEmits<{
  "launch-group": [name: string];
  "launch-template": [name: string];
  close: [];
}>();

const query = ref("");
const activeIndex = ref(0);
const searchInput = ref<HTMLInputElement | null>(null);

interface PaletteItem {
  type: "group" | "template";
  name: string;
  count: number;
  lastUsed?: string;
}

const allItems = computed<PaletteItem[]>(() => {
  const items: PaletteItem[] = [];
  for (const g of Object.values(props.groups)) {
    items.push({ type: "group", name: g.name, count: g.apps.length, lastUsed: g.last_used });
  }
  for (const t of Object.values(props.templates)) {
    items.push({ type: "template", name: t.name, count: t.entries.length, lastUsed: t.last_used });
  }
  // Sort by last used desc (most recent first)
  return items.sort((a, b) => {
    if (a.lastUsed && b.lastUsed) return b.lastUsed.localeCompare(a.lastUsed);
    if (a.lastUsed) return -1;
    if (b.lastUsed) return 1;
    return a.name.localeCompare(b.name);
  });
});

function exeBase(path: string): string {
  return path.replace(/\\/g, "/").split("/").pop()?.toLowerCase().replace(/\.exe$/i, "") ?? path.toLowerCase();
}

const filtered = computed<PaletteItem[]>(() => {
  const q = query.value.trim().toLowerCase();
  if (!q) return allItems.value;

  return allItems.value.filter((item) => {
    if (item.name.toLowerCase().includes(q)) return true;
    // Check app/entry exe names
    if (item.type === "group") {
      const g = props.groups[item.name];
      if (g?.apps.some((a) => exeBase(a.exe_path).includes(q) || a.name.toLowerCase().includes(q))) return true;
      if (g?.tags?.some((t) => t.includes(q))) return true;
    } else {
      const t = props.templates[item.name];
      if (t?.entries.some((e) => exeBase(e.exe_path).includes(q) || e.name.toLowerCase().includes(q))) return true;
      if (t?.tags?.some((t2) => t2.includes(q))) return true;
    }
    return false;
  });
});

function select(item: PaletteItem) {
  if (item.type === "group") {
    emit("launch-group", item.name);
  } else {
    emit("launch-template", item.name);
  }
}

function handleKeydown(e: KeyboardEvent) {
  if (e.key === "ArrowDown") {
    e.preventDefault();
    activeIndex.value = Math.min(activeIndex.value + 1, filtered.value.length - 1);
  } else if (e.key === "ArrowUp") {
    e.preventDefault();
    activeIndex.value = Math.max(activeIndex.value - 1, 0);
  } else if (e.key === "Enter") {
    e.preventDefault();
    const item = filtered.value[activeIndex.value];
    if (item) select(item);
  } else if (e.key === "Escape") {
    emit("close");
  }
}

function handleQueryInput() {
  activeIndex.value = 0;
}

function handleGlobalKey(e: KeyboardEvent) {
  if (e.key === "Escape") emit("close");
}

onMounted(async () => {
  document.addEventListener("keydown", handleGlobalKey);
  await nextTick();
  searchInput.value?.focus();
});

onUnmounted(() => {
  document.removeEventListener("keydown", handleGlobalKey);
});
</script>

<template>
  <div class="modal-overlay palette-overlay" @click.self="emit('close')">
    <div class="palette-modal" role="dialog" aria-modal="true" aria-label="Quick Launch">
      <div class="palette-search-wrap">
        <svg class="palette-search-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <circle cx="11" cy="11" r="8"/>
          <path d="m21 21-4.35-4.35"/>
        </svg>
        <input
          ref="searchInput"
          class="palette-search"
          v-model="query"
          placeholder="Search workspaces and templates…"
          @keydown="handleKeydown"
          @input="handleQueryInput"
        />
        <kbd class="palette-esc-hint">ESC</kbd>
      </div>

      <div class="palette-list" role="listbox">
        <div v-if="filtered.length === 0  && query.length === 0" class="palette-empty">
          No results
        </div>
        <div v-else-if="filtered.length === 0" class="palette-empty">
          No results for "{{ query }}"
        </div>
        <button
          v-for="(item, i) in filtered"
          :key="item.type + item.name"
          class="palette-item"
          :class="{ 'palette-item-active': i === activeIndex }"
          role="option"
          :aria-selected="i === activeIndex"
          @click="select(item)"
          @mouseenter="activeIndex = i"
        >
          <span class="palette-item-icon">{{ item.type === "group" ? "📦" : "📋" }}</span>
          <span class="palette-item-name">{{ item.name }}</span>
          <span class="palette-item-meta">{{ item.count }} app{{ item.count !== 1 ? "s" : "" }}</span>
          <span class="palette-item-type-badge" :class="item.type === 'group' ? 'badge-workspace' : 'badge-template'">
            {{ item.type === "group" ? "workspace" : "template" }}
          </span>
        </button>
      </div>

      <div class="palette-footer">
        <span>↑↓ navigate</span>
        <span>↵ launch</span>
        <span>Esc close</span>
      </div>
    </div>
  </div>
</template>

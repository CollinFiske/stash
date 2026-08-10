<script setup lang="ts">
import { ref, watch, onUnmounted } from "vue";

/**
 * Card overflow menu.  Edit/Delete are always present; the workspace actions
 * above them are opt-in so cards that have no notion of "running" or of a
 * freeze schedule keep the plain two-item menu.
 */
withDefaults(
  defineProps<{
    /** Show "Close Apps" — only meaningful while the workspace is running. */
    showCloseApps?: boolean;
    /** Grey out "Close Apps" (e.g. a launch is still in flight). */
    closeAppsDisabled?: boolean;
    /** Show the "Hard Freeze" entry. */
    showHardFreeze?: boolean;
    /** Freeze entry label — carries the schedule when one is set. */
    hardFreezeLabel?: string;
    /** Tooltip for the freeze entry. */
    hardFreezeTitle?: string;
    /** Highlight the freeze entry while a schedule is armed. */
    hardFreezeOn?: boolean;
  }>(),
  {
    showCloseApps: false,
    closeAppsDisabled: false,
    showHardFreeze: false,
    hardFreezeLabel: "Freeze",
    hardFreezeTitle: "",
    hardFreezeOn: false,
  },
);

const emit = defineEmits<{ edit: []; delete: []; closeApps: []; hardFreeze: [] }>();

const open = ref(false);
const wrapRef = ref<HTMLElement | null>(null);

function toggle(e: MouseEvent) {
  e.stopPropagation();
  open.value = !open.value;
}

function handleOutside(e: MouseEvent) {
  if (wrapRef.value && !wrapRef.value.contains(e.target as Node)) {
    open.value = false;
  }
}

watch(open, (val) => {
  if (val) document.addEventListener("click", handleOutside);
  else document.removeEventListener("click", handleOutside);
});

onUnmounted(() => document.removeEventListener("click", handleOutside));

function closeMenu() {
  open.value = false;
}

function doEdit() {
  emit("edit");
  closeMenu();
}

function doDelete() {
  emit("delete");
  closeMenu();
}

function doCloseApps() {
  emit("closeApps");
  closeMenu();
}

function doHardFreeze() {
  emit("hardFreeze");
  closeMenu();
}
</script>

<template>
  <div ref="wrapRef" class="kebab-wrap">
    <button
      class="btn-icon kebab-btn"
      @click="toggle"
      :aria-expanded="open"
      aria-label="Options"
    >⋮</button>
    <div v-if="open" class="kebab-menu" role="menu">
      <template v-if="showCloseApps || showHardFreeze">
        <button
          v-if="showCloseApps"
          class="kebab-item kebab-item-danger"
          @click="doCloseApps"
          :disabled="closeAppsDisabled"
          role="menuitem"
        >Close Apps</button>
        <button
          v-if="showHardFreeze"
          class="kebab-item"
          :class="{ 'kebab-item-on': hardFreezeOn }"
          @click="doHardFreeze"
          :title="hardFreezeTitle"
          role="menuitem"
        >{{ hardFreezeLabel }}</button>
        <div class="kebab-sep" role="separator"></div>
      </template>
      <button class="kebab-item" @click="doEdit" role="menuitem">Edit</button>
      <button class="kebab-item kebab-item-danger" @click="doDelete" role="menuitem">Delete</button>
    </div>
  </div>
</template>

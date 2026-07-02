<script setup lang="ts">
import { ref, watch, onUnmounted } from "vue";

const emit = defineEmits<{ edit: []; delete: [] }>();

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

function doEdit() { emit("edit"); open.value = false; }
function doDelete() { emit("delete"); open.value = false; }
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
      <button class="kebab-item" @click="doEdit" role="menuitem">Edit</button>
      <button class="kebab-item kebab-item-danger" @click="doDelete" role="menuitem">Delete</button>
    </div>
  </div>
</template>

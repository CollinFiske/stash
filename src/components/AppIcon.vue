<script setup lang="ts">
import { computed } from "vue";
import { getAppIcon } from "../appicons";

const props = defineProps<{
  name: string;
  path: string;
  isVscode?: boolean;
  isBrowser?: boolean;
  size?: number;
}>();

const svg = computed(() =>
  getAppIcon(props.name, props.path, props.isVscode ?? false, props.isBrowser ?? false)
);
</script>

<template>
  <span
    class="app-icon"
    :style="`width:${size ?? 18}px;height:${size ?? 18}px`"
    v-html="svg"
  />
</template>

<style scoped>
.app-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  overflow: hidden;
  border-radius: 3px;
  line-height: 0;
}
.app-icon :deep(svg) {
  width: 100%;
  height: 100%;
  display: block;
}
</style>

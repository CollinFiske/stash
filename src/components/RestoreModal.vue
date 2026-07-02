<script setup lang="ts">
import type { RestoreSummary } from "../types";

defineProps<{
  summary: RestoreSummary;
}>();

const emit = defineEmits<{
  close: [];
}>();
</script>

<template>
  <div class="modal-overlay" @click.self="emit('close')">
    <transition name="slide" appear>
      <div class="modal" role="dialog" aria-modal="true" aria-labelledby="restore-title">
        <div class="modal-header">
          <span class="modal-title" id="restore-title">Restore Results</span>
          <button class="btn-icon" @click="emit('close')" aria-label="Close">✕</button>
        </div>

        <div class="modal-body">
          <div class="result-summary">
            {{ summary.succeeded }} of {{ summary.total }} apps launched successfully
            <span v-if="summary.failed > 0" style="color:var(--danger)">
              · {{ summary.failed }} failed
            </span>
          </div>

          <div class="result-list">
            <div
              v-for="r in summary.results"
              :key="r.app_name"
              class="result-item"
              :class="r.success ? 'ok' : 'fail'"
            >
              <span class="result-dot" :class="r.success ? 'ok' : 'fail'"></span>
              <span>{{ r.app_name }}</span>
              <span v-if="r.error" class="result-error" :title="r.error">
                {{ r.error }}
              </span>
            </div>
          </div>

          <p v-if="summary.succeeded > 0" style="font-size:0.8rem;color:var(--text-muted);">
            Windows are being repositioned in the background. This may take a few seconds.
          </p>
        </div>

        <div class="modal-footer">
          <button class="btn btn-primary" @click="emit('close')">Done</button>
        </div>
      </div>
    </transition>
  </div>
</template>

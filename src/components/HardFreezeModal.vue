<script setup lang="ts">
import { ref, computed } from "vue";
import type { HardFreeze } from "../types";

const props = defineProps<{
  name: string;
  appCount: number;
  existing?: HardFreeze;
}>();

const emit = defineEmits<{
  save: [freeze: HardFreeze];
  remove: [];
  cancel: [];
}>();

const isNew = !props.existing;

const enabled = ref(props.existing?.enabled ?? true);
const time = ref(props.existing?.time ?? "17:30");
const days = ref<number[]>(props.existing?.days ?? []);
const warningSeconds = ref(props.existing?.warning_seconds ?? 60);

const DAY_LABELS = ["S", "M", "T", "W", "T", "F", "S"];
const DAY_FULL = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

function toggleDay(d: number) {
  const idx = days.value.indexOf(d);
  if (idx === -1) days.value = [...days.value, d].sort((a, b) => a - b);
  else days.value = days.value.filter((x) => x !== d);
}

const daysLabel = computed(() => {
  if (days.value.length === 0) return "every day";
  return days.value.map((d) => DAY_FULL[d]).join(", ");
});

const warningLabel = computed(() => {
  if (warningSeconds.value < 60) return `${warningSeconds.value}s`;
  if (warningSeconds.value === 60) return "1 min";
  return `${warningSeconds.value}s`;
});

function handleSave() {
  emit("save", {
    time: time.value,
    days: [...days.value],
    warning_seconds: warningSeconds.value,
    enabled: enabled.value,
  });
}
</script>

<template>
  <div class="modal-overlay" @click.self="emit('cancel')">
    <transition name="slide" appear>
      <div class="modal hf-modal" role="dialog" aria-modal="true" aria-labelledby="hf-title">

        <!-- Header -->
        <div class="modal-header">
          <div style="display:flex;align-items:center;gap:8px;">
            <span class="hf-header-icon">❄</span>
            <span class="modal-title" id="hf-title">Hard Freeze</span>
            <span class="hf-target-chip">{{ name }}</span>
          </div>
          <button class="btn-icon" @click="emit('cancel')" aria-label="Close">✕</button>
        </div>

        <div class="modal-body">

          <!-- Enable toggle -->
          <div class="hf-toggle-row">
            <div>
              <div class="hf-toggle-label">{{ enabled ? "Enabled" : "Disabled" }}</div>
              <div class="hf-toggle-sub">{{ enabled ? "Will trigger at the scheduled time" : "Frozen but inactive" }}</div>
            </div>
            <button
              class="hf-toggle"
              :class="{ 'hf-toggle-on': enabled }"
              @click="enabled = !enabled"
              role="switch"
              :aria-checked="enabled"
              aria-label="Toggle Hard Freeze"
            >
              <span class="hf-toggle-thumb" />
            </button>
          </div>

          <!-- Time picker -->
          <div class="hf-section">
            <div class="hf-section-label">Trigger at</div>
            <input
              class="form-input hf-time-input"
              type="time"
              v-model="time"
              :disabled="!enabled"
            />
          </div>

          <!-- Day chips -->
          <div class="hf-section">
            <div class="hf-section-label">
              Days
              <span class="hf-days-label">{{ daysLabel }}</span>
            </div>
            <div class="hf-day-chips">
              <button
                v-for="(label, d) in DAY_LABELS"
                :key="d"
                class="hf-day-chip"
                :class="{ 'hf-day-chip-on': days.length === 0 || days.includes(d), 'hf-day-chip-disabled': !enabled }"
                @click="enabled && toggleDay(d)"
                :aria-pressed="days.length === 0 || days.includes(d)"
                :title="DAY_FULL[d]"
              >{{ label }}</button>
            </div>
            <button
              v-if="days.length > 0"
              class="hf-link-btn"
              @click="enabled && (days = [])"
              :disabled="!enabled"
            >
              Reset to every day
            </button>
          </div>

          <!-- Warning slider -->
          <div class="hf-section">
            <div class="hf-section-label">
              Warning countdown
              <span class="hf-days-label">{{ warningLabel }} before trigger</span>
            </div>
            <div class="hf-slider-row">
              <span class="hf-slider-bound">30s</span>
              <input
                class="hf-slider"
                type="range"
                min="30"
                max="120"
                step="10"
                v-model.number="warningSeconds"
                :disabled="!enabled"
              />
              <span class="hf-slider-bound">2m</span>
            </div>
          </div>

          <!-- Info card -->
          <div class="hf-info-card">
            <span class="hf-info-icon">ℹ</span>
            <div class="hf-info-text">
              When triggered, <strong>all {{ appCount }} app{{ appCount !== 1 ? "s" : "" }}</strong>
              in <strong>"{{ name }}"</strong> will be force-closed.
              You'll get a {{ warningLabel }} heads-up first.
            </div>
          </div>

        </div>

        <!-- Footer -->
        <div class="modal-footer hf-footer">
          <button v-if="!isNew" class="btn hf-btn-remove" @click="emit('remove')">
            Remove Freeze
          </button>
          <div style="display:flex;gap:8px;margin-left:auto;">
            <button class="btn btn-ghost" @click="emit('cancel')">Cancel</button>
            <button class="btn hf-btn-save" @click="handleSave">
              {{ isNew ? "Create Freeze" : "Save Changes" }}
            </button>
          </div>
        </div>

      </div>
    </transition>
  </div>
</template>

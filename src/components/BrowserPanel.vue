<script setup lang="ts">
import { ref, computed } from "vue";
import type { BridgeStatus, BrowserSnapshot } from "../types";

const props = defineProps<{
  status: BridgeStatus | null;
  snapshot: BrowserSnapshot | null;
}>();

const emit = defineEmits<{ close: []; refresh: [] }>();

const copied = ref(false);

const state = computed<"offline" | "waiting" | "connected">(() => {
  if (!props.status?.running) return "offline";
  return props.status.connected ? "connected" : "waiting";
});

const groupedWindows = computed(() =>
  (props.snapshot?.windows ?? []).map((w) => ({
    id: w.window_id,
    tabCount: w.tabs.length,
    groups: w.groups,
    // Show the first few tab titles as a preview of what the window holds.
    preview: w.tabs.slice(0, 4).map((t) => t.title || hostOf(t.url)),
    more: Math.max(0, w.tabs.length - 4),
  }))
);

function hostOf(url: string): string {
  try {
    return new URL(url).hostname;
  } catch {
    return url;
  }
}

async function copyToken() {
  if (!props.status?.token) return;
  try {
    await navigator.clipboard.writeText(props.status.token);
    copied.value = true;
    setTimeout(() => (copied.value = false), 1800);
  } catch {
    // Clipboard blocked — the token is selectable on screen as a fallback.
  }
}
</script>

<template>
  <div class="modal-overlay" @click.self="emit('close')">
    <transition name="slide" appear>
      <div class="modal" role="dialog" aria-modal="true" aria-labelledby="browser-title">
        <div class="modal-header">
          <div style="display:flex;align-items:center;gap:8px;">
            <span class="modal-title" id="browser-title">Browser Connection</span>
            <span class="bridge-chip" :class="`bridge-chip-${state}`">
              {{ state === "connected" ? "● Connected" : state === "waiting" ? "○ Waiting" : "○ Offline" }}
            </span>
          </div>
          <button class="btn-icon" @click="emit('close')" aria-label="Close">✕</button>
        </div>

        <div class="modal-body">
          <!-- Bridge not listening at all -->
          <div v-if="state === 'offline'" class="bridge-alert">
            <strong>The Stash bridge is not running.</strong>
            Port {{ status?.port ?? 8787 }} is most likely in use by another copy of Stash.
            Close the other instance and restart Stash.
          </div>

          <!-- Connected: show what we can see -->
          <template v-else-if="state === 'connected'">
            <div class="bridge-stats">
              <div class="bridge-stat">
                <div class="bridge-stat-n">{{ status?.window_count ?? 0 }}</div>
                <div class="bridge-stat-l">Windows</div>
              </div>
              <div class="bridge-stat">
                <div class="bridge-stat-n">{{ status?.tab_count ?? 0 }}</div>
                <div class="bridge-stat-l">Tabs</div>
              </div>
              <div class="bridge-stat">
                <div class="bridge-stat-n">{{ status?.browser || "—" }}</div>
                <div class="bridge-stat-l">Browser</div>
              </div>
            </div>

            <div class="form-group">
              <span class="form-label">Live Browser Windows</span>
              <div v-if="groupedWindows.length === 0" class="bridge-empty">
                No browser windows open.
              </div>
              <div v-else class="bridge-window-list">
                <div v-for="w in groupedWindows" :key="w.id" class="bridge-window">
                  <div class="bridge-window-head">
                    <span class="bridge-window-count">{{ w.tabCount }} tab{{ w.tabCount !== 1 ? "s" : "" }}</span>
                    <span
                      v-for="g in w.groups"
                      :key="g.id"
                      class="bridge-group-pill"
                      :data-color="g.color"
                    >{{ g.title || "Untitled group" }}</span>
                  </div>
                  <div class="bridge-window-preview">
                    <span v-for="(t, i) in w.preview" :key="i" class="bridge-tab-chip">{{ t }}</span>
                    <span v-if="w.more" class="bridge-tab-more">+{{ w.more }} more</span>
                  </div>
                </div>
              </div>
            </div>

            <p class="bridge-note">
              Snapshots now capture every tab and tab group. Launching a workspace
              rebuilds them, and restored background tabs stay unloaded until you click them.
            </p>
          </template>

          <!-- Bridge running but extension has not checked in -->
          <template v-else>
            <p class="bridge-note" style="margin-top:0;">
              Install the <strong>Stash Bridge</strong> extension, then paste this
              pairing token into its settings page to capture browser tabs.
            </p>

            <div class="form-group">
              <span class="form-label">Pairing Token</span>
              <div class="browse-row">
                <input class="form-input bridge-token" :value="status?.token ?? ''" readonly spellcheck="false" />
                <button class="btn btn-ghost browse-btn" @click="copyToken">
                  {{ copied ? "Copied" : "Copy" }}
                </button>
              </div>
            </div>

            <ol class="bridge-steps">
              <li>Open <code>chrome://extensions</code> and turn on <strong>Developer mode</strong>.</li>
              <li>Click <strong>Load unpacked</strong> and select the <code>extension/</code> folder in the Stash repo.</li>
              <li>Open the extension's <strong>Settings</strong> and paste the token above.</li>
              <li>Press <strong>Save &amp; Connect</strong>. This panel will turn green.</li>
            </ol>
          </template>
        </div>

        <div class="modal-footer">
          <button class="btn btn-ghost" @click="emit('refresh')">Refresh</button>
          <button class="btn btn-primary" style="margin-left:auto;" @click="emit('close')">Done</button>
        </div>
      </div>
    </transition>
  </div>
</template>

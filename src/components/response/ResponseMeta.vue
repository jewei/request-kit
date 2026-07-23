<script setup lang="ts">
import { computed } from 'vue';
import type { HttpResponseData } from '../../types/response';

const props = defineProps<{ response: HttpResponseData }>();

/** 2xx green, 3xx blue, 4xx orange, 5xx red — anything else muted. */
const statusClass = computed(() => {
  const cls = Math.floor(props.response.status / 100);
  return (
    { 2: 'ok', 3: 'redirect', 4: 'client-error', 5: 'server-error' }[cls] ?? 'other'
  );
});

const duration = computed(() => {
  const ms = props.response.durationMs;
  return ms >= 1000 ? `${(ms / 1000).toFixed(1)} s` : `${Math.round(ms)} ms`;
});

const size = computed(() => humanBytes(props.response.bodyBytes));

function humanBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}
</script>

<template>
  <div class="response-meta">
    <span
      class="status-badge"
      :data-status="statusClass"
    >{{ response.status }} {{ response.statusText }}</span>
    <span
      class="meta-item"
      title="response time (application-observed)"
    >{{ duration }}</span>
    <span
      class="meta-item"
      title="decoded body size"
    >{{ size }}</span>
    <span
      v-if="response.downloadCapped"
      class="meta-flag"
      title="download stopped at the size cap"
    >download capped</span>
    <span
      v-if="response.bodyTruncated"
      class="meta-flag"
      title="body truncated for display — save to file for the full body"
    >truncated</span>
    <span
      class="final-url"
      :title="response.finalUrl"
    >{{ response.finalUrl }}</span>
  </div>
</template>

<style scoped>
.response-meta {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 8px 12px;
  border-bottom: 1px solid var(--rk-border);
  font-size: 12px;
}
.status-badge {
  font-weight: 600;
  padding: 2px 8px;
  border-radius: 3px;
  color: #fff;
  white-space: nowrap;
}
.status-badge[data-status='ok'] {
  background: #16a34a;
}
.status-badge[data-status='redirect'] {
  background: #2563eb;
}
.status-badge[data-status='client-error'] {
  background: #ea580c;
}
.status-badge[data-status='server-error'] {
  background: #dc2626;
}
.status-badge[data-status='other'] {
  background: var(--rk-muted);
}
.meta-item {
  color: var(--rk-fg);
  white-space: nowrap;
}
.meta-flag {
  color: #ea580c;
  border: 1px solid currentColor;
  border-radius: 3px;
  padding: 1px 6px;
  white-space: nowrap;
}
.final-url {
  color: var(--rk-muted);
  font-family: ui-monospace, Menlo, Consolas, monospace;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  direction: rtl;
  text-align: left;
  min-width: 0;
  margin-left: auto;
}
</style>

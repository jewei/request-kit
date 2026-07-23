<script setup lang="ts">
import { computed } from 'vue';
import { friendlyMessage, type AppError } from '../../ipc/errors';

const props = defineProps<{ error: AppError }>();

const headline = computed(() => friendlyMessage(props.error));
// A cancelled request is an expected outcome, not a failure — render it muted.
const isCancelled = computed(() => props.error.kind === 'cancelled');
</script>

<template>
  <div
    class="error-display"
    :class="{ 'is-cancelled': isCancelled }"
    role="alert"
  >
    <p class="error-headline">
      {{ headline }}
    </p>
    <details
      v-if="error.detail"
      class="error-detail"
    >
      <summary>Details</summary>
      <pre>{{ error.detail }}</pre>
    </details>
  </div>
</template>

<style scoped>
.error-display {
  padding: 16px 20px;
  border-left: 3px solid #dc2626;
  background: color-mix(in srgb, #dc2626 6%, transparent);
  border-radius: 3px;
  margin: 12px;
}
.error-display.is-cancelled {
  border-left-color: var(--rk-muted);
  background: none;
  color: var(--rk-muted);
}
.error-headline {
  margin: 0;
  font-size: 13px;
  font-weight: 600;
}
.error-detail {
  margin-top: 8px;
  font-size: 12px;
}
.error-detail summary {
  cursor: pointer;
  color: var(--rk-muted);
}
.error-detail pre {
  margin: 6px 0 0;
  white-space: pre-wrap;
  word-break: break-word;
  font-family: ui-monospace, Menlo, Consolas, monospace;
  color: var(--rk-fg);
}
</style>

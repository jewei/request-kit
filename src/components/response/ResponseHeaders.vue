<script setup lang="ts">
import type { HttpResponseData } from '../../types/response';

defineProps<{ response: HttpResponseData }>();
</script>

<template>
  <div class="response-headers">
    <p class="headers-note">
      normalized response headers
    </p>
    <table>
      <tbody>
        <tr
          v-for="([name, value], index) in response.headers"
          :key="`${name}-${index}`"
        >
          <td class="header-name">
            {{ name }}
          </td>
          <td class="header-value">
            {{ value }}
          </td>
        </tr>
      </tbody>
    </table>
    <p
      v-if="response.headers.length === 0"
      class="headers-empty"
    >
      No response headers.
    </p>
  </div>
</template>

<style scoped>
.response-headers {
  overflow: auto;
  padding: 8px 12px;
  font-size: 12px;
}
.headers-note {
  margin: 0 0 8px;
  color: var(--rk-muted);
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.04em;
}
table {
  width: 100%;
  border-collapse: collapse;
  font-family: ui-monospace, Menlo, Consolas, monospace;
}
td {
  padding: 3px 8px 3px 0;
  vertical-align: top;
  border-bottom: 1px solid var(--rk-border);
}
.header-name {
  color: var(--rk-accent);
  white-space: nowrap;
  font-weight: 600;
}
.header-value {
  color: var(--rk-fg);
  word-break: break-all;
  width: 100%;
}
.headers-empty {
  color: var(--rk-muted);
}
</style>

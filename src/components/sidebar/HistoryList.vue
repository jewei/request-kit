<script setup lang="ts">
/** Most-recent-first request history. Shows template URLs only (already redacted
 *  upstream) — never auth headers or resolved secrets. */
import type { HistoryEntry } from '../../types/history';

defineProps<{ entries: HistoryEntry[] }>();
const emit = defineEmits<{ replay: [entry: HistoryEntry]; clear: [] }>();

function statusClass(entry: HistoryEntry): string {
  if (entry.errorKind) return 'error';
  const cls = Math.floor((entry.status ?? 0) / 100);
  return { 2: 'ok', 3: 'redirect', 4: 'client-error', 5: 'server-error' }[cls] ?? 'other';
}

function statusLabel(entry: HistoryEntry): string {
  if (entry.errorKind) return entry.errorKind;
  return entry.status != null ? String(entry.status) : '—';
}

function relativeTime(iso: string): string {
  const then = new Date(iso).getTime();
  if (Number.isNaN(then)) return '';
  const mins = Math.floor((Date.now() - then) / 60_000);
  if (mins < 1) return 'just now';
  if (mins < 60) return `${mins}m ago`;
  const hours = Math.floor(mins / 60);
  if (hours < 24) return `${hours}h ago`;
  return `${Math.floor(hours / 24)}d ago`;
}
</script>

<template>
  <div class="history">
    <header class="history-head">
      <span class="title">History</span>
      <button
        v-if="entries.length"
        class="clear-btn"
        @click="emit('clear')"
      >
        Clear
      </button>
    </header>

    <p
      v-if="!entries.length"
      class="history-empty"
    >
      No requests yet.
    </p>

    <ul
      v-else
      class="history-list"
    >
      <li
        v-for="entry in entries"
        :key="entry.id"
        class="history-row"
        @click="emit('replay', entry)"
      >
        <span
          class="badge"
          :data-status="statusClass(entry)"
        >{{ statusLabel(entry) }}</span>
        <span class="method">{{ entry.method }}</span>
        <span
          class="url"
          :title="entry.templateUrl"
        >{{ entry.templateUrl }}</span>
        <span class="time">{{ relativeTime(entry.executedAt) }}</span>
      </li>
    </ul>
  </div>
</template>

<style scoped>
.history {
  display: flex;
  flex-direction: column;
  min-height: 0;
  height: 100%;
}
.history-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 12px;
  border-bottom: 1px solid var(--rk-border);
}
.title {
  font-size: 12px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  color: var(--rk-muted);
}
.clear-btn {
  border: none;
  background: none;
  color: var(--rk-accent);
  font-size: 12px;
  cursor: pointer;
}
.history-empty {
  padding: 16px 12px;
  font-size: 12px;
  color: var(--rk-muted);
}
.history-list {
  margin: 0;
  padding: 4px;
  list-style: none;
  overflow-y: auto;
}
.history-row {
  display: grid;
  grid-template-columns: auto auto 1fr auto;
  gap: 8px;
  align-items: center;
  padding: 6px 8px;
  font-size: 12px;
  border-radius: 4px;
  cursor: pointer;
}
.history-row:hover {
  background: color-mix(in srgb, var(--rk-fg) 8%, transparent);
}
.badge {
  font-weight: 600;
  padding: 1px 6px;
  border-radius: 3px;
  color: #fff;
  white-space: nowrap;
}
.badge[data-status='ok'] {
  background: #16a34a;
}
.badge[data-status='redirect'] {
  background: #2563eb;
}
.badge[data-status='client-error'] {
  background: #ea580c;
}
.badge[data-status='server-error'],
.badge[data-status='error'] {
  background: #dc2626;
}
.badge[data-status='other'] {
  background: var(--rk-muted);
}
.method {
  color: var(--rk-muted);
  font-family: ui-monospace, Menlo, Consolas, monospace;
}
.url {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-family: ui-monospace, Menlo, Consolas, monospace;
  color: var(--rk-fg);
}
.time {
  color: var(--rk-muted);
  white-space: nowrap;
}
</style>

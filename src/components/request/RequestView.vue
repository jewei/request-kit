<script setup lang="ts">
import { ref } from 'vue';
import { useTabsStore } from '../../stores/tabs';
import type { EditableRow } from './KeyValueEditor.vue';
import BodyEditor from './BodyEditor.vue';
import KeyValueEditor from './KeyValueEditor.vue';
import RequestSettings from './RequestSettings.vue';
import RequestTopBar from './RequestTopBar.vue';
import ResponsePanel from '../response/ResponsePanel.vue';

const store = useTabsStore();

type SubTab = 'params' | 'headers' | 'body' | 'settings';
const activeSubTab = ref<SubTab>('params');
// Auth and Vars sub-tabs arrive in M3a/M4 and are deliberately not rendered.
const SUB_TABS: { id: SubTab; label: string }[] = [
  { id: 'params', label: 'Params' },
  { id: 'headers', label: 'Headers' },
  { id: 'body', label: 'Body' },
  { id: 'settings', label: 'Settings' },
];

type Patch = Partial<Pick<EditableRow, 'key' | 'value' | 'enabled' | 'description'>>;

// --- Params pane: edits QueryParam rows on the canonical URL model. ---
function editParam(id: string, patch: Patch): void {
  const tab = store.activeTab;
  if (!tab) return;
  const row = tab.draft.url.query.find((q) => q.id === id);
  if (!row) return;
  Object.assign(row, patch);
  // Typing a value promotes `?flag` to `?flag=value`.
  if (patch.value !== undefined && patch.value !== '') row.hasEquals = true;
  store.markDirty();
}
function addParam(patch: Patch): void {
  const tab = store.activeTab;
  if (!tab) return;
  tab.draft.url.query.push({
    id: crypto.randomUUID(),
    key: '',
    value: '',
    enabled: true,
    hasEquals: false,
    ...patch,
  });
  store.markDirty();
}
function removeParam(id: string): void {
  const tab = store.activeTab;
  if (!tab) return;
  tab.draft.url.query = tab.draft.url.query.filter((q) => q.id !== id);
  store.markDirty();
}

// --- Headers pane: edits KeyValueRow rows. ---
function editHeader(id: string, patch: Patch): void {
  const tab = store.activeTab;
  if (!tab) return;
  const row = tab.draft.headers.find((h) => h.id === id);
  if (!row) return;
  Object.assign(row, patch);
  store.markDirty();
}
function addHeader(patch: Patch): void {
  const tab = store.activeTab;
  if (!tab) return;
  tab.draft.headers.push({
    id: crypto.randomUUID(),
    key: '',
    value: '',
    enabled: true,
    ...patch,
  });
  store.markDirty();
}
function removeHeader(id: string): void {
  const tab = store.activeTab;
  if (!tab) return;
  tab.draft.headers = tab.draft.headers.filter((h) => h.id !== id);
  store.markDirty();
}
</script>

<template>
  <div
    v-if="store.activeTab"
    class="request-view"
  >
    <div class="request-region">
      <RequestTopBar />

      <div class="sub-tabs">
        <button
          v-for="tab in SUB_TABS"
          :key="tab.id"
          class="sub-tab"
          :class="{ active: activeSubTab === tab.id }"
          @click="activeSubTab = tab.id"
        >
          {{ tab.label }}
        </button>
      </div>

      <div
        v-if="store.prepareErrors.length || store.prepareWarnings.length"
        class="prepare-issues"
      >
        <p
          v-for="issue in store.prepareErrors"
          :key="`e-${issue.code}-${issue.message}`"
          class="issue error"
        >
          {{ issue.message }}
        </p>
        <p
          v-for="issue in store.prepareWarnings"
          :key="`w-${issue.code}-${issue.message}`"
          class="issue warn"
        >
          {{ issue.message }}
        </p>
      </div>

      <div class="pane">
        <KeyValueEditor
          v-if="activeSubTab === 'params'"
          :rows="store.activeTab.draft.url.query"
          key-placeholder="param"
          @edit="editParam"
          @add="addParam"
          @remove="removeParam"
        />
        <KeyValueEditor
          v-else-if="activeSubTab === 'headers'"
          :rows="store.activeTab.draft.headers"
          key-placeholder="header"
          @edit="editHeader"
          @add="addHeader"
          @remove="removeHeader"
        />
        <BodyEditor v-else-if="activeSubTab === 'body'" />
        <RequestSettings v-else-if="activeSubTab === 'settings'" />
      </div>
    </div>

    <div class="response-region">
      <ResponsePanel />
    </div>
  </div>
</template>

<style scoped>
.request-view {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
}
.request-region {
  display: flex;
  flex-direction: column;
  flex: 1 1 55%;
  min-height: 0;
}
.response-region {
  flex: 1 1 40%;
  min-height: 40%;
  display: flex;
  flex-direction: column;
}
.sub-tabs {
  display: flex;
  gap: 2px;
  padding: 6px 12px 0;
  border-bottom: 1px solid var(--rk-border);
}
.sub-tab {
  font-size: 12px;
  padding: 6px 12px;
  border: none;
  border-bottom: 2px solid transparent;
  background: none;
  color: var(--rk-muted);
  cursor: pointer;
}
.sub-tab.active {
  color: var(--rk-fg);
  border-bottom-color: var(--rk-accent);
}
.prepare-issues {
  padding: 8px 12px;
  border-bottom: 1px solid var(--rk-border);
}
.issue {
  margin: 0;
  font-size: 12px;
  padding: 2px 0;
}
.issue.error {
  color: #dc2626;
}
.issue.warn {
  color: #ea580c;
}
.pane {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  overflow: auto;
}
</style>

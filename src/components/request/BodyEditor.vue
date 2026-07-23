<script setup lang="ts">
import { ref } from 'vue';
import { formatJson } from '../../lib/format/json';
import type { DraftBody } from '../../lib/prepare/prepareRequest';
import { useTabsStore } from '../../stores/tabs';
import CodeEditor from '../shared/CodeEditor.vue';

const store = useTabsStore();
const formatError = ref('');

type BodyMode = DraftBody['mode'];

function setMode(event: Event): void {
  const tab = store.activeTab;
  if (!tab) return;
  const mode = (event.target as HTMLSelectElement).value as BodyMode;
  const previous =
    tab.draft.body.mode === 'none' ? '' : (tab.draft.body as { content: string }).content;
  tab.draft.body =
    mode === 'none' ? { mode: 'none' } : { mode, content: previous };
  formatError.value = '';
  store.markDirty();
}

function setContent(content: string): void {
  const tab = store.activeTab;
  if (!tab || tab.draft.body.mode === 'none') return;
  tab.draft.body.content = content;
  store.markDirty();
}

function setRawContentType(event: Event): void {
  const tab = store.activeTab;
  if (!tab || tab.draft.body.mode !== 'raw') return;
  const value = (event.target as HTMLInputElement).value;
  if (value === '') {
    delete tab.draft.body.contentType;
  } else {
    tab.draft.body.contentType = value;
  }
  store.markDirty();
}

function onFormat(): void {
  const tab = store.activeTab;
  if (!tab || tab.draft.body.mode !== 'json') return;
  const result = formatJson(tab.draft.body.content);
  if (result.ok) {
    tab.draft.body.content = result.formatted;
    formatError.value = '';
    store.markDirty();
  } else {
    formatError.value = `Invalid JSON (line ${result.line}, column ${result.column}): ${result.message}`;
  }
}
</script>

<template>
  <div
    v-if="store.activeTab"
    class="body-editor"
  >
    <div class="body-toolbar">
      <select
        class="mode-select"
        :value="store.activeTab.draft.body.mode"
        @change="setMode"
      >
        <option value="none">
          No body
        </option>
        <option value="raw">
          Raw text
        </option>
        <option value="json">
          JSON
        </option>
      </select>
      <input
        v-if="store.activeTab.draft.body.mode === 'raw'"
        class="ct-input"
        type="text"
        placeholder="Content-Type (optional)"
        :value="store.activeTab.draft.body.contentType ?? ''"
        @input="setRawContentType"
      >
      <button
        v-if="store.activeTab.draft.body.mode === 'json'"
        class="format-button"
        @click="onFormat"
      >
        Format
      </button>
    </div>

    <p
      v-if="formatError"
      class="format-error"
    >
      {{ formatError }}
    </p>

    <div
      v-if="store.activeTab.draft.body.mode !== 'none'"
      class="editor-host"
    >
      <CodeEditor
        :model-value="store.activeTab.draft.body.content"
        :language="store.activeTab.draft.body.mode === 'json' ? 'json' : 'text'"
        :placeholder="store.activeTab.draft.body.mode === 'json' ? '{ }' : 'request body'"
        @update:model-value="setContent"
      />
    </div>
    <p
      v-else
      class="no-body"
    >
      This request has no body.
    </p>
  </div>
</template>

<style scoped>
.body-editor {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 8px 12px;
  min-height: 0;
  flex: 1;
}
.body-toolbar {
  display: flex;
  gap: 8px;
  align-items: center;
}
.mode-select,
.ct-input {
  font-size: 12px;
  padding: 5px 8px;
  border: 1px solid var(--rk-border);
  border-radius: 3px;
  background: var(--rk-bg);
  color: var(--rk-fg);
}
.ct-input {
  font-family: ui-monospace, Menlo, Consolas, monospace;
  width: 240px;
}
.format-button {
  font-size: 12px;
  padding: 5px 10px;
  border: 1px solid var(--rk-border);
  border-radius: 3px;
  background: var(--rk-bg);
  color: var(--rk-fg);
  cursor: pointer;
}
.format-error {
  margin: 0;
  font-size: 12px;
  color: #dc2626;
}
.editor-host {
  flex: 1;
  min-height: 120px;
}
.no-body {
  color: var(--rk-muted);
  font-size: 12px;
}
</style>

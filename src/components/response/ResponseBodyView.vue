<script setup lang="ts">
import { writeText } from '@tauri-apps/plugin-clipboard-manager';
import { computed, ref } from 'vue';
import { chooseAndSaveResponse } from '../../ipc/commands';
import { isAppError } from '../../ipc/errors';
import { formatJson } from '../../lib/format/json';
import type { HttpResponseData } from '../../types/response';
import CodeEditor from '../shared/CodeEditor.vue';

const props = defineProps<{
  response: HttpResponseData;
  /** 'pretty' JSON-formats JSON-ish bodies; 'raw' shows the body verbatim. */
  mode: 'pretty' | 'raw';
}>();

const copied = ref(false);
// Set when a retained body has been evicted and can no longer be saved.
const saveError = ref('');

const isJsonish = computed(() => {
  const ct = props.response.contentType?.toLowerCase() ?? '';
  return /\bjson\b/.test(ct) || ct.endsWith('+json');
});

/** The text shown in the editor and the language it is highlighted as. */
const view = computed<{ text: string; language: 'json' | 'text' }>(() => {
  const body = props.response.body ?? '';
  if (props.mode === 'pretty' && isJsonish.value) {
    const result = formatJson(body);
    if (result.ok) return { text: result.formatted, language: 'json' };
  }
  return { text: body, language: 'text' };
});

async function onCopy(): Promise<void> {
  if (props.response.body === null) return;
  await writeText(props.response.body);
  copied.value = true;
  setTimeout(() => (copied.value = false), 1200);
}

async function onSave(): Promise<void> {
  saveError.value = '';
  try {
    await chooseAndSaveResponse(props.response.executionId);
    // false = user cancelled the dialog — silent, nothing to report.
  } catch (error) {
    saveError.value = isAppError(error)
      ? error.message
      : 'The response is no longer available to save. Send the request again.';
  }
}
</script>

<template>
  <div class="response-body">
    <div class="body-toolbar">
      <button
        v-if="response.body !== null"
        class="body-action"
        @click="onCopy"
      >
        {{ copied ? 'Copied' : 'Copy' }}
      </button>
      <button
        v-if="response.isBinary || response.bodyTruncated || response.downloadCapped"
        class="body-action"
        @click="onSave"
      >
        Save to file…
      </button>
    </div>

    <p
      v-if="saveError"
      class="body-error"
    >
      {{ saveError }}
    </p>

    <div
      v-if="response.isBinary"
      class="binary-notice"
    >
      <p>This response is binary (or not valid UTF-8). Save it to inspect the original bytes.</p>
    </div>
    <div
      v-else
      class="editor-host"
    >
      <CodeEditor
        :model-value="view.text"
        :language="view.language"
        readonly
      />
    </div>
  </div>
</template>

<style scoped>
.response-body {
  display: flex;
  flex-direction: column;
  gap: 6px;
  min-height: 0;
  flex: 1;
  padding: 8px 12px;
}
.body-toolbar {
  display: flex;
  gap: 8px;
}
.body-action {
  font-size: 12px;
  padding: 3px 10px;
  border: 1px solid var(--rk-border);
  border-radius: 3px;
  background: var(--rk-bg);
  color: var(--rk-fg);
  cursor: pointer;
}
.body-error {
  margin: 0;
  font-size: 12px;
  color: #dc2626;
}
.binary-notice {
  color: var(--rk-muted);
  font-size: 13px;
  padding: 12px 0;
}
.editor-host {
  flex: 1;
  min-height: 120px;
}
</style>

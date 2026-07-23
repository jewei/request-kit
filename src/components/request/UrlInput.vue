<script setup lang="ts">
import { ref, watch } from 'vue';
import { useTabsStore } from '../../stores/tabs';
import { parseUrlBar, serializeRequestUrl } from '../../lib/url/requestUrl';

const store = useTabsStore();
const text = ref('');
const focused = ref(false);

// The URL bar is a projection of the RequestUrl model: while unfocused it
// mirrors the model (e.g. after row edits in the Params pane); while the user
// types, the bar drives the model through parseUrlBar.
watch(
  () => {
    const tab = store.activeTab;
    return tab ? serializeRequestUrl(tab.draft.url) : '';
  },
  (serialized) => {
    if (!focused.value) text.value = serialized;
  },
  { immediate: true },
);

function onInput(): void {
  const tab = store.activeTab;
  if (!tab) return;
  tab.draft.url = parseUrlBar(text.value, tab.draft.url);
  store.markDirty();
}

function onEnter(): void {
  void store.sendActiveTab();
}
</script>

<template>
  <input
    v-model="text"
    class="url-input"
    type="text"
    spellcheck="false"
    autocomplete="off"
    placeholder="https://api.example.com/path?param=value or {{baseUrl}}/path"
    @input="onInput"
    @focus="focused = true"
    @blur="focused = false"
    @keydown.enter="onEnter"
  >
</template>

<style scoped>
.url-input {
  flex: 1;
  min-width: 0;
  font-family: ui-monospace, Menlo, Consolas, monospace;
  font-size: 13px;
  padding: 6px 10px;
  border: 1px solid var(--rk-border);
  border-radius: 4px;
  background: var(--rk-bg);
  color: var(--rk-fg);
}
.url-input:focus {
  outline: none;
  border-color: var(--rk-accent);
}
</style>

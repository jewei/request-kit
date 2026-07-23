<script setup lang="ts">
import { useTabsStore } from '../../stores/tabs';
import { useSettingsStore } from '../../stores/settings';

const store = useTabsStore();
const settings = useSettingsStore();

function setTimeoutMs(event: Event): void {
  const tab = store.activeTab;
  if (!tab) return;
  const raw = (event.target as HTMLInputElement).value.trim();
  tab.draft.settings.timeoutMs = raw === '' ? null : Math.max(1, Number(raw) || 0);
  store.markDirty();
}

function setRedirects(event: Event): void {
  const tab = store.activeTab;
  if (!tab) return;
  const value = (event.target as HTMLSelectElement).value;
  tab.draft.settings.followRedirects = value === 'inherit' ? null : value === 'on';
  store.markDirty();
}
</script>

<template>
  <div
    v-if="store.activeTab"
    class="request-settings"
  >
    <label class="setting">
      <span>Timeout (ms)</span>
      <input
        type="number"
        min="1"
        :placeholder="`inherit (${settings.settings.timeoutMs})`"
        :value="store.activeTab.draft.settings.timeoutMs ?? ''"
        @input="setTimeoutMs"
      >
    </label>
    <label class="setting">
      <span>Follow redirects</span>
      <select
        :value="
          store.activeTab.draft.settings.followRedirects === null
            ? 'inherit'
            : store.activeTab.draft.settings.followRedirects
              ? 'on'
              : 'off'
        "
        @change="setRedirects"
      >
        <option value="inherit">Inherit ({{ settings.settings.followRedirects ? 'on' : 'off' }})</option>
        <option value="on">On</option>
        <option value="off">Off</option>
      </select>
    </label>
  </div>
</template>

<style scoped>
.request-settings {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 12px;
  max-width: 380px;
}
.setting {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  font-size: 13px;
}
.setting input,
.setting select {
  font-size: 12px;
  padding: 5px 8px;
  border: 1px solid var(--rk-border);
  border-radius: 3px;
  background: var(--rk-bg);
  color: var(--rk-fg);
  width: 180px;
}
</style>

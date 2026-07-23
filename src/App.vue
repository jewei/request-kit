<script setup lang="ts">
import { onMounted } from 'vue';
import { useHotkeys } from './composables/useHotkeys';
import { useHistoryStore } from './stores/history';
import { useSettingsStore } from './stores/settings';
import { useTabsStore } from './stores/tabs';
import { useUiStore } from './stores/ui';
import { useWorkspaceStore } from './stores/workspace';
import MainLayout from './components/layout/MainLayout.vue';
import SettingsModal from './components/settings/SettingsModal.vue';

const tabsStore = useTabsStore();
const workspaceStore = useWorkspaceStore();
const settingsStore = useSettingsStore();
const historyStore = useHistoryStore();
const ui = useUiStore();
const { register } = useHotkeys();

register('mod+enter', () => {
  void tabsStore.sendActiveTab();
});
register('mod+s', () => {
  void tabsStore.save();
});
register('mod+,', () => {
  ui.toggleSettings();
});

onMounted(() => {
  void settingsStore.loadFromDisk().catch((error) => console.error('settings load failed', error));
  void workspaceStore.load().catch((error) => console.error('workspace load failed', error));
  void historyStore.load().catch((error) => console.error('history load failed', error));
});
</script>

<template>
  <MainLayout />
  <SettingsModal
    v-if="ui.settingsOpen"
    @close="ui.settingsOpen = false"
  />
</template>

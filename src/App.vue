<script setup lang="ts">
import { onMounted } from 'vue';
import { useHotkeys } from './composables/useHotkeys';
import { useTabsStore } from './stores/tabs';
import { useWorkspaceStore } from './stores/workspace';
import MainLayout from './components/layout/MainLayout.vue';

const tabsStore = useTabsStore();
const workspaceStore = useWorkspaceStore();
const { register } = useHotkeys();

register('mod+enter', () => {
  void tabsStore.sendActiveTab();
});
register('mod+s', () => {
  void tabsStore.save();
});

onMounted(() => {
  void workspaceStore.load().catch((error) => {
    console.error('failed to load workspace', error);
  });
});
</script>

<template>
  <MainLayout />
</template>

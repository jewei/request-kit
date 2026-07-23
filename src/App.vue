<script setup lang="ts">
import { useHotkeys } from './composables/useHotkeys';
import { useTabsStore } from './stores/tabs';
import MainLayout from './components/layout/MainLayout.vue';

const tabsStore = useTabsStore();
const { register } = useHotkeys();
register('mod+enter', () => {
  void tabsStore.sendActiveTab();
});
</script>

<template>
  <MainLayout />
</template>

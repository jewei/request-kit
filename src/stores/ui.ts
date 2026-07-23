/** Transient UI state not persisted to disk in v0.1 (settings modal visibility). */
import { defineStore } from 'pinia';
import { ref } from 'vue';

export const useUiStore = defineStore('ui', () => {
  const settingsOpen = ref(false);
  function toggleSettings(): void {
    settingsOpen.value = !settingsOpen.value;
  }
  return { settingsOpen, toggleSettings };
});

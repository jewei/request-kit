<script setup lang="ts">
import { useTabsStore } from '../../stores/tabs';
import MethodSelect from './MethodSelect.vue';
import SendButton from './SendButton.vue';
import UrlInput from './UrlInput.vue';

const store = useTabsStore();
</script>

<template>
  <div v-if="store.activeTab" class="top-bar">
    <MethodSelect
      :model-value="store.activeTab.draft.method"
      @update:model-value="
        (method) => {
          store.activeTab!.draft.method = method;
          store.markDirty();
        }
      "
    />
    <UrlInput />
    <SendButton />
  </div>
</template>

<style scoped>
.top-bar {
  display: flex;
  gap: 8px;
  align-items: center;
  padding: 10px 12px;
  border-bottom: 1px solid var(--rk-border);
}
</style>

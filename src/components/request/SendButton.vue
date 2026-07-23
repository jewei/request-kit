<script setup lang="ts">
import { useTabsStore } from '../../stores/tabs';

const store = useTabsStore();

function onClick(): void {
  if (store.isInFlight) {
    store.cancelActiveTab();
  } else {
    void store.sendActiveTab();
  }
}
</script>

<template>
  <button class="send-button" :data-inflight="store.isInFlight" @click="onClick">
    {{ store.isInFlight ? 'Cancel' : 'Send' }}
  </button>
</template>

<style scoped>
.send-button {
  padding: 6px 18px;
  font-size: 13px;
  font-weight: 600;
  border: none;
  border-radius: 4px;
  background: var(--rk-accent);
  color: #fff;
  cursor: pointer;
}
.send-button[data-inflight='true'] {
  background: #dc2626;
}
</style>

<script setup lang="ts">
import { ref } from 'vue';
import { useTabsStore } from '../../stores/tabs';
import ErrorDisplay from './ErrorDisplay.vue';
import ResponseBodyView from './ResponseBodyView.vue';
import ResponseHeaders from './ResponseHeaders.vue';
import ResponseMeta from './ResponseMeta.vue';

const store = useTabsStore();

type View = 'pretty' | 'raw' | 'headers';
const view = ref<View>('pretty');
const VIEWS: { id: View; label: string }[] = [
  { id: 'pretty', label: 'Pretty' },
  { id: 'raw', label: 'Raw' },
  { id: 'headers', label: 'Headers' },
];
</script>

<template>
  <section
    class="response-panel"
    aria-label="response"
  >
    <!-- In-flight takes precedence: a new send has cleared any prior response. -->
    <div
      v-if="store.isInFlight"
      class="response-status inflight"
    >
      <span class="spinner" />
      <span>Sending…</span>
      <button
        class="cancel-button"
        @click="store.cancelActiveTab()"
      >
        Cancel
      </button>
    </div>

    <ErrorDisplay
      v-else-if="store.activeTab?.responseError"
      :error="store.activeTab.responseError"
    />

    <template v-else-if="store.activeTab?.response">
      <ResponseMeta :response="store.activeTab.response" />
      <div class="view-tabs">
        <button
          v-for="tab in VIEWS"
          :key="tab.id"
          class="view-tab"
          :class="{ active: view === tab.id }"
          @click="view = tab.id"
        >
          {{ tab.label }}
        </button>
      </div>
      <ResponseHeaders
        v-if="view === 'headers'"
        :response="store.activeTab.response"
      />
      <ResponseBodyView
        v-else
        :key="view"
        :response="store.activeTab.response"
        :mode="view"
      />
    </template>

    <div
      v-else
      class="response-status empty"
    >
      Send a request to see the response here.
    </div>
  </section>
</template>

<style scoped>
.response-panel {
  display: flex;
  flex-direction: column;
  min-height: 0;
  height: 100%;
  border-top: 1px solid var(--rk-border);
  background: var(--rk-bg);
}
.response-status {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 24px;
  color: var(--rk-muted);
  font-size: 13px;
}
.response-status.empty {
  justify-content: center;
}
.cancel-button {
  padding: 4px 14px;
  font-size: 12px;
  border: 1px solid #dc2626;
  border-radius: 4px;
  background: none;
  color: #dc2626;
  cursor: pointer;
}
.spinner {
  width: 14px;
  height: 14px;
  border: 2px solid var(--rk-border);
  border-top-color: var(--rk-accent);
  border-radius: 50%;
  animation: spin 0.7s linear infinite;
}
@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}
.view-tabs {
  display: flex;
  gap: 2px;
  padding: 6px 12px 0;
  border-bottom: 1px solid var(--rk-border);
}
.view-tab {
  font-size: 12px;
  padding: 5px 12px;
  border: none;
  border-bottom: 2px solid transparent;
  background: none;
  color: var(--rk-muted);
  cursor: pointer;
}
.view-tab.active {
  color: var(--rk-fg);
  border-bottom-color: var(--rk-accent);
}
</style>

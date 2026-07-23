/**
 * Tab / draft / send / response state machine (PLAN.md "Frontend architecture").
 * M1 is single-tab, but the model is already the plan's Tab shape so M3a
 * multi-tab work only adds UI + persistence.
 */
import { defineStore } from 'pinia';
import { computed, ref, toRaw } from 'vue';
import { cancelRequest, releaseResponse, sendRequest } from '../ipc/commands';
import { isAppError, type AppError } from '../ipc/errors';
import type { KeyValueRow } from '../types/request';
import type { HttpResponseData } from '../types/response';
import { prepareRequest, type RequestDraft } from '../lib/prepare/prepareRequest';
import { emptyRequestUrl } from '../lib/url/requestUrl';

export interface PrepareIssue {
  code: string;
  message: string;
}

export interface Tab {
  tabId: string;
  /** null = scratch tab (not yet saved). */
  requestId: string | null;
  draft: RequestDraft;
  pinned: boolean;
  /** Explicit flag set on first edit — no deep diffing. */
  dirty: boolean;
  response: HttpResponseData | null;
  responseError: AppError | null;
  inFlightExecutionId: string | null;
}

/** App-level defaults; replaced by the settings store in M2b. */
export const APP_DEFAULTS = {
  timeoutMs: 30_000,
  followRedirects: true,
  maxBodyBytes: 10 * 1024 * 1024,
} as const;

export function blankHeaderRow(): KeyValueRow {
  return { id: crypto.randomUUID(), key: '', value: '', enabled: true, description: '' };
}

function createScratchTab(): Tab {
  return {
    tabId: crypto.randomUUID(),
    requestId: null,
    draft: {
      method: 'GET',
      url: emptyRequestUrl(),
      headers: [],
      body: { mode: 'none' },
      settings: { timeoutMs: null, followRedirects: null },
    },
    pinned: false,
    dirty: false,
    response: null,
    responseError: null,
    inFlightExecutionId: null,
  };
}

export const useTabsStore = defineStore('tabs', () => {
  const initialTab = createScratchTab();
  const tabs = ref<Tab[]>([initialTab]);
  const activeTabId = ref(initialTab.tabId);

  /** Validation errors from the last prepare attempt (shown above the editor). */
  const prepareErrors = ref<PrepareIssue[]>([]);
  const prepareWarnings = ref<PrepareIssue[]>([]);

  const activeTab = computed<Tab | null>(
    () => tabs.value.find((tab) => tab.tabId === activeTabId.value) ?? null,
  );
  const isInFlight = computed(() => activeTab.value?.inFlightExecutionId != null);

  /** Set on first edit; idempotent. */
  function markDirty(): void {
    const tab = activeTab.value;
    if (tab) tab.dirty = true;
  }

  /** M2 stub — real persistence arrives with the storage milestone. */
  function save(): void {
    const tab = activeTab.value;
    if (tab) tab.dirty = false;
  }

  async function sendActiveTab(): Promise<void> {
    const tab = activeTab.value;
    if (!tab) return;

    // Hand the pipeline the raw draft: prepareRequest deep-clones internally,
    // and cloning through Vue's reactive proxy is both wasteful and unsupported
    // by some structuredClone implementations.
    const result = prepareRequest(
      toRaw(tab.draft),
      {
        variableSources: { environment: [], globals: [] },
        appDefaults: APP_DEFAULTS,
      },
      { variableMode: 'resolve', sensitiveValueMode: 'include', unresolvedMode: 'error' },
    );
    prepareWarnings.value = result.warnings;
    if (!result.ok) {
      prepareErrors.value = result.errors;
      return;
    }
    prepareErrors.value = [];

    const executionId = crypto.randomUUID();
    // A newer send supersedes any in-flight execution; its completion will be
    // dropped by the stale-execution guard below. Cancel it to free resources.
    if (tab.inFlightExecutionId) {
      void cancelRequest(tab.inFlightExecutionId).catch(() => {});
    }
    // Release the previously displayed response's retained bytes.
    if (tab.response) {
      void releaseResponse(tab.response.executionId).catch(() => {});
    }
    tab.response = null;
    tab.responseError = null;
    tab.inFlightExecutionId = executionId;

    try {
      const response = await sendRequest({ ...result.request, executionId, tabId: tab.tabId });
      if (tab.inFlightExecutionId !== executionId) {
        // Stale completion: never touches the UI, and its retained bytes are freed.
        void releaseResponse(executionId).catch(() => {});
        return;
      }
      tab.response = response;
      tab.responseError = null;
      tab.inFlightExecutionId = null;
    } catch (error) {
      if (tab.inFlightExecutionId !== executionId) {
        void releaseResponse(executionId).catch(() => {});
        return;
      }
      tab.responseError = isAppError(error)
        ? error
        : { kind: 'unknown', message: error instanceof Error ? error.message : String(error) };
      tab.inFlightExecutionId = null;
    }
  }

  /** Fire-and-forget; the pending send promise settles with kind 'cancelled'. */
  function cancelActiveTab(): void {
    const executionId = activeTab.value?.inFlightExecutionId;
    if (executionId) {
      void cancelRequest(executionId).catch(() => {});
    }
  }

  return {
    tabs,
    activeTabId,
    prepareErrors,
    prepareWarnings,
    activeTab,
    isInFlight,
    markDirty,
    save,
    sendActiveTab,
    cancelActiveTab,
  };
});

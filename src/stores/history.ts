/** Request history store. Newest entries are kept at the front. Recording is
 *  optimistic: the entry is prepended immediately and persisted in the
 *  background (a failed append never blocks a send). */
import { defineStore } from 'pinia';
import { ref } from 'vue';
import { appendHistory, clearHistory, readHistory } from '../ipc/commands';
import type { HistoryEntry } from '../types/history';

export const useHistoryStore = defineStore('history', () => {
  const entries = ref<HistoryEntry[]>([]);

  async function load(limit = 200): Promise<void> {
    entries.value = await readHistory(limit);
  }

  function record(entry: HistoryEntry): void {
    entries.value.unshift(entry);
    void appendHistory(entry).catch((error) => {
      console.error('failed to append history', error);
    });
  }

  async function clear(): Promise<void> {
    await clearHistory();
    entries.value = [];
  }

  return { entries, load, record, clear };
});

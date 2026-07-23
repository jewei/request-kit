import { createPinia, setActivePinia } from 'pinia';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { HistoryEntry } from '../types/history';
import { useHistoryStore } from './history';
import * as commands from '../ipc/commands';

vi.mock('../ipc/commands', () => ({
  readHistory: vi.fn(),
  appendHistory: vi.fn().mockResolvedValue(undefined),
  clearHistory: vi.fn().mockResolvedValue(undefined),
}));

function entry(id: string): HistoryEntry {
  return {
    version: 1,
    id,
    executedAt: '2026-07-24T00:00:00.000Z',
    method: 'GET',
    templateUrl: 'https://x/',
    status: 200,
    durationMs: 10,
    bodyBytes: 2,
    requestId: null,
    errorKind: null,
  };
}

describe('history store', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.clearAllMocks();
    vi.mocked(commands.appendHistory).mockResolvedValue(undefined);
    vi.mocked(commands.clearHistory).mockResolvedValue(undefined);
  });

  it('load populates entries from the backend', async () => {
    vi.mocked(commands.readHistory).mockResolvedValue([entry('a'), entry('b')]);
    const store = useHistoryStore();
    await store.load();
    expect(store.entries).toHaveLength(2);
  });

  it('record prepends and appends to the backend', () => {
    const store = useHistoryStore();
    store.record(entry('new'));
    expect(store.entries[0].id).toBe('new');
    expect(commands.appendHistory).toHaveBeenCalledWith(expect.objectContaining({ id: 'new' }));
  });

  it('clear empties and calls the backend', async () => {
    vi.mocked(commands.readHistory).mockResolvedValue([entry('a')]);
    const store = useHistoryStore();
    await store.load();
    await store.clear();
    expect(store.entries).toEqual([]);
    expect(commands.clearHistory).toHaveBeenCalledOnce();
  });
});

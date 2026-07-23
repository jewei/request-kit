import { createPinia, setActivePinia } from 'pinia';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { HttpResponseData } from '../types/response';
import { useTabsStore } from './tabs';
import * as commands from '../ipc/commands';

vi.mock('../ipc/commands', () => ({
  sendRequest: vi.fn(),
  cancelRequest: vi.fn().mockResolvedValue(undefined),
  releaseResponse: vi.fn().mockResolvedValue(undefined),
  chooseAndSaveResponse: vi.fn(),
}));

interface Deferred<T> {
  promise: Promise<T>;
  resolve: (value: T) => void;
  reject: (error: unknown) => void;
}
function deferred<T>(): Deferred<T> {
  let resolve!: (value: T) => void;
  let reject!: (error: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

function makeResponse(executionId: string): HttpResponseData {
  return {
    executionId,
    status: 200,
    statusText: 'OK',
    httpVersion: 'HTTP/1.1',
    headers: [],
    durationMs: 12,
    bodyBytes: 2,
    contentType: 'application/json',
    finalUrl: 'https://example.com',
    body: '{}',
    bodyTruncated: false,
    isBinary: false,
    downloadCapped: false,
  };
}

function setValidUrl(store: ReturnType<typeof useTabsStore>): void {
  store.activeTab!.draft.url.base = 'https://example.com';
}

describe('tabs store send/cancel state machine', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.clearAllMocks();
    vi.mocked(commands.cancelRequest).mockResolvedValue(undefined);
    vi.mocked(commands.releaseResponse).mockResolvedValue(undefined);
  });

  it('flips isInFlight true while sending and back to false on completion', async () => {
    const d = deferred<HttpResponseData>();
    vi.mocked(commands.sendRequest).mockReturnValueOnce(d.promise);

    const store = useTabsStore();
    setValidUrl(store);
    expect(store.isInFlight).toBe(false);

    const pending = store.sendActiveTab();
    expect(store.isInFlight).toBe(true);

    d.resolve(makeResponse('exec-1'));
    await pending;
    expect(store.isInFlight).toBe(false);
    expect(store.activeTab!.response).not.toBeNull();
  });

  it('drops a stale completion and releases its retained bytes', async () => {
    const calls: { executionId: string; d: Deferred<HttpResponseData> }[] = [];
    vi.mocked(commands.sendRequest).mockImplementation((payload) => {
      const d = deferred<HttpResponseData>();
      calls.push({ executionId: payload.executionId, d });
      return d.promise;
    });

    const store = useTabsStore();
    setValidUrl(store);

    // First send, then a newer send supersedes it before the first resolves.
    const first = store.sendActiveTab();
    const second = store.sendActiveTab();
    expect(calls).toHaveLength(2);
    const [older, newer] = calls;

    // Superseding send cancels the older in-flight execution.
    expect(commands.cancelRequest).toHaveBeenCalledWith(older.executionId);

    // The OLDER request finishes last — its completion must be discarded.
    older.d.resolve(makeResponse(older.executionId));
    await first;
    expect(commands.releaseResponse).toHaveBeenCalledWith(older.executionId);
    expect(store.activeTab!.response).toBeNull();

    // The newer request's completion is the one that updates the UI.
    const newerResponse = makeResponse(newer.executionId);
    newer.d.resolve(newerResponse);
    await second;
    expect(store.activeTab!.response).toStrictEqual(newerResponse);
    expect(store.isInFlight).toBe(false);
  });
});

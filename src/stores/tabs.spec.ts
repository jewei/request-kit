import { createPinia, setActivePinia } from 'pinia';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { HttpResponseData } from '../types/response';
import type { RequestFile, WorkspaceBootstrap, WorkspaceNode } from '../types/workspace';
import { useTabsStore } from './tabs';
import { useWorkspaceStore } from './workspace';
import { useSettingsStore } from './settings';
import { useHistoryStore } from './history';
import type { QueryParam } from '../types/request';
import * as commands from '../ipc/commands';

vi.mock('../ipc/commands', () => ({
  sendRequest: vi.fn(),
  cancelRequest: vi.fn().mockResolvedValue(undefined),
  releaseResponse: vi.fn().mockResolvedValue(undefined),
  chooseAndSaveResponse: vi.fn(),
  writeRequest: vi.fn().mockResolvedValue(undefined),
  createCollection: vi.fn(),
  createRequest: vi.fn(),
  createFolder: vi.fn(),
  renameNode: vi.fn(),
  deleteNode: vi.fn(),
  duplicateRequest: vi.fn(),
  loadWorkspace: vi.fn(),
  appendHistory: vi.fn().mockResolvedValue(undefined),
  readHistory: vi.fn().mockResolvedValue([]),
  clearHistory: vi.fn().mockResolvedValue(undefined),
  readSettings: vi.fn(),
  writeSettings: vi.fn().mockResolvedValue(undefined),
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

function requestFile(id: string, name: string): RequestFile {
  return {
    version: 1,
    id,
    name,
    method: 'POST',
    url: { base: 'https://example.com', query: [], fragment: '' },
    headers: [],
    body: { mode: 'none' },
    auth: { type: 'inherit' },
    variables: [],
    settings: { timeoutMs: null, followRedirects: null },
  };
}

function bootstrap(tree: WorkspaceNode[]): WorkspaceBootstrap {
  return { tree, environments: [], globals: [], settings: {}, uiState: {}, quarantined: [] };
}

describe('tabs store save / open / delete', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.clearAllMocks();
    vi.mocked(commands.releaseResponse).mockResolvedValue(undefined);
    vi.mocked(commands.writeRequest).mockResolvedValue(undefined);
  });

  it('openRequest loads the document into the active tab and clears dirty', () => {
    const store = useTabsStore();
    store.activeTab!.dirty = true;
    store.openRequest(requestFile('r1', 'Ping'));

    expect(store.activeTab!.requestId).toBe('r1');
    expect(store.activeTab!.name).toBe('Ping');
    expect(store.activeTab!.draft.method).toBe('POST');
    expect(store.activeTab!.draft.url.base).toBe('https://example.com');
    expect(store.activeTab!.dirty).toBe(false);
  });

  it('saving a scratch tab creates the request under a collection then writes it', async () => {
    const collection: WorkspaceNode = { id: 'c1', kind: 'collection', name: 'API', children: [] };
    vi.mocked(commands.loadWorkspace).mockResolvedValue(bootstrap([collection]));
    vi.mocked(commands.createRequest).mockResolvedValue({
      id: 'r-new',
      kind: 'request',
      name: 'Untitled Request',
    });

    // Populate the workspace store with one collection.
    const workspace = useWorkspaceStore();
    await workspace.load();

    const store = useTabsStore();
    store.markDirty();
    await store.save();

    expect(commands.createRequest).toHaveBeenCalledWith('c1', 'Untitled');
    expect(commands.writeRequest).toHaveBeenCalledOnce();
    expect(store.activeTab!.requestId).toBe('r-new');
    expect(store.activeTab!.dirty).toBe(false);
  });

  it('saving an already-saved tab writes without creating a node', async () => {
    const store = useTabsStore();
    store.openRequest(requestFile('r1', 'Ping'));
    store.markDirty();
    await store.save();

    expect(commands.createRequest).not.toHaveBeenCalled();
    expect(commands.writeRequest).toHaveBeenCalledWith('r1', expect.objectContaining({ id: 'r1' }));
    expect(store.activeTab!.dirty).toBe(false);
  });

  it('onNodeDeleted converts an open request into a scratch tab and releases its response', () => {
    const store = useTabsStore();
    store.openRequest(requestFile('r1', 'Ping'));
    store.activeTab!.response = makeResponse('exec-1');

    store.onNodeDeleted('r1');

    expect(store.activeTab!.requestId).toBeNull();
    expect(store.activeTab!.response).toBeNull();
    expect(commands.releaseResponse).toHaveBeenCalledWith('exec-1');
  });
});

function secretRow(): QueryParam {
  return { id: 'q1', key: 'token', value: 'secret', enabled: true, hasEquals: true };
}

describe('tabs store history + settings', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.clearAllMocks();
    vi.mocked(commands.releaseResponse).mockResolvedValue(undefined);
    vi.mocked(commands.appendHistory).mockResolvedValue(undefined);
  });

  it('records one redacted history entry on a successful send', async () => {
    const d = deferred<HttpResponseData>();
    vi.mocked(commands.sendRequest).mockReturnValueOnce(d.promise);

    const history = useHistoryStore();
    const store = useTabsStore();
    setValidUrl(store);
    store.activeTab!.draft.url.query.push(secretRow());

    const pending = store.sendActiveTab();
    d.resolve(makeResponse('e1'));
    await pending;

    expect(history.entries).toHaveLength(1);
    expect(history.entries[0].templateUrl).toContain('token=<redacted>');
    expect(history.entries[0].status).toBe(200);
    expect(commands.appendHistory).toHaveBeenCalledOnce();
  });

  it('records an error entry but nothing on cancel', async () => {
    const store = useTabsStore();
    setValidUrl(store);
    const history = useHistoryStore();

    const d1 = deferred<HttpResponseData>();
    vi.mocked(commands.sendRequest).mockReturnValueOnce(d1.promise);
    const p1 = store.sendActiveTab();
    d1.reject({ kind: 'dns', message: 'no host' });
    await p1;
    expect(history.entries).toHaveLength(1);
    expect(history.entries[0].errorKind).toBe('dns');

    const d2 = deferred<HttpResponseData>();
    vi.mocked(commands.sendRequest).mockReturnValueOnce(d2.promise);
    const p2 = store.sendActiveTab();
    d2.reject({ kind: 'cancelled', message: 'cancelled' });
    await p2;
    expect(history.entries).toHaveLength(1); // cancel recorded nothing
  });

  it('flows the settings timeout into the send payload', async () => {
    const settings = useSettingsStore();
    settings.load({ timeoutMs: 1234 });

    const d = deferred<HttpResponseData>();
    vi.mocked(commands.sendRequest).mockReturnValueOnce(d.promise);
    const store = useTabsStore();
    setValidUrl(store);

    const pending = store.sendActiveTab();
    d.resolve(makeResponse('e1'));
    await pending;

    expect(commands.sendRequest).toHaveBeenCalledWith(expect.objectContaining({ timeoutMs: 1234 }));
  });
});

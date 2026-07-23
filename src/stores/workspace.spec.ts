import { createPinia, setActivePinia } from 'pinia';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { WorkspaceBootstrap, WorkspaceNode } from '../types/workspace';
import { useWorkspaceStore } from './workspace';
import * as commands from '../ipc/commands';

vi.mock('../ipc/commands', () => ({
  loadWorkspace: vi.fn(),
  createCollection: vi.fn(),
  createFolder: vi.fn(),
  createRequest: vi.fn(),
  renameNode: vi.fn(),
  deleteNode: vi.fn(),
  duplicateRequest: vi.fn(),
}));

function bootstrap(tree: WorkspaceNode[]): WorkspaceBootstrap {
  return { tree, environments: [], globals: [], settings: {}, uiState: {}, quarantined: [] };
}

const collection: WorkspaceNode = {
  id: 'c1',
  kind: 'collection',
  name: 'API',
  children: [{ id: 'r1', kind: 'request', name: 'Ping' }],
};

describe('workspace store', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.clearAllMocks();
  });

  it('load() populates the tree and quarantined list', async () => {
    vi.mocked(commands.loadWorkspace).mockResolvedValue(bootstrap([collection]));
    const store = useWorkspaceStore();
    await store.load();
    expect(store.tree).toEqual([collection]);
  });

  it('nodeById finds nested nodes', async () => {
    vi.mocked(commands.loadWorkspace).mockResolvedValue(bootstrap([collection]));
    const store = useWorkspaceStore();
    await store.load();
    expect(store.nodeById('r1')?.name).toBe('Ping');
    expect(store.nodeById('missing')).toBeNull();
  });

  it('createCollection creates then reloads the tree from disk', async () => {
    const newCol: WorkspaceNode = { id: 'c2', kind: 'collection', name: 'New', children: [] };
    vi.mocked(commands.createCollection).mockResolvedValue(newCol);
    // The reload reflects the newly-created collection.
    vi.mocked(commands.loadWorkspace).mockResolvedValue(bootstrap([collection, newCol]));

    const store = useWorkspaceStore();
    const returned = await store.createCollection('New');

    expect(commands.createCollection).toHaveBeenCalledWith('New');
    expect(commands.loadWorkspace).toHaveBeenCalledOnce(); // resynced from disk
    expect(returned).toEqual(newCol);
    expect(store.nodeById('c2')).not.toBeNull();
  });

  it('remove deletes then reloads', async () => {
    vi.mocked(commands.deleteNode).mockResolvedValue(undefined);
    vi.mocked(commands.loadWorkspace).mockResolvedValue(bootstrap([]));

    const store = useWorkspaceStore();
    await store.remove('c1');

    expect(commands.deleteNode).toHaveBeenCalledWith('c1');
    expect(store.tree).toEqual([]);
  });
});

describe('workspace filteredTree', () => {
  const nested: WorkspaceNode = {
    id: 'c1',
    kind: 'collection',
    name: 'API',
    children: [
      {
        id: 'f1',
        kind: 'folder',
        name: 'Auth',
        children: [
          { id: 'r1', kind: 'request', name: 'Login' },
          { id: 'r2', kind: 'request', name: 'Logout' },
        ],
      },
      { id: 'r3', kind: 'request', name: 'Health' },
    ],
  };

  beforeEach(() => {
    setActivePinia(createPinia());
    vi.clearAllMocks();
    vi.mocked(commands.loadWorkspace).mockResolvedValue(bootstrap([nested]));
  });

  it('empty query returns the full tree', async () => {
    const store = useWorkspaceStore();
    await store.load();
    expect(store.filteredTree('')).toBe(store.tree);
  });

  it('keeps only the path to a matching request', async () => {
    const store = useWorkspaceStore();
    await store.load();
    const result = store.filteredTree('login');
    expect(result).toHaveLength(1);
    expect(result[0].id).toBe('c1');
    const folder = result[0].children!;
    expect(folder).toHaveLength(1); // "Health" excluded
    expect(folder[0].id).toBe('f1');
    expect(folder[0].children).toHaveLength(1); // only "Login"
    expect(folder[0].children![0].id).toBe('r1');
  });

  it('a matching container keeps its whole subtree', async () => {
    const store = useWorkspaceStore();
    await store.load();
    const result = store.filteredTree('auth');
    expect(result[0].children![0].children).toHaveLength(2); // Login + Logout
  });

  it('no match returns an empty list', async () => {
    const store = useWorkspaceStore();
    await store.load();
    expect(store.filteredTree('zzz')).toEqual([]);
  });
});

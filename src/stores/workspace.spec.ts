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

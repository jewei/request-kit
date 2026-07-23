/**
 * Workspace tree store. Disk (via the Rust storage commands) is the source of
 * truth, so every mutation re-loads the whole tree rather than surgically
 * patching it — simpler and always consistent with what the backend scanned.
 */
import { defineStore } from 'pinia';
import { ref } from 'vue';
import {
  createCollection as ipcCreateCollection,
  createFolder as ipcCreateFolder,
  createRequest as ipcCreateRequest,
  deleteNode as ipcDeleteNode,
  duplicateRequest as ipcDuplicateRequest,
  loadWorkspace,
  renameNode as ipcRenameNode,
} from '../ipc/commands';
import type { QuarantineReport, WorkspaceNode } from '../types/workspace';

function findNode(nodes: WorkspaceNode[], id: string): WorkspaceNode | null {
  for (const node of nodes) {
    if (node.id === id) return node;
    if (node.children) {
      const found = findNode(node.children, id);
      if (found) return found;
    }
  }
  return null;
}

/** Keep requests whose name matches; a matching container keeps its whole
 *  subtree, and a non-matching container is kept only if a descendant matches. */
function filterNodes(nodes: WorkspaceNode[], query: string): WorkspaceNode[] {
  const out: WorkspaceNode[] = [];
  for (const node of nodes) {
    const selfMatch = node.name.toLowerCase().includes(query);
    if (node.kind === 'request') {
      if (selfMatch) out.push(node);
    } else if (selfMatch) {
      out.push(node);
    } else {
      const children = filterNodes(node.children ?? [], query);
      if (children.length) out.push({ ...node, children });
    }
  }
  return out;
}

export const useWorkspaceStore = defineStore('workspace', () => {
  const tree = ref<WorkspaceNode[]>([]);
  const quarantined = ref<QuarantineReport[]>([]);

  async function load(): Promise<void> {
    const boot = await loadWorkspace();
    tree.value = boot.tree;
    quarantined.value = boot.quarantined;
  }

  function nodeById(id: string): WorkspaceNode | null {
    return findNode(tree.value, id);
  }

  /** The tree filtered by a name query (case-insensitive); empty query = full tree. */
  function filteredTree(query: string): WorkspaceNode[] {
    const q = query.trim().toLowerCase();
    if (!q) return tree.value;
    return filterNodes(tree.value, q);
  }

  async function createCollection(name: string): Promise<WorkspaceNode> {
    const node = await ipcCreateCollection(name);
    await load();
    return node;
  }

  async function createFolder(parentId: string, name: string): Promise<WorkspaceNode> {
    const node = await ipcCreateFolder(parentId, name);
    await load();
    return node;
  }

  async function createRequest(parentId: string, name: string): Promise<WorkspaceNode> {
    const node = await ipcCreateRequest(parentId, name);
    await load();
    return node;
  }

  async function rename(id: string, newName: string): Promise<void> {
    await ipcRenameNode(id, newName);
    await load();
  }

  async function remove(id: string): Promise<void> {
    await ipcDeleteNode(id);
    await load();
  }

  async function duplicate(id: string): Promise<WorkspaceNode> {
    const node = await ipcDuplicateRequest(id);
    await load();
    return node;
  }

  return {
    tree,
    quarantined,
    load,
    nodeById,
    filteredTree,
    createCollection,
    createFolder,
    createRequest,
    rename,
    remove,
    duplicate,
  };
});

<script setup lang="ts">
/** Collections sidebar: renders the tree and drives create/open/rename/delete
 *  through the workspace + tabs stores. Menus/dialogs live here so the tree
 *  components stay presentational. */
import { computed, ref } from 'vue';
import { readRequest } from '../../ipc/commands';
import { useTabsStore } from '../../stores/tabs';
import { useWorkspaceStore } from '../../stores/workspace';
import CollectionTree from '../sidebar/CollectionTree.vue';
import ConfirmDialog from '../shared/ConfirmDialog.vue';
import ContextMenu, { type MenuItem } from '../shared/ContextMenu.vue';

const workspace = useWorkspaceStore();
const tabs = useTabsStore();

const menu = ref<{ id: string; x: number; y: number } | null>(null);
const renamingId = ref<string | null>(null);
const confirm = ref<{
  title: string;
  message: string;
  onConfirm: () => void | Promise<void>;
} | null>(null);
const quarantineDismissed = ref(false);

const activeRequestId = computed(() => tabs.activeTab?.requestId ?? null);
const activeDirty = computed(() => tabs.activeTab?.dirty ?? false);

const menuItems = computed<MenuItem[]>(() => {
  if (!menu.value) return [];
  const node = workspace.nodeById(menu.value.id);
  if (!node) return [];
  if (node.kind === 'request') {
    return [
      { label: 'Open', value: 'open' },
      { label: 'Rename', value: 'rename' },
      { label: 'Duplicate', value: 'duplicate' },
      { label: 'Delete', value: 'delete', danger: true },
    ];
  }
  return [
    { label: 'New request', value: 'new-request' },
    { label: 'New folder', value: 'new-folder' },
    { label: 'Rename', value: 'rename' },
    { label: 'Delete', value: 'delete', danger: true },
  ];
});

function report(error: unknown): void {
  console.error('sidebar action failed', error);
}

async function newCollection(): Promise<void> {
  try {
    const node = await workspace.createCollection('New collection');
    renamingId.value = node.id;
  } catch (error) {
    report(error);
  }
}

async function openNode(id: string): Promise<void> {
  try {
    const file = await readRequest(id);
    tabs.openRequest(file);
  } catch (error) {
    report(error);
  }
}

function requestOpen(id: string): void {
  if (activeDirty.value) {
    confirm.value = {
      title: 'Discard unsaved changes?',
      message: 'The current request has unsaved edits that will be lost.',
      onConfirm: () => openNode(id),
    };
  } else {
    void openNode(id);
  }
}

async function onMenuSelect(value: string): Promise<void> {
  const target = menu.value?.id;
  menu.value = null;
  if (!target) return;
  try {
    switch (value) {
      case 'open':
        requestOpen(target);
        break;
      case 'new-request': {
        const node = await workspace.createRequest(target, 'New request');
        await openNode(node.id);
        renamingId.value = node.id;
        break;
      }
      case 'new-folder': {
        const node = await workspace.createFolder(target, 'New folder');
        renamingId.value = node.id;
        break;
      }
      case 'rename':
        renamingId.value = target;
        break;
      case 'duplicate':
        await workspace.duplicate(target);
        break;
      case 'delete':
        askDelete(target);
        break;
    }
  } catch (error) {
    report(error);
  }
}

function askDelete(id: string): void {
  const node = workspace.nodeById(id);
  if (!node) return;
  const suffix = node.kind === 'request' ? '' : ' and everything inside it';
  confirm.value = {
    title: `Delete "${node.name}"?`,
    message: `This permanently deletes "${node.name}"${suffix}.`,
    onConfirm: async () => {
      await workspace.remove(id);
      tabs.onNodeDeleted(id);
    },
  };
}

async function onRenameCommit(payload: { id: string; name: string }): Promise<void> {
  renamingId.value = null;
  try {
    await workspace.rename(payload.id, payload.name);
    if (tabs.activeTab?.requestId === payload.id) {
      tabs.activeTab.name = payload.name;
    }
  } catch (error) {
    report(error);
  }
}

async function runConfirm(): Promise<void> {
  const action = confirm.value?.onConfirm;
  confirm.value = null;
  if (action) {
    try {
      await action();
    } catch (error) {
      report(error);
    }
  }
}
</script>

<template>
  <aside class="sidebar">
    <header class="sidebar-head">
      <span class="title">Collections</span>
      <button
        class="new-btn"
        title="New collection"
        @click="newCollection"
      >
        +
      </button>
    </header>

    <div
      v-if="workspace.quarantined.length && !quarantineDismissed"
      class="quarantine-notice"
    >
      <span>{{ workspace.quarantined.length }} file(s) could not be loaded and were set
        aside.</span>
      <button @click="quarantineDismissed = true">
        Dismiss
      </button>
    </div>

    <div class="tree-scroll">
      <CollectionTree
        :nodes="workspace.tree"
        :renaming-id="renamingId"
        :active-request-id="activeRequestId"
        :dirty="activeDirty"
        @open="requestOpen"
        @menu="(payload) => (menu = payload)"
        @rename-commit="onRenameCommit"
        @rename-cancel="renamingId = null"
      />
    </div>

    <ContextMenu
      v-if="menu"
      :items="menuItems"
      :x="menu.x"
      :y="menu.y"
      @select="onMenuSelect"
      @close="menu = null"
    />

    <ConfirmDialog
      v-if="confirm"
      :title="confirm.title"
      :message="confirm.message"
      confirm-label="Delete"
      danger
      @confirm="runConfirm"
      @cancel="confirm = null"
    />
  </aside>
</template>

<style scoped>
.sidebar {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
  border-right: 1px solid var(--rk-border);
  background: color-mix(in srgb, var(--rk-fg) 3%, var(--rk-bg));
}
.sidebar-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 12px;
  border-bottom: 1px solid var(--rk-border);
}
.title {
  font-size: 12px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  color: var(--rk-muted);
}
.new-btn {
  width: 22px;
  height: 22px;
  font-size: 16px;
  line-height: 1;
  border: 1px solid var(--rk-border);
  border-radius: 4px;
  background: var(--rk-bg);
  color: var(--rk-fg);
  cursor: pointer;
}
.quarantine-notice {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  font-size: 12px;
  color: #ea580c;
  background: color-mix(in srgb, #ea580c 8%, transparent);
}
.quarantine-notice button {
  margin-left: auto;
  border: none;
  background: none;
  color: var(--rk-accent);
  cursor: pointer;
  font-size: 12px;
}
.tree-scroll {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
}
</style>

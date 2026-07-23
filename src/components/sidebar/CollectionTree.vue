<script setup lang="ts">
/** Renders the workspace's root nodes; bubbles node intent up to the Sidebar. */
import type { WorkspaceNode } from '../../types/workspace';
import CollectionTreeNode from './CollectionTreeNode.vue';

defineProps<{
  nodes: WorkspaceNode[];
  renamingId: string | null;
  activeRequestId: string | null;
  dirty: boolean;
}>();

const emit = defineEmits<{
  open: [id: string];
  menu: [payload: { id: string; x: number; y: number }];
  'rename-commit': [payload: { id: string; name: string }];
  'rename-cancel': [];
}>();
</script>

<template>
  <ul
    v-if="nodes.length"
    class="tree"
    role="tree"
  >
    <CollectionTreeNode
      v-for="node in nodes"
      :key="node.id"
      :node="node"
      :depth="0"
      :renaming-id="renamingId"
      :active-request-id="activeRequestId"
      :dirty="dirty"
      @open="(id) => emit('open', id)"
      @menu="(payload) => emit('menu', payload)"
      @rename-commit="(payload) => emit('rename-commit', payload)"
      @rename-cancel="emit('rename-cancel')"
    />
  </ul>
  <p
    v-else
    class="tree-empty"
  >
    No collections yet. Create one to get started.
  </p>
</template>

<style scoped>
.tree {
  margin: 0;
  padding: 4px;
  list-style: none;
}
.tree-empty {
  padding: 16px 12px;
  font-size: 12px;
  color: var(--rk-muted);
}
</style>

<script setup lang="ts">
/** One tree row + its children (recursive). Presentational: it emits intent
 *  and the Sidebar performs the store mutations. */
import { ref } from 'vue';
import type { WorkspaceNode } from '../../types/workspace';
import InlineRename from '../shared/InlineRename.vue';

const props = defineProps<{
  node: WorkspaceNode;
  depth: number;
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

const expanded = ref(true);
const isContainer = props.node.kind !== 'request';

function onRowClick(): void {
  if (isContainer) {
    expanded.value = !expanded.value;
  } else {
    emit('open', props.node.id);
  }
}

function onContextMenu(event: MouseEvent): void {
  emit('menu', { id: props.node.id, x: event.clientX, y: event.clientY });
}
</script>

<template>
  <li class="tree-node">
    <div
      class="row"
      :class="{ active: node.id === activeRequestId }"
      :style="{ paddingLeft: `${depth * 14 + 8}px` }"
      role="treeitem"
      @click="onRowClick"
      @contextmenu.prevent="onContextMenu"
    >
      <span
        v-if="isContainer"
        class="caret"
      >{{ expanded ? '▾' : '▸' }}</span>
      <span
        v-else
        class="caret dot"
      >•</span>

      <InlineRename
        v-if="node.id === renamingId"
        :model-value="node.name"
        @commit="(name) => emit('rename-commit', { id: node.id, name })"
        @cancel="emit('rename-cancel')"
      />
      <span
        v-else
        class="label"
      >{{ node.name }}</span>

      <span
        v-if="node.kind === 'request' && node.id === activeRequestId && dirty"
        class="dirty-dot"
        title="unsaved changes"
      />
    </div>

    <ul
      v-if="isContainer && expanded && node.children && node.children.length"
      class="children"
    >
      <CollectionTreeNode
        v-for="child in node.children"
        :key="child.id"
        :node="child"
        :depth="depth + 1"
        :renaming-id="renamingId"
        :active-request-id="activeRequestId"
        :dirty="dirty"
        @open="(id) => emit('open', id)"
        @menu="(payload) => emit('menu', payload)"
        @rename-commit="(payload) => emit('rename-commit', payload)"
        @rename-cancel="emit('rename-cancel')"
      />
    </ul>
  </li>
</template>

<style scoped>
.tree-node {
  list-style: none;
}
.row {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 8px;
  font-size: 13px;
  cursor: pointer;
  border-radius: 4px;
  user-select: none;
}
.row:hover {
  background: color-mix(in srgb, var(--rk-fg) 8%, transparent);
}
.row.active {
  background: color-mix(in srgb, var(--rk-accent) 18%, transparent);
}
.caret {
  width: 12px;
  color: var(--rk-muted);
  font-size: 10px;
  flex-shrink: 0;
}
.caret.dot {
  font-size: 12px;
}
.label {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--rk-fg);
}
.dirty-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: var(--rk-accent);
  flex-shrink: 0;
}
.children {
  margin: 0;
  padding: 0;
}
</style>

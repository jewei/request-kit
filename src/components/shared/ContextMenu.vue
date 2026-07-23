<script setup lang="ts">
/** A teleported, position-anchored menu. Closes on selection, outside click,
 *  or Escape. The parent owns visibility. */
import { onBeforeUnmount, onMounted } from 'vue';

export interface MenuItem {
  label: string;
  value: string;
  danger?: boolean;
}

defineProps<{ items: MenuItem[]; x: number; y: number }>();
const emit = defineEmits<{ select: [value: string]; close: [] }>();

function onKey(event: KeyboardEvent): void {
  if (event.key === 'Escape') emit('close');
}
onMounted(() => window.addEventListener('keydown', onKey));
onBeforeUnmount(() => window.removeEventListener('keydown', onKey));
</script>

<template>
  <teleport to="body">
    <div
      class="ctx-backdrop"
      @click="emit('close')"
      @contextmenu.prevent="emit('close')"
    />
    <ul
      class="ctx-menu"
      :style="{ left: `${x}px`, top: `${y}px` }"
      role="menu"
    >
      <li
        v-for="item in items"
        :key="item.value"
        class="ctx-item"
        :class="{ danger: item.danger }"
        role="menuitem"
        @click="emit('select', item.value)"
      >
        {{ item.label }}
      </li>
    </ul>
  </teleport>
</template>

<style scoped>
.ctx-backdrop {
  position: fixed;
  inset: 0;
  z-index: 40;
}
.ctx-menu {
  position: fixed;
  z-index: 41;
  min-width: 160px;
  margin: 0;
  padding: 4px;
  list-style: none;
  background: var(--rk-bg);
  border: 1px solid var(--rk-border);
  border-radius: 6px;
  box-shadow: 0 6px 24px rgba(0, 0, 0, 0.18);
}
.ctx-item {
  padding: 6px 10px;
  font-size: 13px;
  border-radius: 4px;
  cursor: pointer;
  color: var(--rk-fg);
}
.ctx-item:hover {
  background: var(--rk-accent);
  color: #fff;
}
.ctx-item.danger {
  color: #dc2626;
}
.ctx-item.danger:hover {
  background: #dc2626;
  color: #fff;
}
</style>

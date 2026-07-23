<script setup lang="ts">
/** Inline text edit for renaming a tree node. Enter/blur commit, Escape cancels.
 *  Commits at most once (blur after Enter is a no-op). */
import { onMounted, ref } from 'vue';

const props = defineProps<{ modelValue: string }>();
const emit = defineEmits<{ commit: [value: string]; cancel: [] }>();

const text = ref(props.modelValue);
const input = ref<HTMLInputElement | null>(null);
let settled = false;

onMounted(() => {
  input.value?.focus();
  input.value?.select();
});

function commit(): void {
  if (settled) return;
  settled = true;
  const trimmed = text.value.trim();
  if (trimmed === '' || trimmed === props.modelValue) {
    emit('cancel');
  } else {
    emit('commit', trimmed);
  }
}

function cancel(): void {
  if (settled) return;
  settled = true;
  emit('cancel');
}
</script>

<template>
  <input
    ref="input"
    v-model="text"
    class="inline-rename"
    type="text"
    spellcheck="false"
    @keydown.enter.prevent="commit"
    @keydown.esc.prevent="cancel"
    @blur="commit"
    @click.stop
  >
</template>

<style scoped>
.inline-rename {
  width: 100%;
  font-size: 13px;
  padding: 1px 4px;
  border: 1px solid var(--rk-accent);
  border-radius: 3px;
  background: var(--rk-bg);
  color: var(--rk-fg);
}
.inline-rename:focus {
  outline: none;
}
</style>

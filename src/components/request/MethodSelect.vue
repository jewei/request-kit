<script setup lang="ts">
import type { HttpMethod } from '../../types/request';

defineProps<{ modelValue: HttpMethod }>();
const emit = defineEmits<{ 'update:modelValue': [value: HttpMethod] }>();

const METHODS: HttpMethod[] = ['GET', 'POST', 'PUT', 'PATCH', 'DELETE', 'HEAD', 'OPTIONS'];

function onChange(event: Event): void {
  emit('update:modelValue', (event.target as HTMLSelectElement).value as HttpMethod);
}
</script>

<template>
  <select
    class="method-select"
    :value="modelValue"
    :data-method="modelValue"
    @change="onChange"
  >
    <option
      v-for="method in METHODS"
      :key="method"
      :value="method"
    >
      {{ method }}
    </option>
  </select>
</template>

<style scoped>
.method-select {
  font-family: ui-monospace, Menlo, Consolas, monospace;
  font-weight: 700;
  font-size: 12px;
  padding: 6px 8px;
  border: 1px solid var(--rk-border);
  border-radius: 4px;
  background: var(--rk-bg);
  color: var(--rk-accent);
}
.method-select[data-method='GET'] {
  color: #16a34a;
}
.method-select[data-method='POST'] {
  color: #d97706;
}
.method-select[data-method='PUT'] {
  color: #2563eb;
}
.method-select[data-method='PATCH'] {
  color: #9333ea;
}
.method-select[data-method='DELETE'] {
  color: #dc2626;
}
</style>

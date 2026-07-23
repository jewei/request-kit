<script setup lang="ts">
/** Search box for filtering the collections tree by node name. */
defineProps<{ query: string }>();
const emit = defineEmits<{ 'update:query': [value: string] }>();
</script>

<template>
  <div class="search">
    <input
      class="search-input"
      type="search"
      spellcheck="false"
      placeholder="Search requests…"
      :value="query"
      @input="emit('update:query', ($event.target as HTMLInputElement).value)"
    >
  </div>
</template>

<style scoped>
.search {
  padding: 6px 8px;
  border-bottom: 1px solid var(--rk-border);
}
.search-input {
  width: 100%;
  font-size: 12px;
  padding: 5px 8px;
  border: 1px solid var(--rk-border);
  border-radius: 4px;
  background: var(--rk-bg);
  color: var(--rk-fg);
}
.search-input:focus {
  outline: none;
  border-color: var(--rk-accent);
}
</style>

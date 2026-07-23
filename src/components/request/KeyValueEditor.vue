<script setup lang="ts">
/**
 * Shared key/value rows editor for Params (QueryParam rows) and Headers
 * (KeyValueRow rows). Rows are edited via granular events so the store-bound
 * parent owns all mutations. A trailing blank row is always available.
 */
export interface EditableRow {
  id: string;
  key: string;
  value: string;
  enabled: boolean;
  description?: string;
}

const props = defineProps<{
  rows: EditableRow[];
  keyPlaceholder?: string;
  valuePlaceholder?: string;
}>();

const emit = defineEmits<{
  edit: [id: string, patch: Partial<Pick<EditableRow, 'key' | 'value' | 'enabled' | 'description'>>];
  remove: [id: string];
  add: [patch: Partial<Pick<EditableRow, 'key' | 'value'>>];
}>();

function onField(
  row: EditableRow | null,
  field: 'key' | 'value' | 'description',
  event: Event,
): void {
  const value = (event.target as HTMLInputElement).value;
  if (row === null) {
    // Typing into the trailing blank row creates a real row.
    if (value !== '') emit('add', { [field === 'description' ? 'key' : field]: value });
    return;
  }
  emit('edit', row.id, { [field]: value });
}

function onToggle(row: EditableRow, event: Event): void {
  emit('edit', row.id, { enabled: (event.target as HTMLInputElement).checked });
}

defineExpose({ rowCount: () => props.rows.length });
</script>

<template>
  <div class="kv-editor" role="table">
    <div v-for="row in rows" :key="row.id" class="kv-row" role="row">
      <input
        class="kv-check"
        type="checkbox"
        :checked="row.enabled"
        :aria-label="`toggle ${row.key || 'row'}`"
        @change="onToggle(row, $event)"
      />
      <input
        class="kv-input kv-key"
        type="text"
        spellcheck="false"
        :placeholder="keyPlaceholder ?? 'key'"
        :value="row.key"
        @input="onField(row, 'key', $event)"
      />
      <input
        class="kv-input kv-value"
        type="text"
        spellcheck="false"
        :placeholder="valuePlaceholder ?? 'value'"
        :value="row.value"
        @input="onField(row, 'value', $event)"
      />
      <input
        class="kv-input kv-desc"
        type="text"
        placeholder="description"
        :value="row.description ?? ''"
        @input="onField(row, 'description', $event)"
      />
      <button class="kv-delete" :aria-label="`delete ${row.key || 'row'}`" @click="emit('remove', row.id)">
        ×
      </button>
    </div>

    <div class="kv-row kv-row-blank" role="row">
      <input class="kv-check" type="checkbox" checked disabled aria-label="new row enabled" />
      <input
        class="kv-input kv-key"
        type="text"
        spellcheck="false"
        :placeholder="keyPlaceholder ?? 'key'"
        value=""
        @input="onField(null, 'key', $event)"
      />
      <input
        class="kv-input kv-value"
        type="text"
        spellcheck="false"
        :placeholder="valuePlaceholder ?? 'value'"
        value=""
        @input="onField(null, 'value', $event)"
      />
      <input class="kv-input kv-desc" type="text" placeholder="description" value="" disabled />
      <span class="kv-delete-spacer" />
    </div>
  </div>
</template>

<style scoped>
.kv-editor {
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 8px 12px;
  overflow-y: auto;
}
.kv-row {
  display: grid;
  grid-template-columns: 24px 1fr 1.4fr 1fr 24px;
  gap: 6px;
  align-items: center;
}
.kv-input {
  font-family: ui-monospace, Menlo, Consolas, monospace;
  font-size: 12px;
  padding: 5px 8px;
  border: 1px solid var(--rk-border);
  border-radius: 3px;
  background: var(--rk-bg);
  color: var(--rk-fg);
  min-width: 0;
}
.kv-input:focus {
  outline: none;
  border-color: var(--rk-accent);
}
.kv-delete {
  border: none;
  background: none;
  color: var(--rk-muted);
  font-size: 16px;
  cursor: pointer;
  padding: 0;
}
.kv-delete:hover {
  color: #dc2626;
}
.kv-row-blank .kv-input {
  opacity: 0.75;
}
</style>

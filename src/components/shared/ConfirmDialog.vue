<script setup lang="ts">
/** A teleported confirm modal. The parent owns visibility via v-if. */
withDefaults(
  defineProps<{ title: string; message: string; confirmLabel?: string; danger?: boolean }>(),
  { confirmLabel: 'Confirm', danger: false },
);
const emit = defineEmits<{ confirm: []; cancel: [] }>();
</script>

<template>
  <teleport to="body">
    <div
      class="modal-backdrop"
      @click="emit('cancel')"
    >
      <div
        class="modal"
        role="dialog"
        aria-modal="true"
        @click.stop
      >
        <h2 class="modal-title">
          {{ title }}
        </h2>
        <p class="modal-message">
          {{ message }}
        </p>
        <div class="modal-actions">
          <button
            class="btn"
            @click="emit('cancel')"
          >
            Cancel
          </button>
          <button
            class="btn"
            :class="{ danger }"
            @click="emit('confirm')"
          >
            {{ confirmLabel }}
          </button>
        </div>
      </div>
    </div>
  </teleport>
</template>

<style scoped>
.modal-backdrop {
  position: fixed;
  inset: 0;
  z-index: 50;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, 0.4);
}
.modal {
  width: 360px;
  max-width: 90vw;
  padding: 20px;
  background: var(--rk-bg);
  border: 1px solid var(--rk-border);
  border-radius: 8px;
  box-shadow: 0 12px 40px rgba(0, 0, 0, 0.3);
}
.modal-title {
  margin: 0 0 8px;
  font-size: 15px;
}
.modal-message {
  margin: 0 0 18px;
  font-size: 13px;
  color: var(--rk-muted);
}
.modal-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}
.btn {
  padding: 6px 14px;
  font-size: 13px;
  border: 1px solid var(--rk-border);
  border-radius: 4px;
  background: var(--rk-bg);
  color: var(--rk-fg);
  cursor: pointer;
}
.btn.danger {
  background: #dc2626;
  border-color: #dc2626;
  color: #fff;
}
</style>

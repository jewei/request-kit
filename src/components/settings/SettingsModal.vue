<script setup lang="ts">
/** Teleported settings modal. Each control writes through the settings store,
 *  which applies theme/font live and persists. The parent owns visibility. */
import { computed } from 'vue';
import { useSettingsStore } from '../../stores/settings';

const settings = useSettingsStore();
const emit = defineEmits<{ close: [] }>();

const maxBodyMb = computed(() => Math.round(settings.settings.maxBodyBytes / (1024 * 1024)));

function setTheme(event: Event): void {
  const theme = (event.target as HTMLSelectElement).value as 'system' | 'light' | 'dark';
  void settings.update({ theme });
}
function setNumber(key: 'fontSize' | 'timeoutMs' | 'editorLargeFileKb', event: Event): void {
  const value = Number((event.target as HTMLInputElement).value);
  if (Number.isFinite(value) && value > 0) void settings.update({ [key]: value });
}
function setMaxBodyMb(event: Event): void {
  const mb = Number((event.target as HTMLInputElement).value);
  if (Number.isFinite(mb) && mb > 0) void settings.update({ maxBodyBytes: mb * 1024 * 1024 });
}
function setRedirects(event: Event): void {
  void settings.update({ followRedirects: (event.target as HTMLInputElement).checked });
}
</script>

<template>
  <teleport to="body">
    <div
      class="modal-backdrop"
      @click="emit('close')"
    >
      <div
        class="modal"
        role="dialog"
        aria-modal="true"
        @click.stop
      >
        <header class="modal-head">
          <h2>Settings</h2>
          <button
            class="close"
            aria-label="close settings"
            @click="emit('close')"
          >
            ×
          </button>
        </header>

        <label class="field">
          <span>Theme</span>
          <select
            :value="settings.settings.theme"
            @change="setTheme"
          >
            <option value="system">System</option>
            <option value="light">Light</option>
            <option value="dark">Dark</option>
          </select>
        </label>

        <label class="field">
          <span>Font size (px)</span>
          <input
            type="number"
            min="8"
            :value="settings.settings.fontSize"
            @change="(e) => setNumber('fontSize', e)"
          >
        </label>

        <label class="field">
          <span>Default timeout (ms)</span>
          <input
            type="number"
            min="1"
            :value="settings.settings.timeoutMs"
            @change="(e) => setNumber('timeoutMs', e)"
          >
        </label>

        <label class="field">
          <span>Follow redirects by default</span>
          <input
            type="checkbox"
            :checked="settings.settings.followRedirects"
            @change="setRedirects"
          >
        </label>

        <label class="field">
          <span>Max response size (MB)</span>
          <input
            type="number"
            min="1"
            :value="maxBodyMb"
            @change="setMaxBodyMb"
          >
        </label>

        <label class="field">
          <span>Large-file editor threshold (KB)</span>
          <input
            type="number"
            min="1"
            :value="settings.settings.editorLargeFileKb"
            @change="(e) => setNumber('editorLargeFileKb', e)"
          >
        </label>

        <p class="disclosure">
          Variable values and request credentials are stored unencrypted under
          <code>~/.request-kit</code>.
        </p>
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
  width: 420px;
  max-width: 92vw;
  padding: 20px;
  background: var(--rk-bg);
  border: 1px solid var(--rk-border);
  border-radius: 8px;
  box-shadow: 0 12px 40px rgba(0, 0, 0, 0.3);
}
.modal-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 12px;
}
.modal-head h2 {
  margin: 0;
  font-size: 15px;
}
.close {
  border: none;
  background: none;
  font-size: 20px;
  line-height: 1;
  color: var(--rk-muted);
  cursor: pointer;
}
.field {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 7px 0;
  font-size: 13px;
}
.field input[type='number'],
.field select {
  width: 160px;
  padding: 5px 8px;
  font-size: 12px;
  border: 1px solid var(--rk-border);
  border-radius: 3px;
  background: var(--rk-bg);
  color: var(--rk-fg);
}
.disclosure {
  margin: 14px 0 0;
  font-size: 11px;
  color: var(--rk-muted);
}
.disclosure code {
  font-family: ui-monospace, Menlo, Consolas, monospace;
}
</style>

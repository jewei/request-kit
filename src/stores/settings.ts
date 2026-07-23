/**
 * App settings store. Loads from the workspace bootstrap (or disk), persists
 * changes, and applies theme + font size to the document root. "system" theme
 * follows the OS via matchMedia.
 */
import { defineStore } from 'pinia';
import { ref } from 'vue';
import { readSettings, writeSettings } from '../ipc/commands';
import { DEFAULT_SETTINGS, type Settings } from '../types/settings';

const DARK_QUERY = '(prefers-color-scheme: dark)';

export const useSettingsStore = defineStore('settings', () => {
  const settings = ref<Settings>({ ...DEFAULT_SETTINGS });
  let listenerAttached = false;

  function prefersDark(): boolean {
    if (typeof window === 'undefined' || !window.matchMedia) return false;
    return window.matchMedia(DARK_QUERY).matches;
  }

  function effectiveTheme(): 'light' | 'dark' {
    if (settings.value.theme === 'system') return prefersDark() ? 'dark' : 'light';
    return settings.value.theme;
  }

  function attachSystemListener(): void {
    if (listenerAttached || typeof window === 'undefined' || !window.matchMedia) return;
    const mq = window.matchMedia(DARK_QUERY);
    if (!mq.addEventListener) return;
    listenerAttached = true;
    mq.addEventListener('change', () => {
      if (settings.value.theme === 'system') applyTheme();
    });
  }

  function applyTheme(): void {
    if (typeof document === 'undefined') return;
    document.documentElement.dataset.theme = effectiveTheme();
    document.documentElement.style.setProperty('--rk-font-size', `${settings.value.fontSize}px`);
    attachSystemListener();
  }

  /** Merge a raw settings object (from the bootstrap or disk) and apply it. */
  function load(raw: unknown): void {
    if (raw && typeof raw === 'object') {
      settings.value = { ...DEFAULT_SETTINGS, ...(raw as Partial<Settings>), version: 1 };
    }
    applyTheme();
  }

  async function loadFromDisk(): Promise<void> {
    load(await readSettings());
  }

  /** Merge a patch, apply it live, and persist. */
  async function update(patch: Partial<Settings>): Promise<void> {
    settings.value = { ...settings.value, ...patch, version: 1 };
    applyTheme();
    await writeSettings(settings.value);
  }

  return { settings, load, loadFromDisk, update, applyTheme };
});

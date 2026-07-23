import { createPinia, setActivePinia } from 'pinia';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { useSettingsStore } from './settings';
import * as commands from '../ipc/commands';

vi.mock('../ipc/commands', () => ({
  readSettings: vi.fn(),
  writeSettings: vi.fn().mockResolvedValue(undefined),
}));

const originalMatchMedia = window.matchMedia;

function stubMatchMedia(matches: boolean): void {
  window.matchMedia = vi.fn().mockReturnValue({
    matches,
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
  }) as unknown as typeof window.matchMedia;
}

describe('settings store', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.clearAllMocks();
    vi.mocked(commands.writeSettings).mockResolvedValue(undefined);
    delete document.documentElement.dataset.theme;
  });

  afterEach(() => {
    window.matchMedia = originalMatchMedia;
  });

  it('applies an explicit dark theme and font size to the document root', async () => {
    const store = useSettingsStore();
    await store.update({ theme: 'dark', fontSize: 16 });

    expect(document.documentElement.dataset.theme).toBe('dark');
    expect(document.documentElement.style.getPropertyValue('--rk-font-size')).toBe('16px');
    expect(commands.writeSettings).toHaveBeenCalledWith(
      expect.objectContaining({ theme: 'dark', fontSize: 16, version: 1 }),
    );
  });

  it('resolves the system theme via matchMedia', async () => {
    stubMatchMedia(true); // OS prefers dark
    const store = useSettingsStore();
    await store.update({ theme: 'system' });
    expect(document.documentElement.dataset.theme).toBe('dark');

    stubMatchMedia(false); // OS prefers light
    store.applyTheme();
    expect(document.documentElement.dataset.theme).toBe('light');
  });

  it('load merges raw settings over the defaults', () => {
    const store = useSettingsStore();
    store.load({ theme: 'light', timeoutMs: 5000 });
    expect(store.settings.theme).toBe('light');
    expect(store.settings.timeoutMs).toBe(5000);
    expect(store.settings.maxBodyBytes).toBe(10 * 1024 * 1024); // default kept
  });
});

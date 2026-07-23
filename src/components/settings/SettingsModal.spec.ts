import { mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import SettingsModal from './SettingsModal.vue';

vi.mock('../../ipc/commands', () => ({
  readSettings: vi.fn(),
  writeSettings: vi.fn().mockResolvedValue(undefined),
}));

import * as commands from '../../ipc/commands';

describe('SettingsModal', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.clearAllMocks();
    vi.mocked(commands.writeSettings).mockResolvedValue(undefined);
    delete document.documentElement.dataset.theme;
  });

  it('persists and applies a theme change', async () => {
    const wrapper = mount(SettingsModal, { global: { stubs: { teleport: true } } });
    await wrapper.find('select').setValue('dark');

    expect(commands.writeSettings).toHaveBeenCalledWith(
      expect.objectContaining({ theme: 'dark' }),
    );
    expect(document.documentElement.dataset.theme).toBe('dark');
  });

  it('emits close from the close button', async () => {
    const wrapper = mount(SettingsModal, { global: { stubs: { teleport: true } } });
    await wrapper.find('.close').trigger('click');
    expect(wrapper.emitted('close')).toBeTruthy();
  });
});

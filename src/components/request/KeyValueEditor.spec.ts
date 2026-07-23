import { mount } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';
import KeyValueEditor from './KeyValueEditor.vue';

describe('KeyValueEditor', () => {
  it('emits `add` when the trailing blank row is typed into', async () => {
    const wrapper = mount(KeyValueEditor, { props: { rows: [] } });

    await wrapper.find('.kv-row-blank .kv-key').setValue('X-Token');

    expect(wrapper.emitted('add')).toEqual([[{ key: 'X-Token' }]]);
    // Typing into the blank row never edits or removes an existing row.
    expect(wrapper.emitted('edit')).toBeUndefined();
  });

  it('emits `edit` with the row id when an existing row changes', async () => {
    const wrapper = mount(KeyValueEditor, {
      props: {
        rows: [{ id: 'r1', key: 'k', value: 'v', enabled: true }],
      },
    });

    await wrapper.find('.kv-row:not(.kv-row-blank) .kv-value').setValue('v2');

    expect(wrapper.emitted('edit')).toEqual([['r1', { value: 'v2' }]]);
  });
});

import { mount } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';
import type { HistoryEntry } from '../../types/history';
import HistoryList from './HistoryList.vue';

function entry(over: Partial<HistoryEntry>): HistoryEntry {
  return {
    version: 1,
    id: 'h1',
    executedAt: '2026-07-24T00:00:00.000Z',
    method: 'GET',
    templateUrl: 'https://api.example.com/?token=<redacted>',
    status: 200,
    durationMs: 10,
    bodyBytes: 2,
    requestId: null,
    errorKind: null,
    ...over,
  };
}

const entries = [
  entry({ id: 'h1' }),
  entry({ id: 'h2', method: 'POST', status: null, errorKind: 'dns', templateUrl: 'https://nope/' }),
];

describe('HistoryList', () => {
  it('renders template URLs and error kinds without leaking secrets', () => {
    const wrapper = mount(HistoryList, { props: { entries } });
    const text = wrapper.text();
    expect(text).toContain('token=<redacted>');
    expect(text).not.toContain('secret');
    expect(text).toContain('dns');
  });

  it('emits replay with the entry when a row is clicked', async () => {
    const wrapper = mount(HistoryList, { props: { entries } });
    await wrapper.findAll('.history-row')[0].trigger('click');
    expect(wrapper.emitted('replay')?.[0][0]).toMatchObject({ id: 'h1' });
  });

  it('emits clear from the Clear button', async () => {
    const wrapper = mount(HistoryList, { props: { entries } });
    await wrapper.find('.clear-btn').trigger('click');
    expect(wrapper.emitted('clear')).toBeTruthy();
  });

  it('shows an empty state with no entries', () => {
    const wrapper = mount(HistoryList, { props: { entries: [] } });
    expect(wrapper.text()).toContain('No requests yet');
  });
});

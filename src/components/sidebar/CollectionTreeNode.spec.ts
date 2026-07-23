import { mount } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';
import type { WorkspaceNode } from '../../types/workspace';
import CollectionTreeNode from './CollectionTreeNode.vue';

const tree: WorkspaceNode = {
  id: 'c1',
  kind: 'collection',
  name: 'API',
  children: [{ id: 'r1', kind: 'request', name: 'Ping' }],
};

function mountNode() {
  return mount(CollectionTreeNode, {
    props: { node: tree, depth: 0, renamingId: null, activeRequestId: null, dirty: false },
  });
}

describe('CollectionTreeNode', () => {
  it('renders the collection and its nested request', () => {
    const wrapper = mountNode();
    expect(wrapper.text()).toContain('API');
    expect(wrapper.text()).toContain('Ping');
  });

  it('emits open with the request id when a request row is clicked', async () => {
    const wrapper = mountNode();
    const rows = wrapper.findAll('.row');
    // rows[0] is the collection; rows[1] is the nested request.
    await rows[1].trigger('click');
    expect(wrapper.emitted('open')).toEqual([['r1']]);
  });

  it('emits menu with the node id on right-click', async () => {
    const wrapper = mountNode();
    await wrapper.findAll('.row')[0].trigger('contextmenu');
    const menu = wrapper.emitted('menu');
    expect(menu).toBeTruthy();
    expect((menu![0][0] as { id: string }).id).toBe('c1');
  });

  it('does not emit open when a container row is clicked (it toggles instead)', async () => {
    const wrapper = mountNode();
    await wrapper.findAll('.row')[0].trigger('click');
    expect(wrapper.emitted('open')).toBeUndefined();
  });
});

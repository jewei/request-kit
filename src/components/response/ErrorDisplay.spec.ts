import { mount } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';
import ErrorDisplay from './ErrorDisplay.vue';

describe('ErrorDisplay', () => {
  it('renders the friendly DNS message and exposes detail in a collapsed <details>', () => {
    const wrapper = mount(ErrorDisplay, {
      props: {
        error: { kind: 'dns', message: 'raw resolver error', detail: 'lookup nope.invalid' },
      },
    });

    expect(wrapper.find('.error-headline').text()).toBe(
      'Could not resolve the host. Check the URL or your network.',
    );
    const details = wrapper.find('details');
    expect(details.exists()).toBe(true);
    expect(details.text()).toContain('lookup nope.invalid');
    // Detail is collapsed by default — no `open` attribute.
    expect(details.attributes('open')).toBeUndefined();
  });

  it('renders a cancelled request muted rather than as an error', () => {
    const wrapper = mount(ErrorDisplay, {
      props: { error: { kind: 'cancelled', message: 'cancelled' } },
    });

    expect(wrapper.find('.error-display').classes()).toContain('is-cancelled');
    // Cancellation carries no detail chain, so no <details> is shown.
    expect(wrapper.find('details').exists()).toBe(false);
  });
});

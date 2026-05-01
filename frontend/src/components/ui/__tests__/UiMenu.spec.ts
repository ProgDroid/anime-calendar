import { describe, it, expect } from 'vitest';
import { mount } from '@vue/test-utils';
import { nextTick } from 'vue';
import UiMenu from '../UiMenu.vue';

const slots = {
  trigger: '<button data-testid="t">open</button>',
  default: '<button data-testid="i1">A</button><button data-testid="i2">B</button>',
};

describe('UiMenu', () => {
  it('starts closed: items not in DOM', () => {
    const wrapper = mount(UiMenu, { slots });
    expect(wrapper.find('[data-testid="i1"]').exists()).toBe(false);
  });

  it('opens on trigger click and renders items', async () => {
    const wrapper = mount(UiMenu, { slots, attachTo: document.body });
    await wrapper.find('[data-testid="t"]').trigger('click');
    expect(wrapper.find('[data-testid="i1"]').exists()).toBe(true);
    wrapper.unmount();
  });

  it('closes after a click inside the menu', async () => {
    const wrapper = mount(UiMenu, { slots, attachTo: document.body });
    await wrapper.find('[data-testid="t"]').trigger('click');
    await wrapper.find('[data-testid="i1"]').trigger('click');
    await nextTick();
    expect(wrapper.find('[data-testid="i1"]').exists()).toBe(false);
    wrapper.unmount();
  });

  it('closes on Escape', async () => {
    const wrapper = mount(UiMenu, { slots, attachTo: document.body });
    await wrapper.find('[data-testid="t"]').trigger('click');
    expect(wrapper.find('[data-testid="i1"]').exists()).toBe(true);
    await wrapper.trigger('keydown', { key: 'Escape' });
    await nextTick();
    expect(wrapper.find('[data-testid="i1"]').exists()).toBe(false);
    wrapper.unmount();
  });
});

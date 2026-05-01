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

  it('ArrowDown moves focus to next item; ArrowUp wraps to last', async () => {
    const wrapper = mount(UiMenu, { slots, attachTo: document.body });
    await wrapper.find('[data-testid="t"]').trigger('click');
    await nextTick();
    await nextTick();
    const i1 = wrapper.find('[data-testid="i1"]').element as HTMLElement;
    const i2 = wrapper.find('[data-testid="i2"]').element as HTMLElement;
    expect(document.activeElement).toBe(i1);
    await wrapper.trigger('keydown', { key: 'ArrowDown' });
    expect(document.activeElement).toBe(i2);
    await wrapper.trigger('keydown', { key: 'ArrowUp' });
    expect(document.activeElement).toBe(i1);
    await wrapper.trigger('keydown', { key: 'ArrowUp' });
    expect(document.activeElement).toBe(i2);
    await wrapper.trigger('keydown', { key: 'Home' });
    expect(document.activeElement).toBe(i1);
    await wrapper.trigger('keydown', { key: 'End' });
    expect(document.activeElement).toBe(i2);
    wrapper.unmount();
  });

  it('exposes panelId via trigger slot props for aria-controls', async () => {
    const triggerSlot = `
      <template #trigger="{ open, panelId }">
        <button data-testid="t" :aria-expanded="open" :aria-controls="panelId">open</button>
      </template>
    `;
    const wrapper = mount(UiMenu, {
      slots: {
        ...slots,
        trigger: triggerSlot,
      },
      attachTo: document.body,
    });
    const trigger = wrapper.find('[data-testid="t"]');
    expect(trigger.attributes('aria-expanded')).toBe('false');
    expect(trigger.attributes('aria-controls')).toBeTruthy();
    const panelId = trigger.attributes('aria-controls')!;
    await trigger.trigger('click');
    expect(trigger.attributes('aria-expanded')).toBe('true');
    expect(document.getElementById(panelId)).not.toBeNull();
    wrapper.unmount();
  });
});

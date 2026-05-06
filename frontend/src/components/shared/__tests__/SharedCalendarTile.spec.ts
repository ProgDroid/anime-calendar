import { describe, it, expect, vi } from 'vitest';
import { mount, flushPromises } from '@vue/test-utils';
import { createI18n } from 'vue-i18n';
import { createPinia } from 'pinia';
import { createRouter, createMemoryHistory } from 'vue-router';
import en from '@/locales/en.json';
import SharedCalendarTile from '../SharedCalendarTile.vue';

vi.mock('@/config/api', () => ({
  default: {
    get: vi.fn(async () => ({ data: [] })),
  },
}));

const i18n = createI18n({ legacy: false, locale: 'en', messages: { en } });
const router = createRouter({ history: createMemoryHistory(), routes: [{ path: '/', component: { template: '<div />' } }] });
const pinia = createPinia();

const baseCalendar = {
  id: 42,
  name: 'Shared Calendar',
  item_count: 5,
  airing_count: 0,
  created_at: '2026-04-01T00:00:00Z',
  updated_at: '2026-04-02T00:00:00Z',
  recent_item_ids: [],
  owner: {
    id: 99,
    display: 'Alice',
    avatar: null,
  },
};

function mountTile(props: Partial<Record<string, unknown>> = {}) {
  return mount(SharedCalendarTile, {
    global: { plugins: [i18n, pinia, router] },
    props: { calendar: baseCalendar, ...props },
    attachTo: document.body,
  });
}

describe('SharedCalendarTile', () => {
  it('renders menu button', async () => {
    const wrapper = mountTile();
    await flushPromises();
    expect(wrapper.find('[data-testid="shared-tile-menu-btn"]').exists()).toBe(true);
    wrapper.unmount();
  });

  it('leave menu item emits leave', async () => {
    const wrapper = mountTile();
    await flushPromises();
    // Click the menu button to open the menu
    await wrapper.find('[data-testid="shared-tile-menu-btn"]').trigger('click');
    // The leave-calendar-btn should now be visible
    const leaveBtn = wrapper.find('[data-testid="leave-calendar-btn"]');
    expect(leaveBtn.exists()).toBe(true);
    await leaveBtn.trigger('click');
    expect(wrapper.emitted('leave')).toBeTruthy();
    wrapper.unmount();
  });

  it('open emit still works when card body is clicked', async () => {
    const wrapper = mountTile();
    await flushPromises();
    await wrapper.find('[data-testid="shared-calendar-tile-body"]').trigger('click');
    expect(wrapper.emitted('open')).toBeTruthy();
    wrapper.unmount();
  });
});

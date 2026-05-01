import { describe, it, expect, vi } from 'vitest';
import { mount, flushPromises } from '@vue/test-utils';
import { createI18n } from 'vue-i18n';
import en from '@/locales/en.json';
import CalendarTile from '../CalendarTile.vue';

vi.mock('@/config/api', () => ({
  default: {
    get: vi.fn(async (url: string) => {
      const matches = [...url.matchAll(/id=(\d+)/g)].map((m) => Number(m[1]));
      return {
        data: matches.map((id) => ({
          id,
          cover_image: { large: `cover-${id}.jpg` },
        })),
      };
    }),
  },
}));

const i18n = createI18n({ legacy: false, locale: 'en', messages: { en } });

const baseCalendar = {
  id: 1,
  name: 'My Calendar',
  item_count: 12,
  subscription_token: 'tok',
  created_at: '2026-04-01T00:00:00Z',
  updated_at: '2026-04-02T00:00:00Z',
  recent_item_ids: [101, 102],
};

function mountTile(props: Partial<Record<string, unknown>> = {}) {
  return mount(CalendarTile, {
    global: { plugins: [i18n] },
    props: { calendar: baseCalendar, ...props },
    attachTo: document.body,
  });
}

describe('CalendarTile', () => {
  it('renders name, item count, and updated label', async () => {
    const wrapper = mountTile();
    await flushPromises();
    expect(wrapper.find('[data-testid="calendar-tile-name"]').text()).toBe('My Calendar');
    expect(wrapper.find('[data-testid="calendar-tile-count"]').text()).toContain('12');
    expect(wrapper.find('[data-testid="calendar-tile-updated"]').exists()).toBe(true);
    wrapper.unmount();
  });

  it('emits open when tile body is clicked', async () => {
    const wrapper = mountTile();
    await wrapper.find('[data-testid="calendar-tile-body"]').trigger('click');
    expect(wrapper.emitted('open')).toBeTruthy();
    wrapper.unmount();
  });

  it('emits edit when Edit button is clicked', async () => {
    const wrapper = mountTile();
    await wrapper.find('[data-testid="calendar-tile-edit"]').trigger('click');
    expect(wrapper.emitted('edit')).toBeTruthy();
    expect(wrapper.emitted('open')).toBeFalsy();
    wrapper.unmount();
  });

  it('emits delete when delete button is clicked (does not bubble open)', async () => {
    const wrapper = mountTile();
    await wrapper.find('[data-testid="calendar-tile-delete"]').trigger('click');
    expect(wrapper.emitted('delete')).toBeTruthy();
    expect(wrapper.emitted('open')).toBeFalsy();
    wrapper.unmount();
  });

  it('opens export menu and emits export-ics on click', async () => {
    const wrapper = mountTile();
    await wrapper.find('[data-testid="calendar-tile-export-trigger"]').trigger('click');
    expect(wrapper.find('[data-testid="calendar-tile-export-ics"]').exists()).toBe(true);
    await wrapper.find('[data-testid="calendar-tile-export-ics"]').trigger('click');
    expect(wrapper.emitted('export-ics')).toBeTruthy();
    wrapper.unmount();
  });

  it('opens kebab menu and emits delete from it', async () => {
    const wrapper = mountTile();
    await wrapper.find('[data-testid="calendar-tile-kebab-trigger"]').trigger('click');
    await wrapper.find('[data-testid="calendar-tile-kebab-delete"]').trigger('click');
    expect(wrapper.emitted('delete')).toBeTruthy();
    wrapper.unmount();
  });
});

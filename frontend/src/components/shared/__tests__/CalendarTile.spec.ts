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

describe('CalendarTile', () => {
  it('renders name, item count, and avatar slot', async () => {
    const wrapper = mount(CalendarTile, {
      global: { plugins: [i18n] },
      props: { calendar: baseCalendar, ownerAvatarUrl: '/avatar.png' },
    });
    await flushPromises();
    expect(wrapper.find('[data-testid="calendar-tile-name"]').text()).toBe('My Calendar');
    expect(wrapper.find('[data-testid="calendar-tile-count"]').text()).toContain('12');
    expect(wrapper.find('[data-testid="calendar-tile-avatar"]').exists()).toBe(true);
  });

  it('emits click', async () => {
    const wrapper = mount(CalendarTile, {
      global: { plugins: [i18n] },
      props: {
        calendar: { ...baseCalendar, recent_item_ids: [] },
      },
    });
    await wrapper.find('[data-testid="calendar-tile"]').trigger('click');
    expect(wrapper.emitted('click')).toBeTruthy();
  });

  it('does not render avatar when ownerAvatarUrl is omitted', async () => {
    const wrapper = mount(CalendarTile, {
      global: { plugins: [i18n] },
      props: { calendar: baseCalendar },
    });
    await flushPromises();
    expect(wrapper.find('[data-testid="calendar-tile-avatar"]').exists()).toBe(false);
  });
});

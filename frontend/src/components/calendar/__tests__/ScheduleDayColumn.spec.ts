import { describe, it, expect } from 'vitest';
import { mount } from '@vue/test-utils';
import { createI18n } from 'vue-i18n';
import ScheduleDayColumn from '../ScheduleDayColumn.vue';

const i18n = createI18n({
  legacy: false,
  locale: 'en',
  messages: {
    en: {
      schedule: {
        episode: 'Ep. {n}',
        noEntries: 'No entries',
      },
    },
    pt: {
      schedule: {
        episode: 'Ep. {n}',
        noEntries: 'Sem episódios',
      },
    },
  },
});

describe('ScheduleDayColumn', () => {
  it('renders day label, date, and entries', () => {
    const wrapper = mount(ScheduleDayColumn, {
      global: { plugins: [i18n] },
      props: {
        label: 'Mon',
        date: new Date(Date.UTC(2026, 4, 4)),
        entries: [{ id: 1, title: 'Show A', episode: 5, time: '20:00', coverUrl: 'a.jpg' }],
      },
    });
    expect(wrapper.find('[data-testid="day-label"]').text()).toBe('Mon');
    expect(wrapper.find('[data-testid="day-date"]').text()).toContain('4');
    expect(wrapper.findAll('[data-testid="schedule-entry"]')).toHaveLength(1);
  });

  it('shows empty state when no entries', () => {
    const wrapper = mount(ScheduleDayColumn, {
      global: { plugins: [i18n] },
      props: { label: 'Tue', date: new Date(), entries: [] },
    });
    expect(wrapper.find('[data-testid="day-empty"]').exists()).toBe(true);
  });
});

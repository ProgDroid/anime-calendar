import { describe, it, expect } from 'vitest';
import { mount } from '@vue/test-utils';
import { createI18n } from 'vue-i18n';
import { defineComponent, h } from 'vue';
import { useDayLabels } from '../useDayLabels';

const i18n = createI18n({
  legacy: false,
  locale: 'en',
  messages: {
    en: {
      schedule: {
        days: {
          mon: 'Mon',
          tue: 'Tue',
          wed: 'Wed',
          thu: 'Thu',
          fri: 'Fri',
          sat: 'Sat',
          sun: 'Sun',
        },
      },
    },
    pt: {
      schedule: {
        days: {
          mon: 'Seg',
          tue: 'Ter',
          wed: 'Qua',
          thu: 'Qui',
          fri: 'Sex',
          sat: 'Sáb',
          sun: 'Dom',
        },
      },
    },
  },
});

const Probe = defineComponent({
  setup() {
    const { labels } = useDayLabels();
    return () => h('div', { 'data-testid': 'out' }, labels.value.join(','));
  },
});

describe('useDayLabels', () => {
  it('returns 7 day labels Mon-Sun in EN', () => {
    i18n.global.locale.value = 'en';
    const wrapper = mount(Probe, { global: { plugins: [i18n] } });
    expect(wrapper.get('[data-testid="out"]').text()).toBe('Mon,Tue,Wed,Thu,Fri,Sat,Sun');
  });

  it('reacts to locale switch and re-renders in PT', async () => {
    i18n.global.locale.value = 'en';
    const wrapper = mount(Probe, { global: { plugins: [i18n] } });
    i18n.global.locale.value = 'pt';
    await wrapper.vm.$nextTick();
    expect(wrapper.get('[data-testid="out"]').text()).toBe('Seg,Ter,Qua,Qui,Sex,Sáb,Dom');
  });
});

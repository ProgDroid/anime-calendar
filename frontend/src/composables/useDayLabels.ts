import { computed } from 'vue';
import { useI18n } from 'vue-i18n';

export function useDayLabels() {
  const { t } = useI18n();
  const labels = computed(() => [
    t('schedule.days.mon'),
    t('schedule.days.tue'),
    t('schedule.days.wed'),
    t('schedule.days.thu'),
    t('schedule.days.fri'),
    t('schedule.days.sat'),
    t('schedule.days.sun'),
  ]);
  return { labels };
}

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useDayLabels } from '@/composables/useDayLabels'
import { useWeekRange, formatIsoWeek } from '@/composables/useWeekRange'
import { fetchScheduleForCalendar } from '@/services/calendars'
import type { ScheduleByDay } from '@/types/schedule'
import UiButton from '@/components/ui/UiButton.vue'
import ScheduleDayColumn from './ScheduleDayColumn.vue'

defineOptions({ name: 'CalendarScheduleView' })

const route = useRoute()
const router = useRouter()
const { t } = useI18n()
const { labels } = useDayLabels()

const week = computed(() => (route.query.week as string) || formatIsoWeek(new Date()))
const calendarId = computed(() => Number(route.params.id))
const { days } = useWeekRange(week)

const weekLabel = computed(() => {
  const m = week.value.match(/^(\d{4})-W(\d{2})$/)
  if (!m) return week.value
  return t('schedule.weekOf', { year: m[1], week: m[2] })
})

const entriesByDay = ref<ScheduleByDay>({})

let seq = 0
watch(
  [week, calendarId],
  async ([w, id]) => {
    if (!Number.isFinite(id)) return
    const my = ++seq
    try {
      const result = await fetchScheduleForCalendar(id, w)
      if (my === seq) entriesByDay.value = result
    } catch (err) {
      if (my === seq) {
        entriesByDay.value = {}
        console.error('[schedule] fetch failed', err)
      }
    }
  },
  { immediate: true },
)

function shiftWeek(delta: -1 | 1) {
  const base = new Date(days.value[0]!)
  base.setUTCDate(base.getUTCDate() + delta * 7)
  router.replace({ query: { ...route.query, week: formatIsoWeek(base) } })
}

function isoDay(d: Date) {
  return d.toISOString().slice(0, 10)
}
</script>

<template>
  <div class="flex flex-col gap-4">
    <div class="flex items-center justify-between">
      <UiButton variant="ghost" size="sm" data-testid="schedule-prev" @click="shiftWeek(-1)">
        {{ t('schedule.prev') }}
      </UiButton>
      <time
        data-testid="schedule-week"
        class="font-medium text-fg-1"
        :datetime="week"
        :aria-label="weekLabel"
      >{{ week }}</time>
      <UiButton variant="ghost" size="sm" data-testid="schedule-next" @click="shiftWeek(1)">
        {{ t('schedule.next') }}
      </UiButton>
    </div>
    <div class="grid grid-cols-1 sm:grid-cols-7 gap-3">
      <ScheduleDayColumn
        v-for="(d, i) in days"
        :key="isoDay(d)"
        :label="labels[i]!"
        :date="d"
        :entries="entriesByDay[isoDay(d)] ?? []"
      />
    </div>
  </div>
</template>

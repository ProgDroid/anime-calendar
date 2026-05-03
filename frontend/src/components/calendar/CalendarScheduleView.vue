<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useDayLabels } from '@/composables/useDayLabels'
import { useWeekRange, formatIsoWeek } from '@/composables/useWeekRange'
import { useViewportLayout } from '@/composables/useViewportLayout'
import { fetchScheduleForCalendar } from '@/services/calendars'
import type { ScheduleByDay } from '@/types/schedule'
import UiButton from '@/components/ui/UiButton.vue'
import ScheduleDayColumn from './ScheduleDayColumn.vue'

defineOptions({ name: 'CalendarScheduleView' })

const route = useRoute()
const router = useRouter()
const { t } = useI18n()
const { labels } = useDayLabels()
const { isMobile } = useViewportLayout()

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

// Mobile: active day pill index (0 = Monday). Reset to 0 on week change so
// the pill stays in sync with the visible week.
const activeDayIndex = ref(0)
watch(week, () => {
  activeDayIndex.value = 0
})

// Entries for the active pill day
const activeDayEntries = computed(() => {
  const d = days.value[activeDayIndex.value]
  if (!d) return []
  return entriesByDay.value[isoDay(d)] ?? []
})
</script>

<template>
  <!-- Mobile layout -->
  <div v-if="isMobile" class="flex flex-col gap-4 pb-tab-bar" data-testid="schedule-mobile">
    <header class="px-4 pt-4">
      <p class="text-xs uppercase tracking-wider text-fg-2">{{ t('mobile.schedule.eyebrow') }}</p>
      <h1 class="font-display text-3xl text-fg-1 mt-1">{{ t('mobile.schedule.title') }}</h1>
    </header>

    <!-- Day pills row -->
    <div class="flex gap-2 overflow-x-auto px-4 pb-2" data-testid="schedule-mobile-pills">
      <button
        v-for="(d, i) in days"
        :key="isoDay(d)"
        type="button"
        :data-testid="`schedule-mobile-pill-${i}`"
        :aria-current="i === activeDayIndex ? 'true' : undefined"
        :class="[
          'flex min-w-[46px] flex-col items-center rounded-md border py-2 px-2 transition-colors',
          'motion-reduce:transition-none motion-reduce:duration-0',
          'focus-visible:outline-2 focus-visible:outline-accent-1 focus-visible:outline-offset-2',
          i === activeDayIndex
            ? 'border-transparent bg-accent-1 text-accent-1-text'
            : 'border-line-soft bg-bg-1 text-fg-1',
        ]"
        @click="activeDayIndex = i"
      >
        <span class="text-[10px] uppercase opacity-80">{{ labels[i] }}</span>
        <span class="font-mono text-lg font-semibold">{{ d.getUTCDate() }}</span>
      </button>
    </div>

    <!-- Episode list for active day -->
    <div class="flex flex-col gap-2 px-4" data-testid="schedule-mobile-episodes">
      <article
        v-for="ep in activeDayEntries"
        :key="ep.id"
        class="flex items-center gap-3 rounded-md border border-line-soft bg-bg-1 p-3"
        data-testid="schedule-mobile-episode"
      >
        <img
          v-if="ep.coverUrl"
          :src="ep.coverUrl"
          class="w-10 h-14 rounded object-cover flex-shrink-0"
          alt=""
        />
        <div class="flex flex-col min-w-0 flex-1">
          <span class="text-sm font-medium truncate text-fg-1">{{ ep.title }}</span>
          <span v-if="ep.episode != null" class="text-xs text-fg-2">{{
            t('schedule.episode', { n: ep.episode })
          }}</span>
          <span v-if="ep.time" class="font-mono text-xs text-accent-1-text">{{
            t('schedule.airsAt', { time: ep.time })
          }}</span>
        </div>
      </article>
      <p
        v-if="activeDayEntries.length === 0"
        class="text-sm text-fg-2 py-6 text-center"
        data-testid="schedule-mobile-empty"
      >
        {{ t('schedule.noEntries') }}
      </p>
    </div>
  </div>

  <!-- Desktop layout (kept verbatim) -->
  <div v-else class="flex flex-col gap-4">
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

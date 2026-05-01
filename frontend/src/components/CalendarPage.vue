<script setup lang="ts">
import { computed } from 'vue'
import { useRoute, RouterLink } from 'vue-router'
import { useI18n } from 'vue-i18n'

defineOptions({ name: 'CalendarPage' })

const route = useRoute()
const { t } = useI18n()

const calendarId = computed(() => route.params.id as string)
const isSchedule = computed(() => route.name === 'calendar.schedule')
</script>

<template>
  <div class="flex flex-col gap-4 p-4">
    <h1 class="sr-only">
      {{ isSchedule ? t('calendar.tabs.schedule') : t('calendar.tabs.editor') }}
    </h1>
    <header class="flex items-center justify-between">
      <nav
        :aria-label="t('calendar.tabs.label')"
        data-testid="calendar-tabs"
        class="inline-flex rounded-md bg-bg-2 p-1 gap-1"
      >
        <RouterLink
          :to="`/calendar/${calendarId}`"
          :aria-current="!isSchedule ? 'page' : undefined"
          :class="[
            'h-8 px-3 text-sm rounded-sm transition-all duration-[var(--d-2)] inline-flex items-center focus-visible:outline-2 focus-visible:outline-accent-1 focus-visible:outline-offset-2',
            !isSchedule ? 'bg-bg-1 text-fg-1' : 'text-fg-2 hover:text-fg-1',
          ]"
        >
          {{ t('calendar.tabs.editor') }}
        </RouterLink>
        <RouterLink
          :to="`/calendar/${calendarId}/schedule`"
          :aria-current="isSchedule ? 'page' : undefined"
          :class="[
            'h-8 px-3 text-sm rounded-sm transition-all duration-[var(--d-2)] inline-flex items-center focus-visible:outline-2 focus-visible:outline-accent-1 focus-visible:outline-offset-2',
            isSchedule ? 'bg-bg-1 text-fg-1' : 'text-fg-2 hover:text-fg-1',
          ]"
        >
          {{ t('calendar.tabs.schedule') }}
        </RouterLink>
      </nav>
    </header>
    <router-view />
  </div>
</template>

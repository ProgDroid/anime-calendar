<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import type { ScheduleEntry } from '@/types/schedule';

defineProps<{ label: string; date: Date; entries: ScheduleEntry[] }>();
const { t } = useI18n();
</script>

<template>
  <div class="flex flex-col gap-2 min-w-0">
    <div class="flex flex-col gap-0.5 px-2 py-1">
      <span data-testid="day-label" class="text-xs uppercase tracking-wide text-fg-2">{{
        label
      }}</span>
      <span data-testid="day-date" class="text-lg font-medium">{{ date.getUTCDate() }}</span>
    </div>
    <div class="flex flex-col gap-2">
      <div
        v-for="entry in entries"
        :key="entry.id"
        data-testid="schedule-entry"
        class="flex gap-2 p-2 rounded-md bg-bg-2"
      >
        <img
          v-if="entry.coverUrl"
          :src="entry.coverUrl"
          class="w-10 h-14 rounded object-cover"
          alt=""
        />
        <div class="flex flex-col min-w-0">
          <span class="text-sm font-medium truncate">{{ entry.title }}</span>
          <span v-if="entry.episode != null" class="text-xs text-fg-2">{{
            t('schedule.episode', { n: entry.episode })
          }}</span>
          <span v-if="entry.time" class="text-xs text-fg-2">{{ entry.time }}</span>
        </div>
      </div>
      <div
        v-if="entries.length === 0"
        data-testid="day-empty"
        class="text-xs text-fg-2 px-2 py-4 text-center"
      >
        {{ t('schedule.noEntries') }}
      </div>
    </div>
  </div>
</template>

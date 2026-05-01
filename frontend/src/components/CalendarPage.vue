<script setup lang="ts">
import { computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import UiSegmented from '@/components/ui/UiSegmented.vue'

defineOptions({ name: 'CalendarPage' })

const route = useRoute()
const router = useRouter()
const { t } = useI18n()

const tab = computed<'editor' | 'schedule'>(() =>
  route.name === 'calendar.schedule' ? 'schedule' : 'editor',
)

function setTab(next: string) {
  if (next !== 'editor' && next !== 'schedule') return
  if (next === tab.value) return
  const id = route.params.id
  router.push(next === 'editor' ? `/calendar/${id}` : `/calendar/${id}/schedule`)
}
</script>

<template>
  <div class="flex flex-col gap-4 p-4">
    <header class="flex items-center justify-between">
      <UiSegmented
        :model-value="tab"
        :options="[
          { value: 'editor', label: t('calendar.tabs.editor') },
          { value: 'schedule', label: t('calendar.tabs.schedule') },
        ]"
        data-testid="calendar-tabs"
        @update:model-value="setTab"
      />
    </header>
    <router-view />
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import type { Item } from '@/types/item'
import MediaItemCard from '@/components/shared/MediaItemCard.vue'
import UiButton from '@/components/ui/UiButton.vue'
import { FREE_SHOW_CAP, CAP_WARN_THRESHOLD } from '@/stores/usageStore'

defineOptions({ name: 'CalendarItemsList' })

const { t } = useI18n()

const props = defineProps<{
  items: Item[]
  calendarLanguage: 'english' | 'romaji' | 'native'
  /**
   * Total distinct shows tracked across all calendars (free-tier counter
   * source). When undefined, the chip and banner don't render — pass
   * undefined for Pro users.
   */
  showCount?: number | null
}>()

const emit = defineEmits<{
  remove: [id: number]
  clear: []
}>()

const hasShowCount = computed(
  () => typeof props.showCount === 'number' && props.showCount >= 0,
)
const capReached = computed(
  () => hasShowCount.value && (props.showCount ?? 0) >= FREE_SHOW_CAP,
)
const showWarningBanner = computed(
  () =>
    hasShowCount.value &&
    !capReached.value &&
    (props.showCount ?? 0) / FREE_SHOW_CAP >= CAP_WARN_THRESHOLD,
)
const bannerDismissed = ref(false)
function dismissBanner() {
  bannerDismissed.value = true
}

const getTitle = (item: Item): string => {
  switch (props.calendarLanguage) {
    case 'english': return item.title.english.length > 0 ? item.title.english : item.title.romaji
    case 'romaji': return item.title.romaji
    case 'native': return item.title.native
    default: return item.title.romaji
  }
}
</script>

<template>
  <div class="flex flex-col gap-4 flex-1 min-h-0">
    <div class="flex items-center gap-3 flex-wrap">
      <h3 class="font-semibold text-fg-1">{{ t('calendar.itemsInCalendar') }}</h3>
      <span
        v-if="hasShowCount"
        data-testid="show-counter-chip"
        class="inline-flex items-center rounded-full bg-bg-1 border border-line text-fg-2 px-2.5 py-0.5 text-xs"
        :class="{ 'border-warning text-warning': capReached }"
      >
        {{ t('calendar_limits.counter', { count: props.showCount, max: FREE_SHOW_CAP }) }}
      </span>
    </div>

    <div
      v-if="showWarningBanner && !bannerDismissed"
      data-testid="show-warning-banner"
      role="status"
      class="rounded-md border border-warning/40 bg-warning/10 text-warning px-3 py-2 text-sm flex items-start gap-2"
    >
      <span class="flex-1">
        {{ t('calendar_limits.warningBanner', { count: props.showCount, max: FREE_SHOW_CAP }) }}
      </span>
      <button
        type="button"
        data-testid="show-warning-banner-dismiss"
        class="text-warning/80 hover:text-warning text-xs underline shrink-0"
        @click="dismissBanner"
      >
        {{ t('calendar_limits.dismiss') }}
      </button>
    </div>

    <div class="overflow-y-auto max-h-[400px] flex-1 min-h-0 p-2 border border-line rounded-md space-y-2 bg-bg-1">
      <MediaItemCard
        v-for="item in items"
        :key="item.id"
        :item="item"
        :display-title="getTitle(item)"
        :is-selected="false"
        :is-in-calendar="false"
      >
        <UiButton
          :data-testid="`remove-item-${item.id}`"
          variant="danger"
          size="sm"
          class="mt-2 w-full"
          @click.stop="emit('remove', item.id)"
        >
          {{ t('calendar.remove') }}
        </UiButton>
      </MediaItemCard>
    </div>

    <UiButton
      data-testid="clear-btn"
      variant="secondary"
      class="w-full"
      :disabled="items.length === 0"
      @click="emit('clear')"
    >
      {{ t('calendar.clear') }}
    </UiButton>
  </div>
</template>

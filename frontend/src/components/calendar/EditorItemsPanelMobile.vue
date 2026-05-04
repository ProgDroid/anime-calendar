<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import type { Item } from '@/types/item'
import MediaItemCard from '@/components/shared/MediaItemCard.vue'
import RecommendationsSection from './RecommendationsSection.vue'
import UiButton from '@/components/ui/UiButton.vue'
import { FREE_SHOW_CAP, CAP_WARN_THRESHOLD } from '@/stores/usageStore'

defineOptions({ name: 'EditorItemsPanelMobile' })

const { t } = useI18n()

const props = defineProps<{
  items: Item[]
  calendarLanguage: 'english' | 'romaji' | 'native'
  recommendations?: Item[]
  /**
   * Total distinct shows this user tracks across all calendars (for the
   * counter chip + 80% banner). Optional — when undefined the chip and
   * banner don't render. Pro users should be passed undefined or `null`.
   */
  showCount?: number | null
}>()

const emit = defineEmits<{
  remove: [id: number]
  clear: []
  'add-recommendation': [item: Item]
}>()

const recs = computed(() => props.recommendations ?? [])

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

// Per-session dismissal — clearing the banner sticks until the tab closes.
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
  <div class="flex flex-col gap-3 px-4 pb-tab-bar">
    <!-- Free-tier counter chip -->
    <div
      v-if="hasShowCount"
      class="flex items-center gap-2"
      data-testid="show-counter-row"
    >
      <span
        data-testid="show-counter-chip"
        class="inline-flex items-center rounded-full bg-bg-1 border border-line text-fg-2 px-2.5 py-0.5 text-xs"
        :class="{ 'border-warning text-warning': capReached }"
      >
        {{ t('calendar_limits.counter', { count: props.showCount, max: FREE_SHOW_CAP }) }}
      </span>
    </div>

    <!-- 80% soft warning banner — dismissible per session -->
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

    <div
      v-if="items.length === 0"
      class="flex flex-col items-center justify-center py-16 text-center gap-2"
      data-testid="items-empty-state"
    >
      <p class="text-fg-2 text-sm">{{ t('calendar.noItemsSelected') }}</p>
    </div>

    <MediaItemCard
      v-for="item in items"
      :key="item.id"
      :data-testid="`items-panel-card-${item.id}`"
      :item="item"
      :display-title="getTitle(item)"
      :is-selected="false"
      :is-in-calendar="true"
      :compact="true"
    >
      <UiButton
        :data-testid="`remove-item-mobile-${item.id}`"
        variant="danger"
        size="sm"
        class="mt-2 w-full"
        @click.stop="emit('remove', item.id)"
      >
        {{ t('calendar.remove') }}
      </UiButton>
    </MediaItemCard>

    <UiButton
      v-if="items.length > 0"
      data-testid="items-panel-clear-btn"
      variant="secondary"
      class="w-full mt-2"
      @click="emit('clear')"
    >
      {{ t('calendar.clear') }}
    </UiButton>

    <RecommendationsSection
      v-if="items.length > 0 && recs.length > 0"
      data-testid="items-panel-recommendations"
      :recommendations="recs"
      :calendar-has-items="items.length > 0"
      :calendar-language="calendarLanguage"
      @add="(item) => emit('add-recommendation', item)"
    />
  </div>
</template>

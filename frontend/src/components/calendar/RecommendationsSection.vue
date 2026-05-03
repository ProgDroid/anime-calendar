<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { Item } from '@/types/item'
import MediaItemCard from '@/components/shared/MediaItemCard.vue'
import UiButton from '@/components/ui/UiButton.vue'

const { t } = useI18n()

const props = defineProps<{
  recommendations: Item[]
  calendarHasItems: boolean
  calendarLanguage: 'english' | 'romaji' | 'native'
}>()

const emit = defineEmits<{
  add: [item: Item]
}>()

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
  <div class="bg-bg-1 border border-line rounded-lg shadow-sm mt-4">
    <div class="p-4 flex flex-col gap-3">
      <h2 class="font-semibold text-fg-1">{{ t('calendar.recommendedItems') }}</h2>

      <div v-if="recommendations.length > 0" class="grid grid-cols-1 sm:grid-cols-2 gap-2">
        <div
          v-for="item in recommendations"
          :key="item.id"
        >
          <MediaItemCard
            :item="item"
            :display-title="getTitle(item)"
            :is-selected="false"
            :is-in-calendar="false"
            compact
          >
            <UiButton
              :data-testid="`add-reco-${item.id}`"
              variant="primary"
              size="sm"
              class="mt-2 w-full"
              @click.stop="emit('add', item)"
            >
              {{ t('calendar.add') }}
            </UiButton>
          </MediaItemCard>
        </div>
      </div>

      <div v-else-if="calendarHasItems" class="flex justify-center items-center py-4">
        <p class="text-fg-2">{{ t('calendar.noRecommendations') }}</p>
      </div>

      <div v-else class="flex justify-center items-center py-4">
        <p class="text-fg-2">{{ t('calendar.addItemsToSeeRecommendations') }}</p>
      </div>
    </div>
  </div>
</template>

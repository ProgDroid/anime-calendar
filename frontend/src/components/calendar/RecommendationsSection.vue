<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { Item } from '@/types/item'
import MediaItemCard from '@/components/shared/MediaItemCard.vue'

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
  <div class="card bg-base-100 shadow-md mt-4">
    <div class="card-body">
      <h2 class="card-title">{{ t('calendar.recommendedItems') }}</h2>

      <div v-if="recommendations.length > 0" class="grid grid-cols-5 gap-2">
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
            <button
              :data-testid="`add-reco-${item.id}`"
              class="btn btn-primary btn-xs mt-2 w-full"
              @click.stop="emit('add', item)"
            >
              {{ t('calendar.add') }}
            </button>
          </MediaItemCard>
        </div>
      </div>

      <div v-else-if="calendarHasItems" class="flex justify-center items-center py-4">
        <p>{{ t('calendar.noRecommendations') }}</p>
      </div>

      <div v-else class="flex justify-center items-center py-4">
        <p>{{ t('calendar.addItemsToSeeRecommendations') }}</p>
      </div>
    </div>
  </div>
</template>

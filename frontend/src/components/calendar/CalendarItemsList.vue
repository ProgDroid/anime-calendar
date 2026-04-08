<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { Item } from '@/types/item'
import MediaItemCard from '@/components/shared/MediaItemCard.vue'

const { t } = useI18n()

const props = defineProps<{
  items: Item[]
  calendarLanguage: 'english' | 'romaji' | 'native'
}>()

const emit = defineEmits<{
  remove: [id: number]
  clear: []
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
  <div class="flex flex-col gap-4 flex-1 min-h-0">
    <h3 class="font-bold">{{ t('calendar.itemsInCalendar') }}</h3>
    <div class="overflow-y-auto max-h-[400px] flex-1 min-h-0 p-2 border rounded space-y-1">
      <MediaItemCard
        v-for="item in items"
        :key="item.id"
        :item="item"
        :display-title="getTitle(item)"
        :is-selected="false"
        :is-in-calendar="false"
      >
        <button
          :data-testid="`remove-item-${item.id}`"
          class="btn btn-sm btn-error mt-2 w-full"
          @click.stop="emit('remove', item.id)"
        >
          {{ t('calendar.remove') }}
        </button>
      </MediaItemCard>
    </div>

    <button
      data-testid="clear-btn"
      class="btn btn-warning w-full"
      :disabled="items.length === 0"
      @click="emit('clear')"
    >
      {{ t('calendar.clear') }}
    </button>
  </div>
</template>

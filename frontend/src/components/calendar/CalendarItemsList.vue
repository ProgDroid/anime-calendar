<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { Item } from '@/types/item'
import MediaItemCard from '@/components/shared/MediaItemCard.vue'
import UiButton from '@/components/ui/UiButton.vue'

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
    <h3 class="font-semibold text-fg-1">{{ t('calendar.itemsInCalendar') }}</h3>
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

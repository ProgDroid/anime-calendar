<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { Item } from '@/types/item'
import MediaItemCard from '@/components/shared/MediaItemCard.vue'
import UiButton from '@/components/ui/UiButton.vue'

defineOptions({ name: 'EditorItemsPanelMobile' })

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
  <div class="flex flex-col gap-3 px-4 pb-tab-bar">
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
  </div>
</template>

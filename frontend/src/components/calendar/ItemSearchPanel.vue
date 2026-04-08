<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import type { Item } from '@/types/item'
import MediaItemCard from '@/components/shared/MediaItemCard.vue'

const { t } = useI18n()

const props = defineProps<{
  fetchedItems: Item[]
  selectedItems: number[]
  itemsInCalendar: Item[]
  loading: boolean
  calendarLanguage: 'english' | 'romaji' | 'native'
  searchError?: string | null
}>()

const emit = defineEmits<{
  search: [{ name: string; mediaType: '' | 'ANIME' | 'MANGA' }]
  'toggle-selection': [id: number]
  'add-selected': []
}>()

const nameInput = ref('')
const mediaType = ref<'' | 'ANIME' | 'MANGA'>('')

const getTitle = (item: Item): string => {
  switch (props.calendarLanguage) {
    case 'english': return item.title.english.length > 0 ? item.title.english : item.title.romaji
    case 'romaji': return item.title.romaji
    case 'native': return item.title.native
    default: return item.title.romaji
  }
}

const handleSearch = () => {
  emit('search', { name: nameInput.value, mediaType: mediaType.value })
}
</script>

<template>
  <div class="flex flex-col gap-4 flex-1 min-h-0">
    <!-- Search form -->
    <div class="form-control">
      <label class="label">
        <span class="label-text">{{ t('calendar.itemName') }}:</span>
      </label>
      <input
        v-model="nameInput"
        type="text"
        :placeholder="t('calendar.itemNamePlaceholder')"
        class="input input-bordered mt-2 w-full"
        @keyup.enter="handleSearch"
      />
    </div>

    <div class="form-control">
      <label class="label mb-2">
        <span class="label-text">{{ t('calendar.mediaType') }}:</span>
      </label>
      <div class="flex gap-4 flex-wrap">
        <label class="label cursor-pointer gap-2">
          <input v-model="mediaType" type="radio" value="" class="radio radio-primary" />
          <span class="label-text">{{ t('calendar.mediaTypeAny') }}</span>
        </label>
        <label class="label cursor-pointer gap-2">
          <input v-model="mediaType" type="radio" value="ANIME" class="radio radio-primary" />
          <span class="label-text">{{ t('calendar.mediaTypeAnime') }}</span>
        </label>
        <label class="label cursor-pointer gap-2">
          <input v-model="mediaType" type="radio" value="MANGA" class="radio radio-primary" />
          <span class="label-text">{{ t('calendar.mediaTypeManga') }}</span>
        </label>
      </div>
    </div>

    <button
      type="button"
      :disabled="loading"
      class="btn btn-primary w-full"
      @click="handleSearch"
    >
      {{ loading ? t('calendar.fetchingItems') : t('calendar.fetchItems') }}
    </button>

    <div v-if="searchError" class="alert alert-error">{{ searchError }}</div>

    <!-- Results list -->
    <h3 class="font-bold">{{ t('calendar.fetchedItems') }}</h3>
    <div class="overflow-y-auto max-h-[400px] flex-1 min-h-0 p-2 border rounded space-y-1">
      <MediaItemCard
        v-for="item in fetchedItems"
        :key="item.id"
        :data-testid="`item-card-${item.id}`"
        :item="item"
        :display-title="getTitle(item)"
        :is-selected="selectedItems.includes(item.id)"
        :is-in-calendar="itemsInCalendar.some(c => c.id === item.id)"
        @click="!itemsInCalendar.some(c => c.id === item.id) && emit('toggle-selection', item.id)"
      />
    </div>

    <button
      data-testid="add-selected-btn"
      :disabled="selectedItems.length === 0"
      class="btn btn-primary w-full"
      @click="emit('add-selected')"
    >
      {{ t('calendar.addSelectedToCalendar') }}
    </button>
  </div>
</template>

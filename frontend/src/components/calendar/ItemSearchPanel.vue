<script setup lang="ts">
import { ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import type { Item } from '@/types/item'
import { useUpgradeInterrupt } from '@/composables/useUpgradeInterrupt'
import { FREE_SHOW_CAP } from '@/stores/usageStore'
import MediaItemCard from '@/components/shared/MediaItemCard.vue'
import UiButton from '@/components/ui/UiButton.vue'
import UiInput from '@/components/ui/UiInput.vue'

const { t } = useI18n()

const props = defineProps<{
  fetchedItems: Item[]
  selectedItems: number[]
  itemsInCalendar: Item[]
  loading: boolean
  calendarLanguage: 'english' | 'romaji' | 'native'
  searchError?: string | null
  /**
   * Free-tier distinct-show count, used for cap-aware add gating. Pass
   * undefined for Pro users — they bypass the gate.
   */
  showCount?: number | null
}>()

const emit = defineEmits<{
  search: [{ name: string; mediaType: '' | 'ANIME' | 'MANGA' }]
  'toggle-selection': [id: number]
  'add-selected': []
}>()

const { openUpgradeModal } = useUpgradeInterrupt()
const capReached = computed(
  () =>
    typeof props.showCount === 'number' && props.showCount >= FREE_SHOW_CAP,
)
function isAlreadyTracked(itemId: number): boolean {
  return props.itemsInCalendar.some((c) => c.id === itemId)
}
function handleCardClick(item: Item) {
  if (isAlreadyTracked(item.id)) return
  if (capReached.value) {
    openUpgradeModal('cap_shows')
    return
  }
  emit('toggle-selection', item.id)
}
const addSelectedDisabled = computed(
  () => props.selectedItems.length === 0 || capReached.value,
)
function handleAddSelected() {
  if (capReached.value) {
    openUpgradeModal('cap_shows')
    return
  }
  emit('add-selected')
}

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
    <UiInput
      v-model="nameInput"
      :label="t('calendar.itemName')"
      :placeholder="t('calendar.itemNamePlaceholder')"
      @keyup.enter="handleSearch"
    />

    <div class="flex flex-col gap-2">
      <label class="text-sm text-fg-2">{{ t('calendar.mediaType') }}</label>
      <div class="flex gap-4 flex-wrap">
        <label class="inline-flex items-center gap-2 cursor-pointer text-fg-1">
          <input v-model="mediaType" type="radio" value="" class="accent-accent-1" />
          <span class="text-sm">{{ t('calendar.mediaTypeAny') }}</span>
        </label>
        <label class="inline-flex items-center gap-2 cursor-pointer text-fg-1">
          <input v-model="mediaType" type="radio" value="ANIME" class="accent-accent-1" />
          <span class="text-sm">{{ t('calendar.mediaTypeAnime') }}</span>
        </label>
        <label class="inline-flex items-center gap-2 cursor-pointer text-fg-1">
          <input v-model="mediaType" type="radio" value="MANGA" class="accent-accent-1" />
          <span class="text-sm">{{ t('calendar.mediaTypeManga') }}</span>
        </label>
      </div>
    </div>

    <UiButton
      type="button"
      variant="primary"
      :disabled="loading"
      class="w-full"
      @click="handleSearch"
    >
      {{ loading ? t('calendar.fetchingItems') : t('calendar.fetchItems') }}
    </UiButton>

    <div v-if="searchError" class="text-sm text-danger-text bg-danger/10 border border-danger/30 rounded-md px-3 py-2">{{ searchError }}</div>

    <!-- Results list -->
    <h3 class="font-semibold text-fg-1">{{ t('calendar.fetchedItems') }}</h3>
    <div class="overflow-y-auto max-h-[400px] flex-1 min-h-0 p-2 border border-line rounded-md space-y-2 bg-bg-1">
      <MediaItemCard
        v-for="item in fetchedItems"
        :key="item.id"
        :data-testid="`item-card-${item.id}`"
        :item="item"
        :display-title="getTitle(item)"
        :is-selected="selectedItems.includes(item.id)"
        :is-in-calendar="itemsInCalendar.some(c => c.id === item.id)"
        :class="{ 'opacity-60': capReached && !isAlreadyTracked(item.id) }"
        @click="handleCardClick(item)"
      />
    </div>

    <UiButton
      data-testid="add-selected-btn"
      variant="primary"
      :disabled="addSelectedDisabled"
      class="w-full"
      @click="handleAddSelected"
    >
      {{ t('calendar.addSelectedToCalendar') }}
    </UiButton>
  </div>
</template>

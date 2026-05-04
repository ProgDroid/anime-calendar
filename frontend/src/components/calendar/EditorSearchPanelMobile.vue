<script setup lang="ts">
import { ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import type { Item } from '@/types/item'
import { useEditorSelectionStore } from '@/stores/editorSelection'
import { useCalendarSearch } from '@/composables/useCalendarSearch'
import { useUpgradeInterrupt } from '@/composables/useUpgradeInterrupt'
import { FREE_SHOW_CAP } from '@/stores/usageStore'
import MediaItemCard from '@/components/shared/MediaItemCard.vue'
import RecommendationsSection from './RecommendationsSection.vue'
import UiButton from '@/components/ui/UiButton.vue'
import UiInput from '@/components/ui/UiInput.vue'

defineOptions({ name: 'EditorSearchPanelMobile' })

const { t } = useI18n()
const selection = useEditorSelectionStore()

const { fetchedItems, loading, searchError, handleSearch } = useCalendarSearch()

const props = defineProps<{
  itemsInCalendar: Item[]
  calendarLanguage: 'english' | 'romaji' | 'native'
  recommendations?: Item[]
  /**
   * Total distinct shows the user tracks across all calendars. When at or
   * over `FREE_SHOW_CAP`, adds of new (untracked) media are gated behind
   * the upgrade modal. Already-tracked items remain addable. Pass
   * undefined for Pro users — they bypass the gate entirely.
   */
  showCount?: number | null
}>()

const emit = defineEmits<{
  'add-selected': [items: Item[]]
  'add-recommendation': [item: Item]
}>()

const nameInput = ref('')
const mediaType = ref<'' | 'ANIME' | 'MANGA'>('')
const searchInputRef = ref<{ focus: () => void } | null>(null)

const hasResults = computed(() => fetchedItems.value.length > 0)
const selectionCount = computed(() => selection.selectedMediaIds.size)
const hasRecommendations = computed(() => (props.recommendations?.length ?? 0) > 0)

const { openUpgradeModal } = useUpgradeInterrupt()
const capReached = computed(
  () =>
    typeof props.showCount === 'number' && props.showCount >= FREE_SHOW_CAP,
)
function isAlreadyTracked(itemId: number): boolean {
  return props.itemsInCalendar.some((c) => c.id === itemId)
}
/**
 * Free user at cap clicking on a new (untracked) item — block selection and
 * route to the upgrade modal. Already-tracked items remain selectable.
 */
function handleCardClick(item: Item) {
  if (isAlreadyTracked(item.id)) return
  if (capReached.value) {
    openUpgradeModal('cap_shows')
    return
  }
  selection.toggle(item.id)
}

const submitSelected = () => {
  const items = fetchedItems.value.filter(
    item =>
      selection.has(item.id) &&
      !props.itemsInCalendar.some(c => c.id === item.id),
  )
  if (items.length === 0) return
  emit('add-selected', items)
}

const getTitle = (item: Item): string => {
  switch (props.calendarLanguage) {
    case 'english': return item.title.english.length > 0 ? item.title.english : item.title.romaji
    case 'romaji': return item.title.romaji
    case 'native': return item.title.native
    default: return item.title.romaji
  }
}

const doSearch = () => {
  handleSearch({ name: nameInput.value, mediaType: mediaType.value })
}

// Expose a focus wrapper so the parent can focus the search input (for FAB jump)
defineExpose({
  focus: () => searchInputRef.value?.focus(),
})
</script>

<template>
  <div class="relative flex flex-col gap-4 px-4 pb-tab-bar">
    <!-- Search input -->
    <div class="flex gap-2">
      <div class="flex-1">
        <UiInput
          ref="searchInputRef"
          v-model="nameInput"
          :label="t('calendar.itemName')"
          :placeholder="t('calendar.itemNamePlaceholder')"
          data-testid="search-input-mobile"
          @keyup.enter="doSearch"
        />
      </div>
    </div>

    <!-- Media type filters -->
    <div class="flex gap-3 flex-wrap">
      <label class="inline-flex items-center gap-1.5 cursor-pointer text-fg-1">
        <input v-model="mediaType" type="radio" value="" class="accent-accent-1" />
        <span class="text-sm">{{ t('calendar.mediaTypeAny') }}</span>
      </label>
      <label class="inline-flex items-center gap-1.5 cursor-pointer text-fg-1">
        <input v-model="mediaType" type="radio" value="ANIME" class="accent-accent-1" />
        <span class="text-sm">{{ t('calendar.mediaTypeAnime') }}</span>
      </label>
      <label class="inline-flex items-center gap-1.5 cursor-pointer text-fg-1">
        <input v-model="mediaType" type="radio" value="MANGA" class="accent-accent-1" />
        <span class="text-sm">{{ t('calendar.mediaTypeManga') }}</span>
      </label>
    </div>

    <!-- Search button -->
    <UiButton
      type="button"
      variant="primary"
      :disabled="loading"
      class="w-full"
      data-testid="search-btn-mobile"
      @click="doSearch"
    >
      {{ loading ? t('calendar.fetchingItems') : t('calendar.fetchItems') }}
    </UiButton>

    <!-- Error -->
    <div
      v-if="searchError"
      class="text-sm text-danger-text bg-danger/10 border border-danger/30 rounded-md px-3 py-2"
    >
      {{ searchError }}
    </div>

    <!-- Empty state (no search yet) -->
    <div
      v-if="!hasResults && !loading && !searchError"
      class="flex flex-col items-center justify-center py-6 text-center gap-2"
      data-testid="search-empty-state"
    >
      <p class="font-semibold text-fg-1">{{ t('mobile.editor.emptySearch.title') }}</p>
      <p class="text-sm text-fg-2">{{ t('mobile.editor.emptySearch.subtitle') }}</p>
    </div>

    <!-- Recommendations as a discovery hint when there are no search results -->
    <RecommendationsSection
      v-if="!hasResults && hasRecommendations"
      :recommendations="props.recommendations ?? []"
      :calendar-has-items="props.itemsInCalendar.length > 0"
      :calendar-language="calendarLanguage"
      @add="(item) => emit('add-recommendation', item)"
    />

    <!-- Results -->
    <div v-if="hasResults" class="flex flex-col gap-2" data-testid="search-results">
      <p class="text-sm text-fg-2">
        {{ t('mobile.editor.results.resultsCount', { count: fetchedItems.length }) }}
        <span class="ml-1">— {{ t('mobile.editor.results.tapToSelect') }}</span>
      </p>

      <MediaItemCard
        v-for="item in fetchedItems"
        :key="item.id"
        :data-testid="`search-card-${item.id}`"
        :item="item"
        :display-title="getTitle(item)"
        :is-selected="selection.has(item.id)"
        :is-in-calendar="itemsInCalendar.some(c => c.id === item.id)"
        :compact="true"
        :class="{ 'opacity-60': capReached && !isAlreadyTracked(item.id) }"
        @click="handleCardClick(item)"
      />
    </div>

    <!-- Sticky batch-add CTA — shown when selection is non-empty -->
    <div
      v-if="selectionCount > 0"
      class="sticky bottom-0 left-0 right-0 z-10 flex gap-2 bg-bg-0/90 px-0 py-3 backdrop-blur-sm motion-reduce:transition-none motion-reduce:duration-0"
      data-testid="batch-add-bar"
    >
      <UiButton
        variant="secondary"
        class="shrink-0"
        data-testid="batch-clear-btn"
        @click="selection.clear()"
      >
        {{ t('mobile.editor.batchClear') }}
      </UiButton>
      <UiButton
        variant="primary"
        class="flex-1"
        data-testid="batch-add-btn"
        @click="submitSelected"
      >
        {{ t('mobile.editor.batchAdd', { count: selectionCount }) }}
      </UiButton>
    </div>
  </div>
</template>

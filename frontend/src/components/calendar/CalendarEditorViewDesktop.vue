<template>
  <div class="min-h-[calc(100vh-6.1rem)] bg-bg-0 p-4">
    <div class="grid grid-cols-1 lg:grid-cols-[3fr_2fr] gap-6 max-w-6xl mx-auto">
      <!-- Left: Calendar settings + items list (3fr) -->
      <div data-testid="editor-items-list" class="bg-bg-1 border border-line rounded-lg shadow-sm">
        <div class="p-4 flex flex-col gap-4">
          <div class="flex items-center justify-between gap-2">
            <h2 class="font-semibold text-fg-1 text-lg">{{ $t('calendar.edit') }}</h2>
            <PresenceChip v-if="isExistingCalendar && viewers.length > 0" :viewers="viewers" />
          </div>
          <div
            v-if="showCollisionBanner"
            data-testid="collision-banner"
            class="rounded-md bg-bg-2 border border-line px-3 py-2 text-sm text-fg-2 flex items-center justify-between gap-2"
          >
            <span>{{ $t('sharing.collisionWarning') }}</span>
            <button
              class="text-accent-1 font-medium hover:underline shrink-0"
              @click="reloadPage"
            >
              {{ $t('sharing.reloadButton') }}
            </button>
          </div>
          <CalendarSettingsForm
            :name="calendarName"
            :language="calendarLanguage"
            :event-style="calendarEventStyle"
            :loading="loading"
            :can-submit="itemsInCalendar.length > 0"
            :error="calendarError"
            :is-owner="isOwner"
            :calendar-id="currentCalendar?.id"
            :is-paid="!isFreeTier"
            @update:name="calendarName = $event"
            @update:language="calendarLanguage = $event"
            @update:event-style="calendarEventStyle = $event"
            @submit="submitCalendar"
          />
          <CalendarItemsList
            :items="itemsInCalendar"
            :calendar-language="calendarLanguage"
            :show-count="editorShowCount"
            @remove="removeItemFromCalendar"
            @clear="clearCalendar"
          />
        </div>
      </div>

      <!-- Right: Search panel + recommendations (2fr) -->
      <div class="flex flex-col gap-6 min-w-0">
        <div data-testid="editor-search-panel" class="bg-bg-1 border border-line rounded-lg shadow-sm">
          <div class="p-4 flex flex-col gap-4">
            <h2 class="font-semibold text-fg-1 text-lg">{{ $t('calendar.search') }}</h2>
            <ItemSearchPanel
              :fetched-items="fetchedItems"
              :selected-items="selectedItems"
              :items-in-calendar="itemsInCalendar"
              :loading="loading"
              :calendar-language="calendarLanguage"
              :search-error="searchError"
              :show-count="editorShowCount"
              @search="handleSearch"
              @toggle-selection="toggleItemSelection"
              @add-selected="addItemToCalendar"
            />
          </div>
        </div>

        <div data-testid="editor-recommendations" class="hidden lg:block">
          <RecommendationsSection
            :recommendations="recommendations"
            :calendar-has-items="itemsInCalendar.length > 0"
            :calendar-language="calendarLanguage"
            @add="addItemToCalendarSingle"
          />
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import type { Item } from '@/types/item'
import { useCalendarSearch } from '@/composables/useCalendarSearch'
import { useCalendarEditor } from '@/composables/useCalendarEditor'
import CalendarSettingsForm from '@/components/calendar/CalendarSettingsForm.vue'
import CalendarItemsList from '@/components/calendar/CalendarItemsList.vue'
import ItemSearchPanel from '@/components/calendar/ItemSearchPanel.vue'
import RecommendationsSection from '@/components/calendar/RecommendationsSection.vue'
import PresenceChip from '@/components/shared/PresenceChip.vue'

defineOptions({ name: 'CalendarEditorViewDesktop' })

const {
  isExistingCalendar,
  calendarName,
  calendarLanguage,
  calendarEventStyle,
  itemsInCalendar,
  submitLoading,
  calendarError,
  currentCalendar,
  isFreeTier,
  isOwner,
  editorShowCount,
  addItems,
  removeItemFromCalendar,
  clearCalendar,
  submitCalendar,
  viewers,
  showCollisionBanner,
  reloadPage,
  recommendations,
} = useCalendarEditor()

const {
  fetchedItems,
  selectedItems,
  loading: searchLoading,
  searchError,
  handleSearch,
  toggleItemSelection,
} = useCalendarSearch()

const loading = computed(() => searchLoading.value || submitLoading.value)

const addItemToCalendar = () => {
  const toAdd = fetchedItems.value.filter(item => selectedItems.value.includes(item.id))
  addItems(toAdd)
  selectedItems.value = []
}

const addItemToCalendarSingle = (item: Item) => addItems([item])
</script>

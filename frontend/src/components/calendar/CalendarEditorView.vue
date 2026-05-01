<template>
  <div class="min-h-[calc(100vh-6.1rem)] bg-bg-0 p-4">
    <div class="grid grid-cols-1 lg:grid-cols-[3fr_2fr] gap-6 max-w-6xl mx-auto">
      <!-- Left: Calendar settings + items list (3fr) -->
      <div data-testid="editor-items-list" class="bg-bg-1 border border-line rounded-lg shadow-sm">
        <div class="p-4 flex flex-col gap-4">
          <h2 class="font-semibold text-fg-1 text-lg">{{ $t('calendar.edit') }}</h2>
          <CalendarSettingsForm
            :name="calendarName"
            :language="calendarLanguage"
            :loading="loading"
            :can-submit="itemsInCalendar.length > 0"
            :error="calendarError"
            @update:name="calendarName = $event"
            @update:language="calendarLanguage = $event"
            @submit="submitCalendar"
          />
          <CalendarItemsList
            :items="itemsInCalendar"
            :calendar-language="calendarLanguage"
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
import { ref, computed, onMounted, watch, onBeforeMount, onBeforeUnmount } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import type { Item } from '@/types/item'
import type { Calendar } from '@/types/calendar'
import api from '@/config/api'
import { toastService } from '@/services/toastService'
import { useUserSettingsStore } from '@/stores/userSettingsStore'
import { useCalendarSearch } from '@/composables/useCalendarSearch'
import { useRecommendations } from '@/composables/useRecommendations'
import CalendarSettingsForm from '@/components/calendar/CalendarSettingsForm.vue'
import CalendarItemsList from '@/components/calendar/CalendarItemsList.vue'
import ItemSearchPanel from '@/components/calendar/ItemSearchPanel.vue'
import RecommendationsSection from '@/components/calendar/RecommendationsSection.vue'

defineOptions({ name: 'CalendarEditorView' })

const { t } = useI18n()
const router = useRouter()
const route = useRoute()
const userSettingsStore = useUserSettingsStore()

const { fetchedItems, selectedItems, loading: searchLoading, searchError, handleSearch, toggleItemSelection } = useCalendarSearch()
const { recommendations, calculateRecommendations } = useRecommendations()

const calendarName = ref('')
const calendarLanguage = ref<'english' | 'romaji' | 'native'>('english')
const itemsInCalendar = ref<Item[]>([])
const submitLoading = ref(false)
const calendarError = ref<string | null>(null)
const currentCalendar = ref<Calendar | null>(null)

const loading = computed(() => searchLoading.value || submitLoading.value)

const addItemToCalendar = () => {
  const newItems = fetchedItems.value.filter(
    item => selectedItems.value.includes(item.id) &&
    !itemsInCalendar.value.some(c => c.id === item.id)
  )
  itemsInCalendar.value.push(...newItems)
  selectedItems.value = []
  if (itemsInCalendar.value.length > 0) calculateRecommendations(itemsInCalendar.value)
}

const addItemToCalendarSingle = (item: Item) => {
  if (!itemsInCalendar.value.some(c => c.id === item.id)) {
    itemsInCalendar.value.push(item)
    calculateRecommendations(itemsInCalendar.value)
  }
}

const removeItemFromCalendar = (id: number) => {
  itemsInCalendar.value = itemsInCalendar.value.filter(item => item.id !== id)
  calculateRecommendations(itemsInCalendar.value)
}

const clearCalendar = () => {
  itemsInCalendar.value = []
  calculateRecommendations([])
}

const submitCalendar = async () => {
  const MAX_NAME_LENGTH = 100
  if (!calendarName.value) {
    calendarError.value = t('calendar.enterCalendarName')
    return
  }
  if (calendarName.value.length > MAX_NAME_LENGTH) {
    calendarError.value = t('calendar.nameMaxLength', { max_length: MAX_NAME_LENGTH })
    return
  }
  if (itemsInCalendar.value.length === 0) {
    calendarError.value = t('calendar.noItemsSelected')
    return
  }

  submitLoading.value = true
  calendarError.value = null
  try {
    let response
    if (currentCalendar.value) {
      const calendar: Omit<Calendar, 'created_at' | 'updated_at'> = {
        id: currentCalendar.value.id,
        name: calendarName.value,
        language: calendarLanguage.value,
        items: itemsInCalendar.value
      }
      response = await api.put('/calendar', calendar)
    } else {
      const calendar: Omit<Calendar, 'id' | 'created_at' | 'updated_at'> = {
        name: calendarName.value,
        language: calendarLanguage.value,
        items: itemsInCalendar.value
      }
      response = await api.put('/calendar', calendar)
    }
    toastService.success(t('calendar.updateSuccess', { name: response.data.name }))
    calendarName.value = ''
    itemsInCalendar.value = []
    router.push('/my-calendars')
  } catch {
    calendarError.value = t('calendar.updateFailed')
  } finally {
    submitLoading.value = false
  }
}

onMounted(() => {
  try {
    const saved = sessionStorage.getItem('calendarPageState')
    if (saved) {
      const state = JSON.parse(saved)
      if (state.fetchedItems !== undefined) fetchedItems.value = state.fetchedItems
      if (state.selectedItems !== undefined) selectedItems.value = state.selectedItems
      if (state.itemsInCalendar !== undefined) itemsInCalendar.value = state.itemsInCalendar
      if (state.calendarName !== undefined) calendarName.value = state.calendarName
      if (state.calendarLanguage !== undefined) calendarLanguage.value = state.calendarLanguage
    }
  } catch { /* ignore */ }
})

let sessionStorageTimer: ReturnType<typeof setTimeout> | null = null

watch([fetchedItems, selectedItems, itemsInCalendar, calendarName, calendarLanguage], () => {
  if (sessionStorageTimer !== null) clearTimeout(sessionStorageTimer)
  sessionStorageTimer = setTimeout(() => {
    try {
      sessionStorage.setItem('calendarPageState', JSON.stringify({
        fetchedItems: fetchedItems.value,
        selectedItems: selectedItems.value,
        itemsInCalendar: itemsInCalendar.value,
        calendarName: calendarName.value,
        calendarLanguage: calendarLanguage.value
      }))
    } catch { /* ignore */ }
    sessionStorageTimer = null
  }, 1000)
})

onBeforeUnmount(() => {
  if (sessionStorageTimer !== null) {
    clearTimeout(sessionStorageTimer)
    sessionStorageTimer = null
  }
  if (!route.params.id || route.params.id === 'new') {
    sessionStorage.removeItem('calendarPageState')
  }
})

onBeforeMount(async () => {
  const calendarId = Array.isArray(route.params.id) ? route.params.id[0] : route.params.id
  if (calendarId && calendarId !== 'new') {
    submitLoading.value = true
    try {
      const response = await api.get(`/calendars/${calendarId}`)
      const calendar: Calendar = response.data
      calendarName.value = calendar.name
      calendarLanguage.value = calendar.language
      itemsInCalendar.value = calendar.items
      currentCalendar.value = calendar
      calculateRecommendations(itemsInCalendar.value)
    } catch {
      calendarError.value = t('calendar.loadFailed')
    } finally {
      submitLoading.value = false
    }
  } else {
    const settings = await userSettingsStore.fetchSettings()
    calendarLanguage.value =
      settings.title_language_preference === 'Romaji' ? 'romaji' :
      settings.title_language_preference === 'Native' ? 'native' : 'english'
  }
})
</script>

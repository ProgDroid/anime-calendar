<template>
  <div class="min-h-[calc(100vh-6.1rem)] bg-base-200 p-4">
    <div class="grid grid-cols-1 lg:grid-cols-2 gap-6 max-w-6xl mx-auto">
      <!-- Calendar settings + items list -->
      <div class="card bg-base-100 shadow-md">
        <div class="card-body flex flex-col gap-4">
          <h2 class="card-title">{{ $t('calendar.edit') }}</h2>
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

      <!-- Search panel -->
      <div class="card bg-base-100 shadow-md">
        <div class="card-body flex flex-col gap-4">
          <h2 class="card-title">{{ $t('calendar.search') }}</h2>
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
    </div>

    <!-- Recommendations (desktop only) -->
    <div class="max-w-6xl mx-auto hidden lg:block">
      <RecommendationsSection
        :recommendations="recommendations"
        :calendar-has-items="itemsInCalendar.length > 0"
        :calendar-language="calendarLanguage"
        @add="addItemToCalendarSingle"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, watch, onBeforeMount, onBeforeUnmount } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import type { Item } from '@/types/item'
import type { Calendar } from '@/types/calendar'
import api from '@/config/api'
import { toastService } from '@/services/toastService'
import { useUserSettingsStore } from '@/stores/userSettingsStore'
import CalendarSettingsForm from '@/components/calendar/CalendarSettingsForm.vue'
import CalendarItemsList from '@/components/calendar/CalendarItemsList.vue'
import ItemSearchPanel from '@/components/calendar/ItemSearchPanel.vue'
import RecommendationsSection from '@/components/calendar/RecommendationsSection.vue'

const { t } = useI18n()
const router = useRouter()
const route = useRoute()
const userSettingsStore = useUserSettingsStore()

const fetchedItems = ref<Item[]>([])
const selectedItems = ref<number[]>([])
const calendarName = ref('')
const calendarLanguage = ref<'english' | 'romaji' | 'native'>('english')
const itemsInCalendar = ref<Item[]>([])
const loading = ref(false)
const searchError = ref<string | null>(null)
const calendarError = ref<string | null>(null)
const recommendations = ref<Item[]>([])
const currentCalendar = ref<Calendar | null>(null)

const handleSearch = async ({ name, mediaType }: { name: string; mediaType: '' | 'ANIME' | 'MANGA' }) => {
  if (!name) {
    searchError.value = t('calendar.enterName')
    return
  }
  loading.value = true
  searchError.value = null
  try {
    let url = `/search?name=${encodeURIComponent(name)}`
    if (mediaType) url += `&media_type=${mediaType}`
    const response = await api.get(url)
    fetchedItems.value = response.data
    selectedItems.value = []
  } catch {
    searchError.value = t('calendar.fetchItemsFailed')
  } finally {
    loading.value = false
  }
}

const toggleItemSelection = (id: number) => {
  const idx = selectedItems.value.indexOf(id)
  if (idx === -1) selectedItems.value.push(id)
  else selectedItems.value.splice(idx, 1)
}

const addItemToCalendar = () => {
  const newItems = fetchedItems.value.filter(
    item => selectedItems.value.includes(item.id) &&
    !itemsInCalendar.value.some(c => c.id === item.id)
  )
  itemsInCalendar.value.push(...newItems)
  selectedItems.value = []
  if (itemsInCalendar.value.length > 0) calculateRecommendations()
}

const addItemToCalendarSingle = (item: Item) => {
  if (!itemsInCalendar.value.some(c => c.id === item.id)) {
    itemsInCalendar.value.push(item)
    calculateRecommendations()
  }
}

const removeItemFromCalendar = (id: number) => {
  itemsInCalendar.value = itemsInCalendar.value.filter(item => item.id !== id)
  calculateRecommendations()
}

const clearCalendar = () => {
  itemsInCalendar.value = []
  recommendations.value = []
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

  loading.value = true
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
    loading.value = false
  }
}

const calculateRecommendations = () => {
  if (itemsInCalendar.value.length === 0) {
    recommendations.value = []
    return
  }
  const counts = new Map<number, { count: number; totalRating: number; item: Item }>()
  itemsInCalendar.value.forEach(item => {
    item.recommendations?.forEach(rec => {
      const id = rec.media.id
      if (itemsInCalendar.value.some(c => c.id === id)) return
      if (counts.has(id)) {
        const e = counts.get(id)!
        e.count++
        e.totalRating += rec.rating
      } else {
        counts.set(id, {
          count: 1,
          totalRating: rec.rating,
          item: {
            id: rec.media.id,
            id_mal: rec.media.id_mal,
            title: rec.media.title,
            media_type: item.media_type,
            episode_duration: 0,
            airing_schedule: [],
            cover_image: rec.media.cover_image,
            banner_image: '',
            recommendations: []
          }
        })
      }
    })
  })
  recommendations.value = Array.from(counts.values())
    .sort((a, b) => b.count !== a.count ? b.count - a.count : (b.totalRating / b.count) - (a.totalRating / a.count))
    .slice(0, 5)
    .map(e => e.item)
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

watch([fetchedItems, selectedItems, itemsInCalendar, calendarName, calendarLanguage], () => {
  try {
    sessionStorage.setItem('calendarPageState', JSON.stringify({
      fetchedItems: fetchedItems.value,
      selectedItems: selectedItems.value,
      itemsInCalendar: itemsInCalendar.value,
      calendarName: calendarName.value,
      calendarLanguage: calendarLanguage.value
    }))
  } catch { /* ignore */ }
})

onBeforeUnmount(() => {
  if (!route.params.id || route.params.id === 'new') {
    sessionStorage.removeItem('calendarPageState')
  }
})

onBeforeMount(async () => {
  const calendarId = Array.isArray(route.params.id) ? route.params.id[0] : route.params.id
  if (calendarId && calendarId !== 'new') {
    loading.value = true
    try {
      const response = await api.get(`/calendars/${calendarId}`)
      const calendar: Calendar = response.data
      calendarName.value = calendar.name
      calendarLanguage.value = calendar.language
      itemsInCalendar.value = calendar.items
      currentCalendar.value = calendar
      calculateRecommendations()
    } catch {
      calendarError.value = t('calendar.loadFailed')
    } finally {
      loading.value = false
    }
  } else {
    const settings = await userSettingsStore.fetchSettings()
    calendarLanguage.value =
      settings.title_language_preference === 'Romaji' ? 'romaji' :
      settings.title_language_preference === 'Native' ? 'native' : 'english'
  }
})
</script>

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
import { ref, computed, onMounted, nextTick, watch, onBeforeMount, onBeforeUnmount } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import type { Item } from '@/types/item'
import type { Calendar, EventStyle } from '@/types/calendar'
import type { Viewer, CalendarEvent } from '@/types/sharing'
import api from '@/config/api'
import axios from 'axios'
import { toastService } from '@/services/toastService'
import { addItem, removeItem } from '@/services/calendars'
import { useAuthStore } from '@/stores/auth'
import { useUserSettingsStore } from '@/stores/userSettingsStore'
import { useUsageStore } from '@/stores/usageStore'
import { useCalendarSearch } from '@/composables/useCalendarSearch'
import { useRecommendations } from '@/composables/useRecommendations'
import { usePresence } from '@/composables/usePresence'
import { getMySubscription } from '@/services/subscription'
import CalendarSettingsForm from '@/components/calendar/CalendarSettingsForm.vue'
import CalendarItemsList from '@/components/calendar/CalendarItemsList.vue'
import ItemSearchPanel from '@/components/calendar/ItemSearchPanel.vue'
import RecommendationsSection from '@/components/calendar/RecommendationsSection.vue'
import PresenceChip from '@/components/shared/PresenceChip.vue'

defineOptions({ name: 'CalendarEditorViewDesktop' })

const { t } = useI18n()
const router = useRouter()
const route = useRoute()
const authStore = useAuthStore()
const userSettingsStore = useUserSettingsStore()
const usage = useUsageStore()
const isFreeTier = ref(true)

const calendarId = Array.isArray(route.params.id) ? route.params.id[0] : route.params.id
const isExistingCalendar = calendarId !== 'new' && calendarId !== undefined && calendarId !== ''
const numericCalendarId = isExistingCalendar ? parseInt(calendarId as string, 10) : 0
const { viewers, lastEvent } = isExistingCalendar
  ? usePresence(numericCalendarId)
  : { viewers: ref<Viewer[]>([]), lastEvent: ref<CalendarEvent | null>(null) }

const showCollisionBanner = ref(false)
const localBaselineMetaVersion = ref(0)

const { fetchedItems, selectedItems, loading: searchLoading, searchError, handleSearch, toggleItemSelection } = useCalendarSearch()
const { recommendations, calculateRecommendations } = useRecommendations()

const calendarName = ref('')
const calendarLanguage = ref<'english' | 'romaji' | 'native'>('english')
const calendarEventStyle = ref<EventStyle>('timed')
const itemsInCalendar = ref<Item[]>([])
const submitLoading = ref(false)
const calendarError = ref<string | null>(null)
const currentCalendar = ref<Calendar | null>(null)
const originalItemIds = ref<Set<number>>(new Set())
const currentUserId = ref<number | null>(null)
const isOwner = computed(() =>
  currentCalendar.value != null &&
  currentUserId.value != null &&
  currentCalendar.value.user_id === currentUserId.value,
)

const loading = computed(() => searchLoading.value || submitLoading.value)
const editorShowCount = computed(() =>
  isFreeTier.value && usage.loaded ? usage.showCount : null,
)

const addItemToCalendar = () => {
  const newItems = fetchedItems.value.filter(
    item => selectedItems.value.includes(item.id) &&
    !itemsInCalendar.value.some(c => c.id === item.id)
  )
  itemsInCalendar.value.push(...newItems)
  selectedItems.value = []
  if (itemsInCalendar.value.length > 0) calculateRecommendations(itemsInCalendar.value)
  if (isFreeTier.value) void usage.refresh()
}

const addItemToCalendarSingle = (item: Item) => {
  if (!itemsInCalendar.value.some(c => c.id === item.id)) {
    itemsInCalendar.value.push(item)
    calculateRecommendations(itemsInCalendar.value)
    if (isFreeTier.value) void usage.refresh()
  }
}

const removeItemFromCalendar = (id: number) => {
  itemsInCalendar.value = itemsInCalendar.value.filter(item => item.id !== id)
  calculateRecommendations(itemsInCalendar.value)
  if (isFreeTier.value) void usage.refresh()
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
    if (currentCalendar.value) {
      const calId = currentCalendar.value.id

      // 1. Compute diff
      const currentIds = new Set(itemsInCalendar.value.map(i => i.id))
      const toAdd = [...currentIds].filter(id => !originalItemIds.value.has(id))
      const toRemove = [...originalItemIds.value].filter(id => !currentIds.has(id))

      // 2. Apply per-item changes (works for both owner and editor)
      if (toAdd.length > 0 || toRemove.length > 0) {
        try {
          await Promise.all([
            ...toAdd.map(id => addItem(calId, id)),
            ...toRemove.map(id => removeItem(calId, id)),
          ])
          originalItemIds.value = new Set(currentIds)
        } catch {
          calendarError.value = t('calendar.updateFailed')
          submitLoading.value = false
          return
        }
      }

      // 3. Save meta via PUT (owners only — editors get 403, handled silently)
      try {
        const calendar = {
          id: calId,
          name: calendarName.value,
          language: calendarLanguage.value,
          event_style: calendarEventStyle.value,
          items: itemsInCalendar.value,
        }
        const response = await api.put('/calendar', calendar)
        toastService.success(t('calendar.updateSuccess', { name: response.data.name }))
        calendarName.value = ''
        itemsInCalendar.value = []
        router.push('/my-calendars')
      } catch (err) {
        if (axios.isAxiosError(err) && err.response?.status === 403) {
          // Editor: meta save forbidden (expected). Item ops already succeeded.
          toastService.success(t('calendar.updateSuccess', { name: calendarName.value }))
          calendarName.value = ''
          itemsInCalendar.value = []
          router.push('/my-calendars')
        } else {
          calendarError.value = t('calendar.updateFailed')
        }
      }
    } else {
      const calendar: Omit<Calendar, 'id' | 'created_at' | 'updated_at'> = {
        name: calendarName.value,
        language: calendarLanguage.value,
        event_style: calendarEventStyle.value,
        items: itemsInCalendar.value
      }
      const response = await api.put('/calendar', calendar)
      toastService.success(t('calendar.updateSuccess', { name: response.data.name }))
      calendarName.value = ''
      itemsInCalendar.value = []
      router.push('/my-calendars')
    }
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
      if (state.calendarEventStyle !== undefined) calendarEventStyle.value = state.calendarEventStyle
    }
  } catch { /* ignore */ }
})

let sessionStorageTimer: ReturnType<typeof setTimeout> | null = null

watch([fetchedItems, selectedItems, itemsInCalendar, calendarName, calendarLanguage, calendarEventStyle], () => {
  if (sessionStorageTimer !== null) clearTimeout(sessionStorageTimer)
  sessionStorageTimer = setTimeout(() => {
    try {
      sessionStorage.setItem('calendarPageState', JSON.stringify({
        fetchedItems: fetchedItems.value,
        selectedItems: selectedItems.value,
        itemsInCalendar: itemsInCalendar.value,
        calendarName: calendarName.value,
        calendarLanguage: calendarLanguage.value,
        calendarEventStyle: calendarEventStyle.value
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

const reloadPage = () => { window.location.reload() }

watch(lastEvent, (frame) => {
  if (!frame) return
  switch (frame.type) {
    case 'item_added':
      // Only media_id is available; no endpoint to fetch a full Item — skip local list update.
      if (frame.actor !== authStore.user) {
        toastService.success(t('sharing.toasts.itemAdded', { actor: frame.actor }))
      }
      break
    case 'item_removed':
      if (frame.actor !== authStore.user) {
        itemsInCalendar.value = itemsInCalendar.value.filter(i => i.id !== frame.media_id)
        toastService.success(t('sharing.toasts.itemRemoved', { actor: frame.actor }))
      }
      break
    case 'meta_updated':
      if (frame.actor !== authStore.user && frame.v > localBaselineMetaVersion.value) {
        showCollisionBanner.value = true
      }
      break
    case 'member_joined':
      toastService.success(t('sharing.toasts.joined', { name: frame.display }))
      break
    case 'member_left':
      toastService.success(t(`sharing.toasts.left.${frame.reason}`))
      break
    case 'kick':
      toastService.error(t(`sharing.toasts.kick.${frame.reason}`))
      void router.push('/my-calendars')
      break
  }
})

onBeforeMount(async () => {
  // Bootstrap effective tier + usage counts. Free users get the counter
  // chip + 80% banner + cap-aware add gates; Pro users do not.
  try {
    const ent = await getMySubscription()
    isFreeTier.value = ent.tier !== 'paid'
  } catch {
    isFreeTier.value = true
  }
  if (isFreeTier.value) void usage.refresh()
  if (calendarId && calendarId !== 'new') {
    submitLoading.value = true
    try {
      const response = await api.get(`/calendars/${calendarId}`)
      const calendar: Calendar = response.data
      calendarName.value = calendar.name
      calendarLanguage.value = calendar.language
      calendarEventStyle.value = calendar.event_style ?? 'timed'
      itemsInCalendar.value = calendar.items
      currentCalendar.value = calendar
      originalItemIds.value = new Set(calendar.items.map(i => i.id))
      calculateRecommendations(itemsInCalendar.value)
      localBaselineMetaVersion.value = calendar.meta_version ?? 0
    } catch {
      calendarError.value = t('calendar.loadFailed')
    } finally {
      submitLoading.value = false
    }
    try {
      const settings = await userSettingsStore.fetchSettings()
      currentUserId.value = settings.user_id ?? null
    } catch { /* non-critical */ }
  } else {
    const settings = await userSettingsStore.fetchSettings()
    currentUserId.value = settings.user_id ?? null
    calendarLanguage.value =
      settings.title_language_preference === 'Romaji' ? 'romaji' :
      settings.title_language_preference === 'Native' ? 'native' : 'english'
  }
})
</script>

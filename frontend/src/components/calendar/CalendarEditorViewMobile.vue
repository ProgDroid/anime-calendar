<script setup lang="ts">
import { ref, computed, nextTick, onBeforeUnmount, watch } from 'vue'
import { useRoute, useRouter, RouterLink, onBeforeRouteLeave, onBeforeRouteUpdate } from 'vue-router'
import { useI18n } from 'vue-i18n'
import axios from 'axios'
import api from '@/config/api'
import { toastService } from '@/services/toastService'
import { addItem, removeItem } from '@/services/calendars'
import type { Item } from '@/types/item'
import type { Calendar, EventStyle } from '@/types/calendar'
import type { Viewer, CalendarEvent } from '@/types/sharing'
import { useAuthStore } from '@/stores/auth'
import { useUserSettingsStore } from '@/stores/userSettingsStore'
import { useEditorSelectionStore } from '@/stores/editorSelection'
import { useUsageStore } from '@/stores/usageStore'
import { useRecommendations } from '@/composables/useRecommendations'
import { usePresence } from '@/composables/usePresence'
import { getMySubscription } from '@/services/subscription'
import UiSegmented from '@/components/ui/UiSegmented.vue'
import IconPlus from '@/components/ui/icons/IconPlus.vue'
import EditorItemsPanelMobile from './EditorItemsPanelMobile.vue'
import EditorSearchPanelMobile from './EditorSearchPanelMobile.vue'
import CalendarSettingsForm from './CalendarSettingsForm.vue'
import PresenceChip from '@/components/shared/PresenceChip.vue'

defineOptions({ name: 'CalendarEditorViewMobile' })

const { t } = useI18n()
const route = useRoute()
const router = useRouter()
const authStore = useAuthStore()
const userSettingsStore = useUserSettingsStore()
const selection = useEditorSelectionStore()
const usage = useUsageStore()
const isFreeTier = ref(true)

const { recommendations, calculateRecommendations } = useRecommendations()

// Editor state — mirrors CalendarEditorViewDesktop
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

const loading = computed(() => submitLoading.value)
const itemCount = computed(() => itemsInCalendar.value.length)
// Pro users see no chip / banner / disabled add — pass `null` through.
const editorShowCount = computed(() =>
  isFreeTier.value && usage.loaded ? usage.showCount : null,
)

// Tab state
type Tab = 'items' | 'search'
const tab = ref<Tab>('items')
const searchPanelRef = ref<InstanceType<typeof EditorSearchPanelMobile> | null>(null)

const tabOptions = computed(() => [
  { value: 'items', label: t('mobile.editor.itemsTab', { count: itemCount.value }) },
  { value: 'search', label: t('mobile.editor.searchTab') },
])

function jumpToSearch() {
  tab.value = 'search'
  void nextTick(() => {
    searchPanelRef.value?.focus()
  })
}

const addItemFromSearch = (items: Item[]) => {
  const newItems = items.filter(
    item => !itemsInCalendar.value.some(c => c.id === item.id),
  )
  if (newItems.length === 0) return
  itemsInCalendar.value.push(...newItems)
  selection.clear()
  calculateRecommendations(itemsInCalendar.value)
  if (isFreeTier.value) void usage.refresh()
  // Switch back to items tab to see the added items
  tab.value = 'items'
}

const addRecommendation = (item: Item) => {
  if (itemsInCalendar.value.some(c => c.id === item.id)) return
  itemsInCalendar.value.push(item)
  calculateRecommendations(itemsInCalendar.value)
  if (isFreeTier.value) void usage.refresh()
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
        items: itemsInCalendar.value,
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

// Session storage persistence — debounced, mirrors desktop
let sessionStorageTimer: ReturnType<typeof setTimeout> | null = null // TODO: add debounced session-storage watcher (matches desktop)

onBeforeUnmount(() => {
  if (sessionStorageTimer !== null) {
    clearTimeout(sessionStorageTimer)
    sessionStorageTimer = null
  }
  if (!route.params.id || route.params.id === 'new') {
    sessionStorage.removeItem('calendarPageState')
  }
})

// Clear selection on route changes
onBeforeRouteUpdate(() => selection.clear())
onBeforeRouteLeave(() => selection.clear())

// Bootstrap effective tier + usage counts (mobile editor). Free users get
// the counter chip + 80% banner + cap-aware add buttons; Pro users do not.
void (async () => {
  try {
    const ent = await getMySubscription()
    isFreeTier.value = ent.tier !== 'paid'
  } catch {
    isFreeTier.value = true
  }
  if (isFreeTier.value) void usage.refresh()
})()

// Load calendar on mount — same logic as desktop onBeforeMount
const calendarId = Array.isArray(route.params.id) ? route.params.id[0] : route.params.id
const isExistingCalendar = calendarId !== 'new' && calendarId !== undefined && calendarId !== ''
const numericCalendarId = isExistingCalendar ? parseInt(calendarId as string, 10) : 0
const { viewers, lastEvent } = isExistingCalendar
  ? usePresence(numericCalendarId)
  : { viewers: ref<Viewer[]>([]), lastEvent: ref<CalendarEvent | null>(null) }

const showCollisionBanner = ref(false)
const localBaselineMetaVersion = ref(0)

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

if (calendarId && calendarId !== 'new') {
  submitLoading.value = true
  api.get(`/calendars/${calendarId}`)
    .then((response) => {
      const calendar: Calendar = response.data
      calendarName.value = calendar.name
      calendarLanguage.value = calendar.language
      calendarEventStyle.value = calendar.event_style ?? 'timed'
      itemsInCalendar.value = calendar.items
      currentCalendar.value = calendar
      originalItemIds.value = new Set(calendar.items.map(i => i.id))
      calculateRecommendations(itemsInCalendar.value)
      localBaselineMetaVersion.value = calendar.meta_version ?? 0
    })
    .catch(() => {
      calendarError.value = t('calendar.loadFailed')
    })
    .finally(() => {
      submitLoading.value = false
    })
  userSettingsStore.fetchSettings().then((settings) => {
    currentUserId.value = settings.user_id ?? null
  }).catch(() => { /* non-critical */ })
} else {
  userSettingsStore.fetchSettings().then((settings) => {
    currentUserId.value = settings.user_id ?? null
    calendarLanguage.value =
      settings.title_language_preference === 'Romaji' ? 'romaji' :
      settings.title_language_preference === 'Native' ? 'native' : 'english'
  })
}
</script>

<template>
  <div class="relative flex flex-col min-h-screen bg-bg-0">
    <!-- Top bar -->
    <header class="bg-bg-0 px-4 pb-2 border-b border-line-soft"
      :style="{ paddingTop: 'max(16px, calc(env(safe-area-inset-top) + 12px))' }"
    >
      <div class="flex items-center gap-3">
        <RouterLink
          to="/my-calendars"
          class="text-fg-1 shrink-0 p-1 -ml-1"
          :aria-label="t('app.back')"
        >
          ‹
        </RouterLink>
        <span class="font-semibold text-fg-1 truncate flex-1">
          {{ calendarName || t('calendar.edit') }}
        </span>
        <PresenceChip v-if="isExistingCalendar && viewers.length > 0" :viewers="viewers" />
      </div>
    </header>

    <!-- Collision banner -->
    <div
      v-if="showCollisionBanner"
      data-testid="collision-banner"
      class="mx-4 mt-2 rounded-md bg-bg-2 border border-line px-3 py-2 text-sm text-fg-2 flex items-center justify-between gap-2"
    >
      <span>{{ $t('sharing.collisionWarning') }}</span>
      <button
        class="text-accent-1 font-medium hover:underline shrink-0"
        @click="reloadPage"
      >
        {{ $t('sharing.reloadButton') }}
      </button>
    </div>

    <!-- Settings form (always visible at top on mobile) -->
    <div class="px-4 pt-3">
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
    </div>

    <!-- Tab switcher — wrap in w-full div since UiSegmented is inline-flex -->
    <div class="px-4 pt-4 pb-3" data-testid="editor-tab-bar">
      <div class="w-full flex">
        <UiSegmented
          v-model="tab"
          :options="tabOptions"
          variant="tab"
          :aria-label="t('mobile.editor.tabGroupLabel')"
          class="w-full [&>button]:flex-1"
        />
      </div>
    </div>

    <!-- Tab panels: page-level scroll keeps it native and avoids nested scrollers. -->
    <div class="flex-1 min-h-0">
      <KeepAlive>
        <EditorItemsPanelMobile
          v-if="tab === 'items'"
          :items="itemsInCalendar"
          :calendar-language="calendarLanguage"
          :recommendations="recommendations"
          :show-count="editorShowCount"
          @remove="removeItemFromCalendar"
          @clear="clearCalendar"
          @add-recommendation="addRecommendation"
        />
        <EditorSearchPanelMobile
          v-else
          ref="searchPanelRef"
          :items-in-calendar="itemsInCalendar"
          :calendar-language="calendarLanguage"
          :recommendations="recommendations"
          :show-count="editorShowCount"
          @add-selected="addItemFromSearch"
          @add-recommendation="addRecommendation"
        />
      </KeepAlive>
    </div>

    <!-- FAB — Items tab only, jumps to Search -->
    <button
      v-if="tab === 'items'"
      :aria-label="t('mobile.editor.fabAdd')"
      data-testid="editor-fab"
      class="fixed z-20 flex h-14 w-14 items-center justify-center rounded-full bg-accent-1 shadow-lg transition-all duration-[var(--d-2)] motion-reduce:transition-none motion-reduce:duration-0"
      :style="{ right: '18px', bottom: '90px', color: 'var(--accent-1-fg)' }"
      @click="jumpToSearch"
    >
      <IconPlus />
    </button>
  </div>
</template>

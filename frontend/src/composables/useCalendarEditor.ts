import { ref, computed, watch, onBeforeUnmount } from 'vue'
import { useRoute, useRouter } from 'vue-router'
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
import { useUsageStore } from '@/stores/usageStore'
import { useRecommendations } from '@/composables/useRecommendations'
import { usePresence } from '@/composables/usePresence'
import { getMySubscription } from '@/services/subscription'

type CalendarLanguage = 'english' | 'romaji' | 'native'

interface CalendarDraft {
  calendarName: string
  calendarLanguage: CalendarLanguage
  calendarEventStyle: EventStyle
  itemsInCalendar: Item[]
}

const DRAFT_DEBOUNCE_MS = 1000

/**
 * Shared core for both editor views (desktop + mobile). Owns editor state,
 * item mutations, submit, the SSE live-sync watcher, presence, recommendations,
 * the bootstrap (tier + calendar load + settings), and keyed draft persistence
 * (F2-11). Each view keeps only its own search-input glue + template.
 */
export function useCalendarEditor() {
  const { t } = useI18n()
  const route = useRoute()
  const router = useRouter()
  const authStore = useAuthStore()
  const userSettingsStore = useUserSettingsStore()
  const usage = useUsageStore()

  const { recommendations, calculateRecommendations } = useRecommendations()

  // ── Identity ──────────────────────────────────────────────────────────────
  const calendarId = Array.isArray(route.params.id) ? route.params.id[0] : route.params.id
  const isExistingCalendar = calendarId !== 'new' && calendarId !== undefined && calendarId !== ''
  const numericCalendarId = isExistingCalendar ? parseInt(calendarId as string, 10) : 0
  // F2-11: draft key is scoped to the route id so calendar A's draft can never
  // be restored under /calendar/B, and a desktop↔mobile viewport flip for the
  // same id reuses the same slot (no edit loss).
  const draftKey = `calendarDraft:${calendarId ?? 'new'}`

  const { viewers, lastEvent } = isExistingCalendar
    ? usePresence(numericCalendarId)
    : { viewers: ref<Viewer[]>([]), lastEvent: ref<CalendarEvent | null>(null) }

  // ── State ─────────────────────────────────────────────────────────────────
  const calendarName = ref('')
  const calendarLanguage = ref<CalendarLanguage>('english')
  const calendarEventStyle = ref<EventStyle>('timed')
  const itemsInCalendar = ref<Item[]>([])
  const submitLoading = ref(false)
  const calendarError = ref<string | null>(null)
  const currentCalendar = ref<Calendar | null>(null)
  const currentUserId = ref<number | null>(null)
  const originalItemIds = ref<Set<number>>(new Set())
  const isFreeTier = ref(true)
  const showCollisionBanner = ref(false)
  const localBaselineMetaVersion = ref(0)

  const isOwner = computed(() =>
    currentCalendar.value != null &&
    currentUserId.value != null &&
    currentCalendar.value.user_id === currentUserId.value,
  )
  const editorShowCount = computed(() =>
    isFreeTier.value && usage.loaded ? usage.showCount : null,
  )

  // ── Item mutations ──────────────────────────────────────────────────────────
  const addItems = (items: Item[]) => {
    const newItems = items.filter(
      item => !itemsInCalendar.value.some(c => c.id === item.id),
    )
    if (newItems.length === 0) return
    itemsInCalendar.value.push(...newItems)
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

  // ── Draft persistence (F2-11) ───────────────────────────────────────────────
  function readDraft(): CalendarDraft | null {
    try {
      const raw = sessionStorage.getItem(draftKey)
      return raw ? (JSON.parse(raw) as CalendarDraft) : null
    } catch {
      return null
    }
  }

  function applyDraft(d: CalendarDraft) {
    if (d.calendarName !== undefined) calendarName.value = d.calendarName
    if (d.calendarLanguage !== undefined) calendarLanguage.value = d.calendarLanguage
    if (d.calendarEventStyle !== undefined) calendarEventStyle.value = d.calendarEventStyle
    if (d.itemsInCalendar !== undefined) {
      itemsInCalendar.value = d.itemsInCalendar
      calculateRecommendations(itemsInCalendar.value)
    }
  }

  function clearDraft() {
    try {
      sessionStorage.removeItem(draftKey)
    } catch {
      /* ignore */
    }
  }

  let draftTimer: ReturnType<typeof setTimeout> | null = null
  watch(
    [calendarName, calendarLanguage, calendarEventStyle, itemsInCalendar],
    () => {
      if (draftTimer !== null) clearTimeout(draftTimer)
      draftTimer = setTimeout(() => {
        try {
          sessionStorage.setItem(
            draftKey,
            JSON.stringify({
              calendarName: calendarName.value,
              calendarLanguage: calendarLanguage.value,
              calendarEventStyle: calendarEventStyle.value,
              itemsInCalendar: itemsInCalendar.value,
            }),
          )
        } catch {
          /* ignore */
        }
        draftTimer = null
      }, DRAFT_DEBOUNCE_MS)
    },
    { deep: true },
  )

  onBeforeUnmount(() => {
    if (draftTimer !== null) {
      clearTimeout(draftTimer)
      draftTimer = null
    }
    // Only the throwaway "new" draft is cleared on unmount; existing-calendar
    // drafts persist (keyed by id) so a reload / viewport flip keeps edits.
    if (!isExistingCalendar) clearDraft()
  })

  // ── Submit ──────────────────────────────────────────────────────────────────
  const finishSuccess = (name: string) => {
    toastService.success(t('calendar.updateSuccess', { name }))
    // Cancel any pending debounced draft write before clearing the slot, so a
    // late timer can't re-persist an empty draft after clearDraft() (which would
    // clobber an existing calendar's items on the next visit).
    if (draftTimer !== null) {
      clearTimeout(draftTimer)
      draftTimer = null
    }
    clearDraft()
    calendarName.value = ''
    itemsInCalendar.value = []
    router.push('/my-calendars')
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
          finishSuccess(response.data.name)
        } catch (err) {
          if (axios.isAxiosError(err) && err.response?.status === 403) {
            // Editor: meta save forbidden (expected). Item ops already succeeded.
            finishSuccess(calendarName.value)
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
        finishSuccess(response.data.name)
      }
    } catch {
      calendarError.value = t('calendar.updateFailed')
    } finally {
      submitLoading.value = false
    }
  }

  // ── Live sync (SSE) ─────────────────────────────────────────────────────────
  const reloadPage = () => {
    window.location.reload()
  }

  watch(lastEvent, frame => {
    if (!frame) return
    switch (frame.type) {
      case 'item_added':
        // Only media_id is available; no endpoint to fetch a full Item — skip
        // local list update. F2-12: toast the human display name, not the id.
        if (frame.actor !== String(authStore.userId)) {
          toastService.success(t('sharing.toasts.itemAdded', { actor: frame.display }))
        }
        break
      case 'item_removed':
        if (frame.actor !== String(authStore.userId)) {
          itemsInCalendar.value = itemsInCalendar.value.filter(i => i.id !== frame.media_id)
          toastService.success(t('sharing.toasts.itemRemoved', { actor: frame.display }))
        }
        break
      case 'meta_updated':
        if (frame.actor !== String(authStore.userId) && frame.v > localBaselineMetaVersion.value) {
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

  // ── Bootstrap (tier + calendar load + settings + draft overlay) ──────────────
  void (async () => {
    try {
      const ent = await getMySubscription()
      isFreeTier.value = ent.tier !== 'paid'
    } catch {
      isFreeTier.value = true
    }
    if (isFreeTier.value) void usage.refresh()

    if (isExistingCalendar) {
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
      } catch {
        /* non-critical */
      }
    } else {
      const settings = await userSettingsStore.fetchSettings()
      currentUserId.value = settings.user_id ?? null
      calendarLanguage.value =
        settings.title_language_preference === 'Romaji'
          ? 'romaji'
          : settings.title_language_preference === 'Native'
            ? 'native'
            : 'english'
    }

    // F2-11: overlay a saved draft AFTER the server load so in-progress edits
    // win over the persisted server state on reload / viewport flip.
    const draft = readDraft()
    if (draft) applyDraft(draft)
  })()

  return {
    calendarId,
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
  }
}

import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import api from '@/config/api'

/**
 * Free-tier usage caps. Hardcoded on the frontend for now; matches
 * `LimitsConfig::default()` in `server/src/config/server.rs`. A future change
 * can pipe these in via `/api/public-config`.
 */
export const FREE_CALENDAR_CAP = 3
export const FREE_SHOW_CAP = 25

/**
 * 0..1 ratio above which the editor shows a soft "approaching cap" banner.
 */
export const CAP_WARN_THRESHOLD = 0.8

interface UsageResponse {
  shows: number
  calendars: number
}

/**
 * Per-user usage counts driving the free-tier cap UX (counter chips, 80%
 * banner, disabled-add buttons). Backed by `GET /api/account/usage`.
 *
 * Refreshed on demand: call `refresh()` after a successful create/add/remove
 * mutation. Coupling counts to the settings store would force a settings
 * refetch on every editor edit — separate endpoint avoids that.
 */
export const useUsageStore = defineStore('usage', () => {
  const showCount = ref(0)
  const calendarCount = ref(0)
  const loaded = ref(false)
  let inFlight: Promise<void> | null = null

  const showCapReached = computed(() => showCount.value >= FREE_SHOW_CAP)
  const calendarCapReached = computed(() => calendarCount.value >= FREE_CALENDAR_CAP)
  const showsNearCap = computed(
    () => showCount.value / FREE_SHOW_CAP >= CAP_WARN_THRESHOLD,
  )

  async function refresh(): Promise<void> {
    // Deduplicate concurrent refresh calls — second caller waits on the first.
    if (inFlight) return inFlight
    inFlight = (async () => {
      try {
        const res = await api.get<UsageResponse>('/account/usage')
        showCount.value = res.data.shows
        calendarCount.value = res.data.calendars
        loaded.value = true
      } catch {
        // Silent — leave previous values in place. Cap UI degrades gracefully
        // (defaults to 0/0 = no chip on a fresh load, no false positives).
      } finally {
        inFlight = null
      }
    })()
    return inFlight
  }

  function reset() {
    showCount.value = 0
    calendarCount.value = 0
    loaded.value = false
  }

  return {
    showCount,
    calendarCount,
    loaded,
    showCapReached,
    calendarCapReached,
    showsNearCap,
    refresh,
    reset,
  }
})

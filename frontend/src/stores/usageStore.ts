import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import api from '@/config/api'
import { getPublicConfig } from '@/services/publicConfig'

/**
 * Free-tier usage caps + Pro ceiling, sourced from the runtime
 * `/api/public-config` payload (mirrors `LimitsConfig` on the server) so a
 * server-side limit change drives both cap *enforcement* (the computeds below,
 * consumed across the editor + my-calendars surfaces) and cap *display*
 * (pricing / interrupt copy) from one place — no more silent drift (M-10,
 * F2-28). Evaluated at module load, which in the running app happens after the
 * `loadPublicConfig()` bootstrap; the `catch` falls back to the backend
 * defaults for unit tests, which never bootstrap the config cache.
 */
function freeLimits(): { freeCalendarLimit: number; freeShowCap: number; proMaxReminders: number } {
  try {
    return getPublicConfig().limits
  } catch {
    return { freeCalendarLimit: 3, freeShowCap: 25, proMaxReminders: 5 }
  }
}

const RESOLVED_LIMITS = freeLimits()
export const FREE_CALENDAR_CAP = RESOLVED_LIMITS.freeCalendarLimit
export const FREE_SHOW_CAP = RESOLVED_LIMITS.freeShowCap
export const PRO_MAX_REMINDERS = RESOLVED_LIMITS.proMaxReminders

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

  return {
    showCount,
    calendarCount,
    loaded,
    showCapReached,
    calendarCapReached,
    showsNearCap,
    refresh,
  }
})

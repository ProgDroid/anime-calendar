import { ref, readonly } from 'vue'

/**
 * Reasons the upgrade modal can be opened. Matches the backend `reason` codes
 * from `Error::PaymentRequired` (`server/src/error.rs`):
 *
 * - `cap_calendars` — Free user attempting to create a 4th calendar.
 * - `cap_shows` — Free user attempting to add a 26th distinct show.
 * - `pro_accent` — Free user attempting to apply a Pro-tier accent. Default
 *   fallback when the backend omits a reason on a 402.
 * - `share_calendar` — Free owner attempting to invite a co-editor.
 */
export type UpgradeReason = 'cap_calendars' | 'cap_shows' | 'pro_accent' | 'share_calendar'

// Module-level singleton refs so every consumer of the composable shares one
// modal mount + state. Mounted once globally in App.vue.
const open = ref(false)
const reason = ref<UpgradeReason>('pro_accent')

/**
 * Imperative handle for opening the global upgrade modal. Used by:
 *
 * 1. Cap-aware client gates that disable buttons and route to the modal.
 * 2. The axios 402 interceptor (see `config/api.ts`).
 *
 * The modal itself lives in `App.vue` bound to these refs.
 */
export function useUpgradeInterrupt() {
  function openUpgradeModal(r: UpgradeReason = 'pro_accent') {
    reason.value = r
    open.value = true
  }
  function closeUpgradeModal() {
    open.value = false
  }
  function setOpen(v: boolean) {
    open.value = v
  }
  return {
    open: readonly(open),
    reason: readonly(reason),
    openUpgradeModal,
    closeUpgradeModal,
    setOpen,
  }
}

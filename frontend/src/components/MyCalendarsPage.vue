<template>
  <div class="min-h-[calc(100vh-6.1rem)] bg-bg-0 text-fg-1 px-6 py-8 sm:px-10 sm:py-10">
    <header class="flex flex-col gap-4 sm:flex-row sm:items-end sm:gap-4">
      <div class="min-w-0 sm:flex-1">
        <p class="text-xs uppercase tracking-wider text-fg-2">{{ $t('calendars.yourLibrary') }}</p>
        <div class="flex items-center gap-3 flex-wrap mt-1">
          <h1 class="font-display text-3xl sm:text-4xl text-fg-1">{{ $t('calendars.title') }}</h1>
          <span
            v-if="showCounter"
            data-testid="calendar-counter-chip"
            class="inline-flex items-center rounded-full bg-bg-1 border border-line text-fg-2 px-2.5 py-0.5 text-xs"
            :class="{ 'border-warning text-warning': calendarCapReached }"
          >
            {{ $t('calendars_limits.counter', { count: usage.calendarCount, max: 3 }) }}
          </span>
        </div>
      </div>
      <UiButton
        data-testid="my-calendars-new"
        variant="primary"
        :class="['self-start sm:self-auto', { 'opacity-60': newCalendarDisabled }]"
        :aria-disabled="newCalendarDisabled"
        @click="createNewCalendar"
      >
        <IconPlus class="w-3.5 h-3.5" /> {{ $t('calendars.createNew') }}
      </UiButton>
    </header>

    <div
      v-if="!loading && !error && calendars.length > 0"
      data-testid="my-calendars-stats"
      class="flex flex-wrap items-center gap-3 mt-4 text-sm text-fg-2"
    >
      <span>{{ $t('calendars.stats.calendarCount', { count: pagination.total }) }}</span>
      <span aria-hidden="true">·</span>
      <span>{{ $t('calendars.stats.totalItems', { count: totalItems }) }}</span>
      <template v-if="totalAiring > 0">
        <span aria-hidden="true">·</span>
        <span
          data-testid="my-calendars-airing"
          class="inline-flex items-center gap-1 text-warning"
        >
          <span class="w-1.5 h-1.5 rounded-full bg-warning" aria-hidden="true" />
          {{ $t('calendars.stats.airing', { count: totalAiring }) }}
        </span>
      </template>
    </div>

    <div
      v-if="error"
      data-testid="my-calendars-error"
      role="alert"
      class="mt-6 rounded-md border border-danger/40 bg-danger/10 text-danger-text px-4 py-3 text-sm"
    >
      {{ error }}
    </div>

    <div
      v-else-if="loading"
      data-testid="my-calendars-loading"
      class="mt-6 text-sm text-fg-2"
    >
      {{ $t('calendars.loading') }}
    </div>

    <UiEmptyState
      v-else-if="calendars.length === 0"
      data-testid="my-calendars-empty"
      class="mt-12"
      :title="$t('calendars.notFound')"
      :body="$t('calendars.emptyBody')"
    >
      <template #action>
        <UiButton variant="primary" @click="createNewCalendar">
          <IconPlus class="w-3.5 h-3.5" /> {{ $t('calendars.createNew') }}
        </UiButton>
      </template>
    </UiEmptyState>

    <div
      v-else-if="isMobile"
      data-testid="my-calendars-mobile-stack"
      class="mt-6 flex flex-col gap-3"
    >
      <CalendarTile
        v-for="calendar in calendars"
        :key="calendar.id"
        :calendar="calendar"
        @open="editCalendar(calendar.id)"
        @edit="editCalendar(calendar.id)"
        @delete="confirmDelete(calendar.id)"
        @export-ics="exportCalendar(calendar.id)"
        @copy-link="copySubscriptionLink(calendar.subscription_token)"
        @open-google="openInGoogleCalendar(calendar.subscription_token)"
      />
      <button
        type="button"
        data-testid="my-calendars-mobile-create"
        :class="[
          'rounded-xl border border-dashed border-line px-5 py-6 focus-visible:outline-2 focus-visible:outline-accent-1 focus-visible:outline-offset-2 transition',
          newCalendarDisabled
            ? 'cursor-not-allowed opacity-60 text-fg-2'
            : 'text-fg-2 hover:border-line-strong hover:bg-bg-1 hover:text-fg-1',
        ]"
        :aria-disabled="newCalendarDisabled"
        @click="createNewCalendar"
      >
        + {{ $t('calendars.createNew') }}
      </button>
    </div>

    <div
      v-else
      data-testid="my-calendars"
      class="mt-6 grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-5"
    >
      <CalendarTile
        v-for="calendar in calendars"
        :key="calendar.id"
        :calendar="calendar"
        @open="editCalendar(calendar.id)"
        @edit="editCalendar(calendar.id)"
        @delete="confirmDelete(calendar.id)"
        @export-ics="exportCalendar(calendar.id)"
        @copy-link="copySubscriptionLink(calendar.subscription_token)"
        @open-google="openInGoogleCalendar(calendar.subscription_token)"
      />
    </div>

    <PaginationControls
      v-if="!loading && !error && calendars.length > 0"
      :page="pagination.page"
      :page_size="pagination.page_size"
      :total="pagination.total"
      :total_pages="pagination.total_pages"
      @page-change="loadCalendars"
    />

    <ConfirmModal
      :open="confirmModalOpen"
      :title="$t('calendars.deleteConfirmTitle')"
      :message="$t('calendars.deleteConfirmMessage')"
      :confirm-label="$t('calendars.delete')"
      :danger="true"
      @confirm="executeDelete"
      @cancel="confirmModalOpen = false"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import type { PageCalendar } from '@/types/calendar'
import api, { getApiUrl } from '@/config/api'
import { toastService } from '@/services/toastService'
import { useViewportLayout } from '@/composables/useViewportLayout'
import { useUpgradeInterrupt } from '@/composables/useUpgradeInterrupt'
import { useUsageStore, FREE_CALENDAR_CAP } from '@/stores/usageStore'
import { getMySubscription } from '@/services/subscription'
import PaginationControls from '@/components/shared/PaginationControls.vue'
import ConfirmModal from '@/components/shared/ConfirmModal.vue'
import CalendarTile from '@/components/shared/CalendarTile.vue'
import UiButton from '@/components/ui/UiButton.vue'
import UiEmptyState from '@/components/ui/UiEmptyState.vue'
import IconPlus from '@/components/ui/icons/IconPlus.vue'

defineOptions({ name: 'MyCalendarsPage' })

const { t } = useI18n()
const router = useRouter()
const { isMobile } = useViewportLayout()
const usage = useUsageStore()
const { openUpgradeModal } = useUpgradeInterrupt()

const calendars = ref<PageCalendar[]>([])
const loading = ref(false)
const error = ref<string | null>(null)
const pagination = ref({ page: 1, page_size: 6, total: 0, total_pages: 0 })
const confirmModalOpen = ref(false)
const calendarToDelete = ref<number | null>(null)
const isFreeTier = ref(true)

// Free-tier counter chip + cap state. Pro users see no chip.
const showCounter = computed(() => isFreeTier.value && usage.loaded)
const calendarCapReached = computed(
  () => isFreeTier.value && usage.calendarCount >= FREE_CALENDAR_CAP,
)
const newCalendarDisabled = computed(() => calendarCapReached.value)

const totalItems = computed(() =>
  calendars.value.reduce((sum, c) => sum + c.item_count, 0),
)

const totalAiring = computed(() =>
  calendars.value.reduce((sum, c) => sum + (c.airing_count ?? 0), 0),
)

onMounted(async () => {
  void loadCalendars(1)
  // Resolve effective tier so we know whether to render the cap chip.
  // Silent on failure — defaults to free, which is the safer fallback for
  // counter visibility (the chip just sits there showing real counts).
  try {
    const ent = await getMySubscription()
    isFreeTier.value = ent.tier !== 'paid'
  } catch {
    isFreeTier.value = true
  }
  if (isFreeTier.value) {
    void usage.refresh()
  }
})

async function loadCalendars(page: number = 1) {
  error.value = null
  const loadingTimer = setTimeout(() => {
    loading.value = true
  }, 150)
  try {
    const response = await api.get('/calendars', {
      params: { page, page_size: pagination.value.page_size },
    })
    calendars.value = response.data.data
    pagination.value = response.data.pagination
  } catch {
    error.value = t('calendars.loadFailed')
  } finally {
    clearTimeout(loadingTimer)
    loading.value = false
  }
}

function createNewCalendar() {
  if (newCalendarDisabled.value) {
    openUpgradeModal('cap_calendars')
    return
  }
  router.push('/calendar/new')
}

function editCalendar(id: number) {
  router.push(`/calendar/${id}`)
}

function confirmDelete(id: number) {
  calendarToDelete.value = id
  confirmModalOpen.value = true
}

async function executeDelete() {
  confirmModalOpen.value = false
  if (calendarToDelete.value === null) return
  try {
    await api.delete(`/calendars/${calendarToDelete.value}`)
    await loadCalendars(pagination.value.page)
    if (isFreeTier.value) void usage.refresh()
  } catch {
    error.value = t('calendars.deleteFailed')
  } finally {
    calendarToDelete.value = null
  }
}

async function copySubscriptionLink(token: string) {
  try {
    await navigator.clipboard.writeText(getApiUrl(`/calendars/subscribe/${token}`))
    toastService.success(t('calendars.linkCopied'))
  } catch {
    error.value = t('errors.generic')
  }
}

function openInGoogleCalendar(token: string) {
  const icalUrl = getApiUrl(`/calendars/subscribe/${token}`)
  window.open(
    `https://calendar.google.com/calendar/r?cid=${encodeURIComponent(icalUrl)}`,
    '_blank',
    'noopener,noreferrer',
  )
}

async function exportCalendar(id: number) {
  try {
    const response = await api.get(`/calendars/${id}/export`, { responseType: 'text' })
    const blob = new Blob([response.data], { type: 'text/calendar' })
    const url = window.URL.createObjectURL(blob)
    const link = document.createElement('a')
    link.href = url
    link.setAttribute('download', `calendar-${id}.ics`)
    document.body.appendChild(link)
    link.click()
    document.body.removeChild(link)
    window.URL.revokeObjectURL(url)
  } catch {
    error.value = t('calendars.exportFailed')
  }
}
</script>

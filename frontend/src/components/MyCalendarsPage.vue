<template>
  <div class="min-h-[calc(100vh-6.1rem)] bg-base-200 p-4">
    <h1 class="text-2xl font-bold mb-6">{{ $t('calendars.title') }}</h1>

    <div class="flex justify-center mb-6">
      <button class="btn btn-primary" @click="createNewCalendar">
        {{ $t('calendars.createNew') }}
      </button>
    </div>

    <div v-if="loading" class="alert alert-info">{{ $t('calendars.loading') }}</div>
    <div v-else-if="error" class="alert alert-error">{{ error }}</div>
    <div v-else-if="calendars.length === 0" class="alert alert-info">
      {{ $t('calendars.notFound') }}
      <button class="btn btn-sm btn-primary ml-2" @click="createNewCalendar">
        {{ $t('calendars.createNew') }}
      </button>
    </div>

    <div v-else class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
      <div
        v-for="calendar in calendars"
        :key="calendar.id"
        class="card bg-base-100 shadow-md hover:shadow-lg transition-shadow duration-300"
      >
        <div class="card-body">
          <h2 class="card-title text-lg font-bold">{{ calendar.name }}</h2>
          <div class="space-y-1">
            <p class="text-sm text-base-content/60">
              {{ $t('calendars.created') }}: <span class="font-semibold">{{ formatDate(calendar.created_at) }}</span>
            </p>
            <p class="text-sm text-base-content/60">
              {{ $t('calendars.updated') }}: <span class="font-semibold">{{ formatDate(calendar.updated_at) }}</span>
            </p>
          </div>
          <div class="mt-3 flex items-center gap-4">
            <div class="flex items-center gap-2">
              <svg xmlns="http://www.w3.org/2000/svg" width="22" height="22" fill="currentColor" class="text-blue-500" viewBox="0 0 16 16">
                <path d="M2.5 13.5A.5.5 0 0 1 3 13h10a.5.5 0 0 1 0 1H3a.5.5 0 0 1-.5-.5M13.991 3l.024.001a1.5 1.5 0 0 1 .538.143.76.76 0 0 1 .302.254c.067.1.145.277.145.602v5.991l-.001.024a1.5 1.5 0 0 1-.143.538.76.76 0 0 1-.254.302c-.1.067-.277.145-.602.145H2.009l-.024-.001a1.5 1.5 0 0 1-.538-.143.76.76 0 0 1-.302-.254C1.078 10.502 1 10.325 1 10V4.009l.001-.024a1.5 1.5 0 0 1 .143-.538.76.76 0 0 1 .254-.302C1.498 3.078 1.675 3 2 3zM14 2H2C0 2 0 4 0 4v6c0 2 2 2 2 2h12c2 0 2-2 2-2V4c0-2-2-2-2-2"/>
              </svg>
              <span class="text-sm">{{ calendar.item_count }}</span>
            </div>
          </div>
          <div class="card-actions justify-end mt-4">
            <div class="dropdown dropdown-end">
              <div tabindex="0" role="button" class="btn btn-sm btn-success">
                {{ $t('calendars.export') }}
                <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" fill="currentColor" viewBox="0 0 16 16">
                  <path d="M7.247 11.14 2.451 5.658C1.885 5.013 2.345 4 3.204 4h9.592a1 1 0 0 1 .753 1.659l-4.796 5.48a1 1 0 0 1-1.506 0z"/>
                </svg>
              </div>
              <ul tabindex="0" class="dropdown-content menu bg-base-100 rounded-box z-10 w-52 p-2 shadow-lg border border-base-200">
                <li>
                  <button @click.stop="exportCalendar(calendar.id)">{{ $t('calendars.exportDownload') }}</button>
                </li>
                <li>
                  <button @click.stop="copySubscriptionLink(calendar.subscription_token)">{{ $t('calendars.copyLink') }}</button>
                </li>
                <li>
                  <button @click.stop="openInGoogleCalendar(calendar.subscription_token)">{{ $t('calendars.openInGoogle') }}</button>
                </li>
              </ul>
            </div>
            <button class="btn btn-sm btn-primary" @click.stop="editCalendar(calendar.id)">
              {{ $t('calendars.edit') }}
            </button>
            <button class="btn btn-sm btn-error" @click.stop="confirmDelete(calendar.id)">
              {{ $t('calendars.delete') }}
            </button>
          </div>
        </div>
      </div>
    </div>

    <PaginationControls
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
import { ref, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import type { PageCalendar } from '@/types/calendar'
import api, { getApiUrl } from '@/config/api'
import { toastService } from '@/services/toastService'
import PaginationControls from '@/components/shared/PaginationControls.vue'
import ConfirmModal from '@/components/shared/ConfirmModal.vue'

const { t } = useI18n()
const router = useRouter()

const calendars = ref<PageCalendar[]>([])
const loading = ref(false)
const error = ref<string | null>(null)
const pagination = ref({ page: 1, page_size: 6, total: 0, total_pages: 0 })
const confirmModalOpen = ref(false)
const calendarToDelete = ref<number | null>(null)

onMounted(() => loadCalendars(1))

const loadCalendars = async (page: number = 1) => {
  error.value = null
  const loadingTimer = setTimeout(() => {
    loading.value = true
  }, 150)
  try {
    const response = await api.get('/calendars', { params: { page, page_size: pagination.value.page_size } })
    calendars.value = response.data.data
    pagination.value = response.data.pagination
  } catch {
    error.value = t('calendars.loadingFailed')
  } finally {
    clearTimeout(loadingTimer)
    loading.value = false
  }
}

const createNewCalendar = () => router.push('/calendar/new')
const editCalendar = (id: number) => router.push(`/calendar/${id}`)

const confirmDelete = (id: number) => {
  calendarToDelete.value = id
  confirmModalOpen.value = true
}

const executeDelete = async () => {
  confirmModalOpen.value = false
  if (calendarToDelete.value === null) return
  try {
    await api.delete(`/calendars/${calendarToDelete.value}`)
    await loadCalendars(pagination.value.page)
  } catch {
    error.value = t('calendars.deleteFailed')
  } finally {
    calendarToDelete.value = null
  }
}

const formatDate = (dateString: string) => new Date(dateString).toLocaleDateString()

const copySubscriptionLink = async (token: string) => {
  try {
    await navigator.clipboard.writeText(getApiUrl(`/calendars/subscribe/${token}`))
    toastService.success(t('calendars.linkCopied'))
  } catch {
    error.value = t('errors.generic')
  }
}

const openInGoogleCalendar = (token: string) => {
  const icalUrl = getApiUrl(`/calendars/subscribe/${token}`)
  window.open(
    `https://calendar.google.com/calendar/r?cid=${encodeURIComponent(icalUrl)}`,
    '_blank',
    'noopener,noreferrer',
  )
}

const exportCalendar = async (id: number) => {
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

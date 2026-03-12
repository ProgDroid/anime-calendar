<template>
  <div class="min-h-[calc(100vh-6.1rem)] bg-base-200 p-4">
    <h1 class="text-2xl font-bold mb-6">{{ $t('calendars.title') }}</h1>
    
    <div class="flex justify-center mb-6">
      <button @click="createNewCalendar" class="btn btn-primary">
        {{ $t('calendars.createNew') }}
      </button>
    </div>
    
    <div v-if="loading" class="alert alert-info">
      {{ $t('calendars.loading') }}
    </div>
    
    <div v-else-if="error" class="alert alert-error">
      {{ error }}
    </div>
    
    <div v-else-if="calendars.length === 0" class="alert alert-info">
      {{ $t('calendars.notFound') }} <button @click="createNewCalendar" class="btn btn-sm btn-primary">{{ $t('calendars.createNew') }}</button>
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
            <p class="text-sm text-gray">{{ $t('calendars.created') }}: <span class="font-semibold">{{ formatDate(calendar.created_at) }}</span></p>
            <p class="text-sm text-gray">{{ $t('calendars.updated') }}: <span class="font-semibold">{{ formatDate(calendar.updated_at) }}</span></p>
          </div>
          
          <!-- Icon indicators with counts for Anime and Manga -->
          <div class="mt-3 flex items-center gap-4">
            <div class="flex items-center gap-2">
              <svg xmlns="http://www.w3.org/2000/svg" width="22" height="22" fill="currentColor" class="text-blue-500" viewBox="0 0 16 16">
                <path d="M2.5 13.5A.5.5 0 0 1 3 13h10a.5.5 0 0 1 0 1H3a.5.5 0 0 1-.5-.5M13.991 3l.024.001a1.5 1.5 0 0 1 .538.143.76.76 0 0 1 .302.254c.067.1.145.277.145.602v5.991l-.001.024a1.5 1.5 0 0 1-.143.538.76.76 0 0 1-.254.302c-.1.067-.277.145-.602.145H2.009l-.024-.001a1.5 1.5 0 0 1-.538-.143.76.76 0 0 1-.302-.254C1.078 10.502 1 10.325 1 10V4.009l.001-.024a1.5 1.5 0 0 1 .143-.538.76.76 0 0 1 .254-.302C1.498 3.078 1.675 3 2 3zM14 2H2C0 2 0 4 0 4v6c0 2 2 2 2 2h12c2 0 2-2 2-2V4c0-2-2-2-2-2"/>
              </svg>
              <span class="text-sm">{{ calendar.item_count }}</span>
            </div>
          </div>
          
          <div class="card-actions justify-end mt-4">
            <button @click.stop="exportCalendar(calendar.id)" class="btn btn-sm btn-success">
              {{ $t('calendars.export') }}
            </button>
            <button @click.stop="editCalendar(calendar.id)" class="btn btn-sm btn-primary">
              {{ $t('calendars.edit') }}
            </button>
            <button @click.stop="deleteCalendar(calendar.id)" class="btn btn-sm btn-error">
              {{ $t('calendars.delete') }}
            </button>
          </div>
        </div>
      </div>
    </div>
    
    <!-- Pagination Controls -->
    <div v-if="pagination.total_pages > 1" class="join mt-8 flex justify-center">
      <button 
        @click="onPageChange(pagination.page - 1)" 
        :disabled="pagination.page === 1"
        class="join-item btn"
      >
        {{ $t('calendars.pagePrevious') }}
      </button>
      
      <button 
        v-for="page in getPaginationRange()" 
        :key="page"
        @click="onPageChange(page)"
        :class="{
          'join-item btn btn-primary': page === pagination.page,
          'join-item btn': page !== pagination.page
        }"
      >
        {{ page }}
      </button>
      
      <button 
        @click="onPageChange(pagination.page + 1)" 
        :disabled="pagination.page === pagination.total_pages"
        class="join-item btn"
      >
        {{ $t('calendars.pageNext') }}
      </button>
    </div>
    
    <div v-if="pagination.total_pages > 1" class="text-center mt-4 text-sm text-gray">
      {{ $t('calendars.paginationText', {first: pagination.page_size * (pagination.page - 1) + 1, last: pagination.page_size * pagination.page, total: pagination.total }) }}
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, watch } from 'vue'
import { useRouter } from 'vue-router'
import type { PageCalendar } from '@/types/calendar'
import api from '@/config/api'
import { useUserSettingsStore } from '@/stores/userSettingsStore'
import { i18n } from '@/plugins/i18n'

const { t } = i18n.global

// State
const calendars = ref<PageCalendar[]>([])
const loading = ref(false)
const error = ref<string | null>(null)
const pagination = ref({
  page: 1,
  page_size: 6,
  total: 0,
  total_pages: 0
})
const userSettingsStore = useUserSettingsStore()

// Router
const router = useRouter()

// Load calendars on component mount
onMounted(() => {
  loadCalendars()
})

// Load calendars from API
const loadCalendars = async (page: number = 1) => {
  loading.value = true
  error.value = null
  
  try {
    const response = await api.get('/calendars', {
      params: {
        page,
        page_size: pagination.value.page_size
      }
    })
    
    calendars.value = response.data.data
    pagination.value = {
      page: response.data.pagination.page,
      page_size: response.data.pagination.page_size,
      total: response.data.pagination.total,
      total_pages: response.data.pagination.total_pages
    }
  } catch (err) {
    error.value = t('calendars.loadingFailed')
  } finally {
    loading.value = false
  }
}

// Handle page change
const onPageChange = (newPage: number) => {
  if (newPage >= 1 && newPage <= pagination.value.total_pages) {
    loadCalendars(newPage)
  }
}

// Create a new calendar
const createNewCalendar = () => {
  router.push('/calendar/new')
}

// Edit a calendar
const editCalendar = (id: number) => {
  router.push(`/calendar/${id}`)
}

// Delete a calendar
const deleteCalendar = async (id: number) => {
  if (!confirm('Are you sure you want to delete this calendar?')) {
    return
  }
  
  try {
    await api.delete(`/calendars/${id}`)
    // Refresh the list
    await loadCalendars()
  } catch (err) {
    error.value = t('calendars.deleteFailed')
  }
}

// Format date for display
const formatDate = (dateString: string) => {
  const date = new Date(dateString)
  return date.toLocaleDateString()
}

// Export a calendar
const exportCalendar = async (id: number) => {
  try {
    const response = await api.get(`/calendar/${id}/export`, {
      responseType: 'text'
    })
    
    // Create a download link for the ICS file
    const blob = new Blob([response.data], { type: 'text/calendar' })
    const url = window.URL.createObjectURL(blob)
    const link = document.createElement('a')
    link.href = url
    link.setAttribute('download', `calendar-${id}.ics`)
    document.body.appendChild(link)
    link.click()
    document.body.removeChild(link)
    window.URL.revokeObjectURL(url)
  } catch (err) {
    console.error('Failed to export calendar:', err)
    // Optionally show an error message to the user
  }
}

// Get pagination range for display
const getPaginationRange = () => {
  const range = []
  const delta = 2 // Number of pages to show around current page
  const start = Math.max(1, pagination.value.page - delta)
  const end = Math.min(pagination.value.total_pages, pagination.value.page + delta)
  
  for (let i = start; i <= end; i++) {
    range.push(i)
  }
  
  return range
}
</script>

<style scoped>
@media (max-width: 768px) {
  .my-calendars-page {
    padding: 10px;
  }
  
  .calendars-grid {
    grid-template-columns: 1fr;
  }
}
</style>

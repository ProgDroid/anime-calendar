<template>
  <div class="min-h-screen bg-base-200 p-4">
    <h1 class="text-2xl font-bold mb-6">My Calendars</h1>
    
    <div class="flex justify-center mb-6">
      <button @click="createNewCalendar" class="btn btn-primary">
        Create New Calendar
      </button>
    </div>
    
    <div v-if="loading" class="alert alert-info">
      Loading calendars...
    </div>
    
    <div v-else-if="error" class="alert alert-error">
      {{ error }}
    </div>
    
    <div v-else-if="calendars.length === 0" class="alert alert-info">
      No calendars found. <button @click="createNewCalendar" class="btn btn-sm btn-primary">Create your first calendar</button>
    </div>
    
    <div v-else class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
      <div 
        v-for="calendar in calendars" 
        :key="calendar.id" 
        class="card bg-base-100 shadow-md hover:shadow-lg transition-shadow duration-300 cursor-pointer"
        @click="viewCalendar(calendar.id)"
      >
        <div class="card-body">
          <h2 class="card-title">{{ calendar.name }}</h2>
          <p class="text-sm">Items: {{ calendar.items.length }}</p>
          <p class="text-sm">Created: {{ formatDate(calendar.created_at) }}</p>
          <p class="text-sm">Updated: {{ formatDate(calendar.updated_at) }}</p>
          
          <!-- Cascade of cover images for first 3 items -->
          <div class="calendar-cascade mt-3">
            <CalendarCoverCascade :items="calendar.items.slice(0, 3)" />
          </div>
          
          <div class="card-actions justify-end mt-4">
            <button @click.stop="editCalendar(calendar.id)" class="btn btn-sm btn-primary">
              Edit
            </button>
            <button @click.stop="deleteCalendar(calendar.id)" class="btn btn-sm btn-error">
              Delete
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import type { Calendar } from '@/types/calendar'
import api from '@/config/api'
import CalendarCoverCascade from './CalendarCoverCascade.vue'

// State
const calendars = ref<Calendar[]>([])
const loading = ref(false)
const error = ref<string | null>(null)

// Router
const router = useRouter()

// Load calendars on component mount
onMounted(() => {
  loadCalendars()
})

// Load calendars from API
const loadCalendars = async () => {
  loading.value = true
  error.value = null
  
  try {
    const response = await api.get('/calendars')
    calendars.value = response.data
  } catch (err) {
    error.value = err instanceof Error ? err.message : 'Failed to load calendars'
  } finally {
    loading.value = false
  }
}

// Create a new calendar
const createNewCalendar = () => {
  router.push('/calendar/new')
}

// View a calendar
const viewCalendar = (id: number) => {
  router.push(`/calendar/${id}`)
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
    error.value = err instanceof Error ? err.message : 'Failed to delete calendar'
  }
}

// Format date for display
const formatDate = (dateString: string) => {
  const date = new Date(dateString)
  return date.toLocaleDateString()
}
</script>

<style scoped>
.calendar-card {
  background-color: #f8f9fa;
  padding: 20px;
  border-radius: 8px;
  border: 1px solid #ddd;
  cursor: pointer;
  transition: all 0.2s ease;
  position: relative;
}

.calendar-card:hover {
  box-shadow: 0 4px 8px rgba(0,0,0,0.1);
  transform: translateY(-2px);
}

.calendar-card h2 {
  margin-top: 0;
  color: #333;
}

.calendar-card p {
  margin: 5px 0;
  color: #666;
}

.calendar-cascade {
  margin: 15px 0;
  display: flex;
  justify-content: center;
}

.calendar-actions {
  margin-top: 15px;
  display: flex;
  gap: 10px;
}

.edit-button, .delete-button {
  padding: 8px 12px;
  border: none;
  border-radius: 4px;
  cursor: pointer;
  font-size: 0.9em;
}

.edit-button {
  background-color: #007bff;
  color: white;
}

.delete-button {
  background-color: #dc3545;
  color: white;
}

@media (max-width: 768px) {
  .my-calendars-page {
    padding: 10px;
  }
  
  .calendars-grid {
    grid-template-columns: 1fr;
  }
}
</style>

<template>
  <div class="my-calendars-page">
    <h1>My Calendars</h1>
    
    <div class="actions">
      <button @click="createNewCalendar" class="new-calendar-button">
        Create New Calendar
      </button>
    </div>
    
    <div v-if="loading" class="loading">Loading calendars...</div>
    
    <div v-else-if="error" class="error">{{ error }}</div>
    
    <div v-else-if="calendars.length === 0" class="no-calendars">
      No calendars found. <button @click="createNewCalendar">Create your first calendar</button>
    </div>
    
    <div v-else class="calendars-grid">
      <div 
        v-for="calendar in calendars" 
        :key="calendar.id" 
        class="calendar-card"
        @click="viewCalendar(calendar.id)"
      >
        <h2>{{ calendar.name }}</h2>
        <p>Items: {{ calendar.items.length }}</p>
        <p>Created: {{ formatDate(calendar.created_at) }}</p>
        <p>Updated: {{ formatDate(calendar.updated_at) }}</p>
        <div class="calendar-actions">
          <button @click.stop="editCalendar(calendar.id)" class="edit-button">Edit</button>
          <button @click.stop="deleteCalendar(calendar.id)" class="delete-button">Delete</button>
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
    await api.delete(`/calendar/${id}`)
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
.my-calendars-page {
  max-width: 1200px;
  margin: 0 auto;
  padding: 20px;
}

.actions {
  margin-bottom: 20px;
}

.new-calendar-button {
  padding: 10px 15px;
  background-color: #28a745;
  color: white;
  border: none;
  border-radius: 4px;
  cursor: pointer;
  font-size: 1em;
}

.new-calendar-button:hover {
  background-color: #218838;
}

.loading, .error, .no-calendars {
  text-align: center;
  padding: 20px;
  margin: 20px 0;
}

.error {
  color: #dc3545;
  background-color: #f8d7da;
  border: 1px solid #f5c6cb;
  border-radius: 4px;
}

.calendars-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
  gap: 20px;
  margin-top: 20px;
}

.calendar-card {
  background-color: #f8f9fa;
  padding: 20px;
  border-radius: 8px;
  border: 1px solid #ddd;
  cursor: pointer;
  transition: all 0.2s ease;
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

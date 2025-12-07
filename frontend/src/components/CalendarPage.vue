<script setup lang="ts">
import { ref } from 'vue'
import type { Item } from '@/types/item'
import type { Calendar } from '@/types/calendar'
import { getApiUrl } from '@/config/api'

// State
const idInput = ref('')
const fetchedItem = ref<Item | null>(null)
const calendarName = ref('')
const calendarLanguage = ref<'english' | 'romaji' | 'native'>('english')
const itemsInCalendar = ref<Item[]>([])
const loading = ref(false)
const error = ref<string | null>(null)

// Fetch item by ID
const fetchItem = async () => {
  if (!idInput.value) {
    error.value = 'Please enter an ID'
    return
  }

  loading.value = true
  error.value = null
  
  try {
    const response = await fetch(getApiUrl(`/item/${idInput.value}`))
    if (!response.ok) {
      throw new Error(`Failed to fetch item: ${response.status} ${response.statusText}`)
    }
    const item: Item = await response.json()
    fetchedItem.value = item
  } catch (err) {
    error.value = err instanceof Error ? err.message : 'Failed to fetch item'
  } finally {
    loading.value = false
  }
}

// Add item to calendar
const addItemToCalendar = () => {
  if (fetchedItem.value) {
    itemsInCalendar.value.push(fetchedItem.value)
    fetchedItem.value = null
    idInput.value = ''
  }
}

// Submit calendar
const submitCalendar = async () => {
  if (!calendarName.value) {
    error.value = 'Please enter a name for the calendar'
    return
  }

  if (itemsInCalendar.value.length === 0) {
    error.value = 'Please add at least one item to the calendar'
    return
  }

  loading.value = true
  error.value = null

  try {
    // Create calendar object to send to API
    const calendar: Calendar = {
      id: 0, // Will be set by the server
      name: calendarName.value,
      language: calendarLanguage.value,
      items: itemsInCalendar.value
    }
    
    const response = await fetch(getApiUrl('/calendar'), {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json'
      },
      body: JSON.stringify(calendar)
    })
    
    if (!response.ok) {
      throw new Error(`Failed to submit calendar: ${response.status} ${response.statusText}`)
    }
    
    const result = await response.json()
    alert(`Calendar submitted successfully: ${result.name}`)
    
    // Reset form
    calendarName.value = ''
    itemsInCalendar.value = []
  } catch (err) {
    error.value = err instanceof Error ? err.message : 'Failed to submit calendar'
  } finally {
    loading.value = false
  }
}
</script>

<template>
  <div class="calendar-page">
    <h1>Calendar Manager</h1>
    
    <div class="section">
      <h2>Fetch Item</h2>
      <div class="input-group">
        <label for="idInput">Item ID:</label>
        <input 
          id="idInput" 
          v-model="idInput" 
          type="number" 
          placeholder="Enter item ID"
        />
        <button @click="fetchItem" :disabled="loading">
          {{ loading ? 'Fetching...' : 'Fetch Item' }}
        </button>
      </div>
      
      <div v-if="error" class="error">{{ error }}</div>
      
      <div v-if="fetchedItem" class="item-preview">
        <h3>Fetched Item</h3>
        <div class="item-details">
          <p><strong>ID:</strong> {{ fetchedItem.id }}</p>
          <p><strong>Title:</strong> {{ fetchedItem.title.romaji }}</p>
          <p><strong>Type:</strong> {{ fetchedItem.media_type }}</p>
          <p><strong>Episodes:</strong> {{ fetchedItem.episode_duration }}</p>
        </div>
        <button @click="addItemToCalendar">Add to Calendar</button>
      </div>
    </div>

    <div class="section">
      <h2>Calendar</h2>
      <div class="form-group">
        <label for="calendarName">Calendar Name:</label>
        <input 
          id="calendarName" 
          v-model="calendarName" 
          type="text" 
          placeholder="Enter calendar name"
        />
      </div>

      <div class="form-group">
        <label>Language:</label>
        <div class="radio-group">
          <label>
            <input 
              v-model="calendarLanguage" 
              type="radio" 
              value="english" 
            />
            English
          </label>
          <label>
            <input 
              v-model="calendarLanguage" 
              type="radio" 
              value="romaji" 
            />
            Romaji
          </label>
          <label>
            <input 
              v-model="calendarLanguage" 
              type="radio" 
              value="native" 
            />
            Native
          </label>
        </div>
      </div>

      <div v-if="itemsInCalendar.length > 0" class="items-grid">
        <h3>Items in Calendar</h3>
        <div class="grid">
          <div 
            v-for="item in itemsInCalendar" 
            :key="item.id" 
            class="item-card"
          >
            <p><strong>{{ item.title.romaji }}</strong></p>
            <p>Type: {{ item.media_type }}</p>
            <p>Episodes: {{ item.episode_duration }}</p>
          </div>
        </div>
      </div>

      <button @click="submitCalendar" :disabled="loading || itemsInCalendar.length === 0">
        {{ loading ? 'Submitting...' : 'Submit Calendar' }}
      </button>
    </div>
  </div>
</template>

<style scoped>
.calendar-page {
  max-width: 800px;
  margin: 0 auto;
  padding: 20px;
}

.section {
  background: #f5f5f5;
  padding: 20px;
  margin-bottom: 20px;
  border-radius: 8px;
}

.section h2 {
  margin-top: 0;
  color: #333;
}

.input-group {
  margin-bottom: 15px;
}

.input-group label {
  display: block;
  margin-bottom: 5px;
  font-weight: bold;
}

.input-group input {
  width: 100%;
  padding: 8px;
  margin-bottom: 10px;
  border: 1px solid #ddd;
  border-radius: 4px;
}

button {
  padding: 10px 15px;
  background-color: #007bff;
  color: white;
  border: none;
  border-radius: 4px;
  cursor: pointer;
}

button:disabled {
  background-color: #ccc;
  cursor: not-allowed;
}

.error {
  color: #dc3545;
  padding: 10px;
  background-color: #f8d7da;
  border: 1px solid #f5c6cb;
  border-radius: 4px;
  margin-bottom: 15px;
}

.item-preview {
  background-color: #e9f7ef;
  padding: 15px;
  border-radius: 4px;
  margin-top: 15px;
}

.item-details p {
  margin: 5px 0;
}

.radio-group {
  display: flex;
  gap: 15px;
  margin-top: 10px;
}

.radio-group label {
  display: flex;
  align-items: center;
  gap: 5px;
}

.items-grid {
  margin-top: 20px;
}

.grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
  gap: 15px;
  margin-top: 15px;
}

.item-card {
  background-color: white;
  padding: 15px;
  border-radius: 8px;
  box-shadow: 0 2px 4px rgba(0,0,0,0.1);
}

.item-card p {
  margin: 5px 0;
}
</style>

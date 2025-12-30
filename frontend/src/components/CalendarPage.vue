<template>
  <div class="calendar-page">
    <h1>Calendar Manager</h1>
    
    <!-- Vertical Layout (for mobile) -->
    <div v-if="isMobile" class="vertical-layout">
      <div class="section">
        <h2>Search</h2>
        <div class="input-group">
          <label for="nameInput">Item Name:</label>
          <input 
            id="nameInput" 
            v-model="nameInput" 
            type="text" 
            placeholder="Enter item name"
          />
        </div>
        
        <div class="input-group">
          <label for="mediaType">Media Type:</label>
          <div class="radio-group">
            <label>
              <input 
                v-model="mediaType" 
                type="radio" 
                value="" 
              />
              Any
            </label>
            <label>
              <input 
                v-model="mediaType" 
                type="radio" 
                value="ANIME" 
              />
              Anime
            </label>
            <label>
              <input 
                v-model="mediaType" 
                type="radio" 
                value="MANGA" 
              />
              Manga
            </label>
          </div>
        </div>
        
        <button @click="fetchItems" :disabled="loading">
          {{ loading ? 'Fetching...' : 'Fetch Items' }}
        </button>
        
        <div v-if="error" class="error">{{ error }}</div>
        
        <div v-if="fetchedItems.length > 0" class="items-preview">
          <h3>Fetched Items</h3>
          <div class="scrollable-items">
            <div class="items-grid">
              <div 
                v-for="item in fetchedItems" 
                :key="item.id" 
                class="item-card"
                :class="{ 
                  selected: selectedItems.includes(item.id),
                  'already-in-calendar': itemsInCalendar.some(calendarItem => calendarItem.id === item.id)
                }"
                @click="!itemsInCalendar.some(calendarItem => calendarItem.id === item.id) && toggleItemSelection(item.id)"
              >
                <div class="item-header">
                  <div class="item-image-container" v-if="item.cover_image?.medium">
                    <img 
                      :src="item.cover_image.medium" 
                      :alt="item.title.romaji" 
                      class="item-image"
                      @error="onImageError"
                      @load="onImageLoad"
                    />
                  </div>
                  <p><strong>{{ getSelectedItemTitle(item) }}</strong></p>
                </div>
                <div class="item-type-pill">{{ item.media_type }}</div>
                <p v-if="item.media_type === 'ANIME'">Episodes: {{ item.episode_duration }}</p>
                <div v-if="itemsInCalendar.some(calendarItem => calendarItem.id === item.id)" class="already-in-calendar-hint">
                  Already in calendar
                </div>
              </div>
            </div>
          </div>
          <button @click="addItemToCalendar" :disabled="selectedItems.length === 0" class="add-button">
            Add Selected Items to Calendar
          </button>
        </div>
      </div>

      <div class="section">
        <h2>Calendar</h2>
        <div class="input-group">
          <label for="calendarName">Calendar Name:</label>
          <input 
            id="calendarName" 
            v-model="calendarName" 
            type="text" 
            placeholder="Enter calendar name"
          />
        </div>

        <div class="input-group">
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
              <div class="item-header">
                <div class="item-image-container" v-if="item.cover_image?.medium">
                  <img 
                    :src="item.cover_image.medium" 
                    :alt="item.title.romaji" 
                    class="item-image"
                    @error="onImageError"
                    @load="onImageLoad"
                  />
                </div>
                <p><strong>{{ getSelectedItemTitle(item) }}</strong></p>
              </div>
              <div class="item-type-pill">{{ item.media_type }}</div>
              <p v-if="item.media_type === 'ANIME'">Episodes: {{ item.episode_duration }}</p>
              <button @click="removeItemFromCalendar(item.id)" class="remove-button">Remove</button>
            </div>
          </div>
          <button @click="clearCalendar" class="clear-button" :disabled="itemsInCalendar.length === 0">Clear</button>
        </div>

        <button @click="submitCalendar" :disabled="loading || itemsInCalendar.length === 0">
          {{ loading ? 'Submitting...' : 'Submit Calendar' }}
        </button>
      </div>
    </div>

    <!-- TODO things don't align it's stupid -->

    <!-- Horizontal Cards Layout (for desktop/tablet) -->
    <div v-else class="horizontal-cards-layout">
      <div class="layout-container">
        <!-- Calendar Block -->
        <div class="block">
          <div class="block-form">
            <h2>Calendar</h2>
            <div class="input-group">
              <label for="calendarName">Calendar Name:</label>
              <input 
                id="calendarName" 
                v-model="calendarName" 
                type="text" 
                placeholder="Enter calendar name"
              />
            </div>

            <div class="input-group">
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

            <button @click="submitCalendar" :disabled="loading || itemsInCalendar.length === 0">
              {{ loading ? 'Submitting...' : 'Submit Calendar' }}
            </button>
          </div>
          
          <div class="block-items">
            <h3>Items in Calendar</h3>
            <div class="items-container">
              <div v-if="itemsInCalendar.length > 0" class="scrollable-items">
                <div class="items-list">
                  <div 
                    v-for="item in itemsInCalendar" 
                    :key="item.id" 
                    class="item-card"
                  >
                    <div class="item-header">
                      <div class="item-image-container" v-if="item.cover_image?.medium">
                        <img 
                          :src="item.cover_image.medium" 
                          :alt="item.title.romaji" 
                          class="item-image"
                          @error="onImageError"
                          @load="onImageLoad"
                        />
                      </div>
                      <p><strong>{{ getSelectedItemTitle(item) }}</strong></p>
                    </div>
                    <div class="item-type-pill">{{ item.media_type }}</div>
                    <p v-if="item.media_type === 'ANIME'">Episodes: {{ item.episode_duration }}</p>
                    <button @click="removeItemFromCalendar(item.id)" class="remove-button">Remove</button>
                  </div>
                </div>
              </div>
              <div v-else>
                <p>No items in calendar</p>
              </div>
              <button @click="clearCalendar" class="clear-button" :disabled="itemsInCalendar.length === 0">Clear</button>
            </div>
          </div>
        </div>

        <!-- Search Block -->
        <div class="block">
          <div class="block-form">
            <h2>Search</h2>
            <div class="input-group">
              <label for="nameInput">Item Name:</label>
              <input 
                id="nameInput" 
                v-model="nameInput" 
                type="text" 
                placeholder="Enter item name"
              />
            </div>
            
            <div class="input-group">
              <label for="mediaType">Media Type:</label>
              <div class="radio-group">
                <label>
                  <input 
                    v-model="mediaType" 
                    type="radio" 
                    value="" 
                  />
                  Any
                </label>
                <label>
                  <input 
                    v-model="mediaType" 
                    type="radio" 
                    value="ANIME" 
                  />
                  Anime
                </label>
                <label>
                  <input 
                    v-model="mediaType" 
                    type="radio" 
                    value="MANGA" 
                  />
                  Manga
                </label>
              </div>
            </div>
            
            <button @click="fetchItems" :disabled="loading">
              {{ loading ? 'Fetching...' : 'Fetch Items' }}
            </button>
            
            <div v-if="error" class="error">{{ error }}</div>
          </div>
          
          <div class="block-items">
            <h3>Fetched Items</h3>
            <div class="items-container">
              <div v-if="fetchedItems.length > 0" class="scrollable-items">
                <div class="items-list">
                  <div 
                    v-for="item in fetchedItems" 
                    :key="item.id" 
                    class="item-card"
                    :class="{ 
                      selected: selectedItems.includes(item.id),
                      'already-in-calendar': itemsInCalendar.some(calendarItem => calendarItem.id === item.id)
                    }"
                    @click="!itemsInCalendar.some(calendarItem => calendarItem.id === item.id) && toggleItemSelection(item.id)"
                  >
                    <div class="item-header">
                      <div class="item-image-container" v-if="item.cover_image?.medium">
                        <img 
                          :src="item.cover_image.medium" 
                          :alt="item.title.romaji" 
                          class="item-image"
                          @error="onImageError"
                          @load="onImageLoad"
                        />
                      </div>
                      <p><strong>{{ getSelectedItemTitle(item) }}</strong></p>
                    </div>
                    <div class="item-type-pill">{{ item.media_type }}</div>
                    <p v-if="item.media_type === 'ANIME'">Episodes: {{ item.episode_duration }}</p>
                    <div v-if="itemsInCalendar.some(calendarItem => calendarItem.id === item.id)" class="already-in-calendar-hint">
                      Already in calendar
                    </div>
                  </div>
                </div>
              </div>
              <div v-else>
                <p>No items fetched</p>
              </div>
            </div>
            <div class="add-button-container">
              <button @click="addItemToCalendar" :disabled="selectedItems.length === 0" class="add-button">
                Add Selected Items to Calendar
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>

  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, watch, computed, onBeforeMount, onBeforeUnmount } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import type { Item } from '@/types/item'
import type { Calendar } from '@/types/calendar'
import api from '@/config/api'

// Router
const router = useRouter()

// State
const nameInput = ref('')
const mediaType = ref<'ANIME' | 'MANGA' | ''>('')
const fetchedItems = ref<Item[]>([])
const selectedItems = ref<number[]>([])
const calendarName = ref('')
const calendarLanguage = ref<'english' | 'romaji' | 'native'>('english')
const itemsInCalendar = ref<Item[]>([])
const loading = ref(false)
const error = ref<string | null>(null) // TODO need calendar block error and search block error

// Route
const route = useRoute()

// Helper function to get title based on selected language
const getSelectedItemTitle = (item: Item): string => {
  switch (calendarLanguage.value) {
    case 'english':
      return item.title.english.length > 0 ? item.title.english : item.title.romaji
    case 'romaji':
      return item.title.romaji
    case 'native':
      return item.title.native
    default:
      return item.title.romaji
  }
}

// Determine layout based on screen width (mobile: vertical, desktop/tablet: horizontal-cards)
const isMobile = computed(() => {
  return window.innerWidth < 768
})

// Load persisted state on component mount
onMounted(() => {
  try {
    const persistedState = sessionStorage.getItem('calendarPageState')
    if (persistedState) {
      const state = JSON.parse(persistedState)
      if (state.nameInput !== undefined) nameInput.value = state.nameInput
      if (state.mediaType !== undefined) mediaType.value = state.mediaType
      if (state.fetchedItems !== undefined) fetchedItems.value = state.fetchedItems
      if (state.selectedItems !== undefined) selectedItems.value = state.selectedItems
      if (state.itemsInCalendar !== undefined) itemsInCalendar.value = state.itemsInCalendar
      if (state.calendarName !== undefined) calendarName.value = state.calendarName
      if (state.calendarLanguage !== undefined) calendarLanguage.value = state.calendarLanguage
    }
  } catch (e) {
    console.error('Failed to restore persisted state:', e)
  }
})

// Watch for changes and save state to sessionStorage
watch([nameInput, mediaType, fetchedItems, selectedItems, itemsInCalendar, calendarName, calendarLanguage], () => {
  try {
    const state = {
      nameInput: nameInput.value,
      mediaType: mediaType.value,
      fetchedItems: fetchedItems.value,
      selectedItems: selectedItems.value,
      itemsInCalendar: itemsInCalendar.value,
      calendarName: calendarName.value,
      calendarLanguage: calendarLanguage.value
    }
    sessionStorage.setItem('calendarPageState', JSON.stringify(state))
  } catch (e) {
    console.error('Failed to save state:', e)
  }
})

// Clear persisted state when navigating away from the page (but not on refresh)
onBeforeUnmount(() => {
  // Only clear the state if we're not editing an existing calendar
  // If we're editing an existing calendar, we want to preserve the state
  // If we're creating a new calendar, we want to clear it when navigating away
  if (route.params.id && route.params.id !== 'new') {
    // We're editing an existing calendar, so don't clear the state
    // This allows the user to refresh the page and retain their data
  } else {
    // We're creating a new calendar, so clear the state when navigating away
    sessionStorage.removeItem('calendarPageState')
  }
})

// Load calendar data when in edit mode
  const currentCalendar = ref<Calendar | null>(null)
  
  onBeforeMount(async () => {
    // Check if we're in edit mode (route contains calendar ID)
    const calendarId = Array.isArray(route.params.id) ? route.params.id[0] : route.params.id
    if (calendarId && calendarId !== 'new') {
      loading.value = true
      error.value = null
      
      try {
        const response = await api.get(`/calendars/${route.params.id}`)
        const calendar: Calendar = response.data
        
        // Pre-populate form with calendar data
        calendarName.value = calendar.name
        calendarLanguage.value = calendar.language
        
        // Load items into the calendar
        itemsInCalendar.value = calendar.items
        currentCalendar.value = calendar
      } catch (err) {
        error.value = err instanceof Error ? err.message : 'Failed to load calendar'
        console.error('Failed to load calendar:', err)
      } finally {
        loading.value = false
      }
    }
  })

// Fetch items by name
const fetchItems = async () => {
  if (!nameInput.value) {
    error.value = 'Please enter a name'
    return
  }

  loading.value = true
  error.value = null
  
  try {
    // Build search URL with optional media type parameter
    let url = `/search?name=${encodeURIComponent(nameInput.value)}`
    if (mediaType.value) {
      url += `&media_type=${mediaType.value}`
    }
    
    const response = await api.get(url)
    const items: Item[] = response.data
    fetchedItems.value = items
    selectedItems.value = [] // Reset selection when new search is performed
  } catch (err) {
    error.value = err instanceof Error ? err.message : 'Failed to fetch items'
  } finally {
    loading.value = false
  }
}

// Add item to calendar
const addItemToCalendar = () => {
  if (selectedItems.value.length > 0) {
    const selectedItemsData = fetchedItems.value.filter(item => selectedItems.value.includes(item.id))
    
    // Filter out items that are already in the calendar
    const newItems = selectedItemsData.filter(item => 
      !itemsInCalendar.value.some(calendarItem => calendarItem.id === item.id)
    )
    
    itemsInCalendar.value.push(...newItems)
    selectedItems.value = []
    nameInput.value = ''
  }
}

// Remove item from calendar
const removeItemFromCalendar = (itemId: number) => {
  itemsInCalendar.value = itemsInCalendar.value.filter(item => item.id !== itemId)
}

// Toggle item selection
const toggleItemSelection = (itemId: number) => {
  const index = selectedItems.value.indexOf(itemId)
  if (index > -1) {
    selectedItems.value.splice(index, 1)
  } else {
    selectedItems.value.push(itemId)
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
      let response
      
// Check if we're editing an existing calendar (has an ID)
      if (route.params.id && route.params.id !== 'new') {
        // For editing an existing calendar, we should send the items that are currently in the calendar
        const calendarId = parseInt(String(route.params.id))
        
        // When editing, we send the items that are currently in the calendar (which should include both existing and new items)
        const calendar: Omit<Calendar, 'created_at' | 'updated_at'> = {
          id: calendarId,
          name: calendarName.value,
          language: calendarLanguage.value,
          items: itemsInCalendar.value
        }

        response = await api.put('/calendar', calendar)
      } else {
        const calendar: Omit<Calendar, 'id' | 'created_at' | 'updated_at'> = {
          name: calendarName.value,
          language: calendarLanguage.value,
          items: itemsInCalendar.value
        }

        response = await api.put('/calendar', calendar)
      }
      
      alert(`Calendar submitted successfully: ${response.data.name}`)
      
      // Reset form
      calendarName.value = ''
      itemsInCalendar.value = []
      
      // Redirect to my-calendars page
      router.push('/my-calendars')
    } catch (err) {
      error.value = err instanceof Error ? err.message : 'Failed to submit calendar'
    } finally {
      loading.value = false
    }
  }

// Clear all items from calendar
const clearCalendar = () => {
  itemsInCalendar.value = []
}

const onImageError = (event: Event) => {
  const img = event.target as HTMLImageElement
  img.style.display = 'none'
  console.error('Image failed to load:', (event.target as HTMLImageElement).src)
}

const onImageLoad = (event: Event) => {
  console.log('Image loaded successfully', (event.target as HTMLImageElement).src)
}
</script>

<style scoped>
.calendar-page {
  max-width: 1200px;
  margin: 0 auto;
  padding: 20px;
}

.section {
  background: #f8f9fa;
  padding: 20px;
  margin-bottom: 10px;
  border-radius: 8px;
  border: 1px solid #ddd;
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

.remove-button {
  padding: 10px 15px;
  background-color: #dc3545;
  color: white;
  border: none;
  border-radius: 4px;
  cursor: pointer;
  margin-top: 10px;
  font-size: 0.9em;
  max-width: fit-content;
}

.clear-button {
  padding: 10px 15px;
  background-color: #6c757d;
  color: white;
  border: none;
  border-radius: 4px;
  cursor: pointer;
  margin-top: 15px;
  font-size: 1em;
  width: 100%;
  max-width: fit-content;
  align-self: flex-start;
}

.clear-button:not(:disabled) {
  background-color: #dc3545;
}

.clear-button:not(:disabled):hover {
  background-color: #c82333;
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
  transition: all 0.2s ease;
  position: relative;
  display: flex;
  flex-direction: column;
  min-height: 100px;
  overflow: hidden;
}

.item-header {
  display: flex;
  align-items: flex-start;
  width: 100%;
  margin-bottom: 5px;
  flex-direction: row;
  flex-wrap: nowrap;
  position: relative;
}

.item-image-container {
  flex-shrink: 0;
  margin-right: 10px;
  width: 60px;
  height: 80px;
  overflow: hidden;
  border-radius: 4px;
  display: flex;
  align-items: center;
  justify-content: center;
  background-color: #f0f0f0;
}

.item-image {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
}

.item-card p {
  margin: 5px 0;
  word-break: break-word;
}

.item-type-pill {
  position: absolute;
  bottom: 10px;
  right: 10px;
  margin-left: auto;
  margin-right: 0;
}

.item-type-pill {
  display: inline-block;
  background-color: #007bff;
  color: white;
  padding: 4px 8px;
  border-radius: 12px;
  font-size: 0.8em;
  margin: 5px 0;
  min-width: fit-content;
  max-width: fit-content;
}

.scrollable-items {
  max-height: 300px;
  overflow-y: auto;
  padding: 10px;
  border: 1px solid #ddd;
  border-radius: 4px;
  margin-bottom: 15px;
  box-sizing: border-box;
}

.add-button-container {
  padding: 10px;
  margin-top: 10px;
}

.add-button {
  max-width: fit-content;
}

.already-in-calendar-hint {
  position: absolute;
  top: 5px;
  right: 5px;
  background-color: #28a745;
  color: white;
  padding: 2px 6px;
  border-radius: 4px;
  font-size: 0.7em;
  font-weight: bold;
}

/* Horizontal Cards Layout Styles */
.horizontal-cards-layout {
  display: flex;
  flex-direction: column;
}

.horizontal-cards-layout .layout-container {
  display: flex;
  flex-direction: row;
  gap: 20px;
  margin-top: 20px;
}

.horizontal-cards-layout .block {
  flex: 1;
  min-width: 300px;
  background-color: #f8f9fa;
  padding: 15px;
  border-radius: 8px;
  display: flex;
  flex-direction: column;
  min-width: 0;
  border: 1px solid #ddd;
}

.horizontal-cards-layout .block-form {
  flex: 1;
  padding: 0;
  margin-bottom: 10px;
}

.horizontal-cards-layout .block-items {
  flex: 1;
  padding: 0;
  margin-bottom: 10px;
  display: flex;
  flex-direction: column;
}

.block-items > div {
  flex: 1;
  display: flex;
  flex-direction: column;
}

.scrollable-items {
  max-height: 300px;
  overflow-y: auto;
  padding: 10px;
  border: 1px solid #ddd;
  border-radius: 4px;
  margin-bottom: 15px;
  box-sizing: border-box;
  flex: 1;
  display: flex;
  flex-direction: column;
}

.scrollable-items > div {
  flex: 1;
  display: flex;
  flex-direction: column;
}

.clear-button {
  align-self: flex-start;
  margin-top: 10px;
  margin-bottom: 10px;
  flex-shrink: 0;
}

.items-list .item-card {
  min-height: auto;
  margin-bottom: 0;
  display: flex;
  flex-direction: column;
  padding: 15px;
  transition: all 0.2s ease;
  background-color: white;
  border-radius: 8px;
  box-shadow: 0 2px 4px rgba(0,0,0,0.1);
  transition: all 0.2s ease;
  position: relative;
  display: flex;
  flex-direction: column;
  border: 1px solid #ddd;
}

.item-card.selected {
  background-color: #007bff;
  color: white;
  box-shadow: 0 4px 8px rgba(0,123,255,0.3);
  transform: translateY(-2px);
}

.item-card.selected p {
  color: white;
}

.item-card.already-in-calendar {
  background-color: #f8f9fa;
  border: 2px solid #28a745;
  opacity: 0.8;
}

.item-card.already-in-calendar p {
  color: #28a745;
}

@media (max-width: 768px) {
  .layout-container {
    flex-direction: column;
  }
  
  .block {
    flex: 1;
  }
  
  .block-form, .block-items {
    flex: 1;
    padding: 0;
  }
  
  .calendar-page {
    max-width: 100%; /* Full width on mobile */
    padding: 10px;
  }
  
  .item-card {
    min-height: auto;
  }
}
</style>

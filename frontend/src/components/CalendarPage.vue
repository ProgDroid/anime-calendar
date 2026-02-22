<template>
  <div class="min-h-[calc(100vh-6.1rem)] bg-base-200 p-4">
    <h1 class="text-2xl font-bold mb-6">Edit Calendar</h1>
    
    <!-- Mobile Layout -->
    <div v-if="isMobile" class="flex flex-col gap-6">
      <!-- Search Section -->
      <div class="card bg-base-100 shadow-md">
        <div class="card-body">
          <h2 class="card-title">Search</h2>
          <div class="form-control">
            <label class="label">
              <span class="label-text">Item Name:</span>
            </label>
            <input 
              id="nameInput" 
              v-model="nameInput" 
              type="text" 
              placeholder="Enter item name"
              class="input input-bordered mt-2"
            />
          </div>
          
          <div class="form-control">
            <label class="label mb-2">
              <span class="label-text">Media Type:</span>
            </label>
            <div class="flex gap-4 mb-2">
              <label class="label cursor-pointer">
                <input 
                  v-model="mediaType" 
                  type="radio" 
                  value="" 
                  class="radio radio-primary"
                />
                <span class="label-text">Any</span>
              </label>
              <label class="label cursor-pointer">
                <input 
                  v-model="mediaType" 
                  type="radio" 
                  value="ANIME" 
                  class="radio radio-primary"
                />
                <span class="label-text">Anime</span>
              </label>
              <label class="label cursor-pointer">
                <input 
                  v-model="mediaType" 
                  type="radio" 
                  value="MANGA" 
                  class="radio radio-primary"
                />
                <span class="label-text">Manga</span>
              </label>
            </div>
          </div>
          
          <button @click="fetchItems" :disabled="loading" class="btn btn-primary w-full">
            {{ loading ? 'Fetching...' : 'Fetch Items' }}
          </button>
          
          <div v-if="searchError" class="alert alert-error mt-4">
            {{ searchError }}
          </div>
        </div>
      </div>

      <!-- Fetched Items Section -->
      <div class="card bg-base-100 shadow-md">
        <div class="card-body">
          <h3 class="font-bold mb-2">Fetched Items</h3>
          <div class="overflow-y-auto max-h-96 p-2 border rounded max-h-[250px] min-h-[250px]">
            <div class="flex flex-col gap-1">
                  <div 
                    v-for="item in fetchedItems" 
                    :key="item.id" 
                    class="card bg-base-100 shadow-sm border"
                    :class="{ 
                      'border-primary': selectedItems.includes(item.id),
                      'border-success': itemsInCalendar.some(calendarItem => calendarItem.id === item.id)
                    }"
                    @click="!itemsInCalendar.some(calendarItem => calendarItem.id === item.id) && toggleItemSelection(item.id)"
                  >
                    <div class="card-body p-3 relative overflow-hidden">
                      <div class="flex items-start gap-2">
                        <div class="flex-shrink-0">
                          <div v-if="item.cover_image?.medium" class="bg-gray-200 border rounded w-16 h-20 overflow-hidden">
                            <img 
                              :src="item.cover_image.medium" 
                              :alt="item.title.romaji" 
                              class="w-full h-full object-cover"
                              @error="onImageError"
                              @load="onImageLoad"
                            />
                          </div>
                          <div v-else class="bg-gray-200 border rounded w-16 h-20 flex items-center justify-center">
                            <span class="text-xs">No image</span>
                          </div>
                        </div>
                        <div class="flex-grow">
                          <h4 class="font-bold line-clamp-1">{{ getSelectedItemTitle(item) }}</h4>
                          <div class="badge badge-secondary mt-1">{{ item.media_type }}</div>
                          <p v-if="item.media_type === 'ANIME'" class="text-xs mt-1">Episodes: {{ item.episode_duration }}</p>
                        </div>
                      </div>
                      <div v-if="itemsInCalendar.some(calendarItem => calendarItem.id === item.id)" class="absolute top-2 right-2 bg-success text-white text-xs px-2 py-1 rounded">
                        Already in calendar
                      </div>
                      <!-- Background image for selected items -->
                      <div 
                        v-if="selectedItems.includes(item.id) && item.banner_image"
                        class="absolute inset-0 transition-all duration-300 ease-in-out"
                        :class="{ 
                          'opacity-0': !selectedItems.includes(item.id),
                          'opacity-100': selectedItems.includes(item.id)
                        }"
                        :style="{ 
                          'background-image': `url(${item.banner_image})`,
                          'background-size': 'cover',
                          'background-position': 'center',
                          'background-repeat': 'no-repeat',
                          'mask-image': 'linear-gradient(to right, rgba(0,0,0,0) 0%, rgba(0,0,0,0) 50%, rgba(0,0,0,0.2) 65%, rgba(0,0,0,1) 95%)',
                          'border-radius': '8px 8px 8px 8px'
                        }"
                      >
                      </div>
                      <div
                        v-if="!item.banner_image && item.cover_image?.medium && selectedItems.includes(item.id)"
                        class="absolute inset-0 transition-all duration-300 ease-in-out"
                        :class="{ 
                          'opacity-0': !selectedItems.includes(item.id),
                          'opacity-100': selectedItems.includes(item.id)
                        }"
                        :style="{ 
                          'background-image': `url(${item.cover_image?.medium})`,
                          'background-size': 'fit',
                          'background-position': 'right',
                          'background-repeat': 'no-repeat',
                          'mask-image': 'linear-gradient(to right, rgba(0,0,0,0) 0%, rgba(0,0,0,0.0) 83.5%, rgba(0,0,0,1) 95%)',
                          'border-radius': '8px 8px 8px 8px'
                        }"
                      >
                      </div>
                    </div>
                  </div>
            </div>
          </div>
          
          <div class="mt-4">
            <button @click="addItemToCalendar" :disabled="selectedItems.length === 0" class="btn btn-primary w-full">
              Add Selected Items to Calendar
            </button>
          </div>
        </div>
      </div>

      <!-- Calendar Section -->
      <div class="card bg-base-100 shadow-md">
        <div class="card-body">
          <h2 class="card-title">Calendar</h2>
          <div class="form-control">
            <label class="label">
              <span class="label-text">Calendar Name:</span>
            </label>
            <input 
              id="calendarName" 
              v-model="calendarName" 
              type="text" 
              placeholder="Enter calendar name"
              class="input input-bordered mt-2"
            />
          </div>

          <div class="form-control">
            <label class="label mb-2">
              <span class="label-text">Language:</span>
            </label>
            <div class="flex gap-4">
              <label class="label cursor-pointer">
                <input 
                  v-model="calendarLanguage" 
                  type="radio" 
                  value="english" 
                  class="radio radio-primary"
                />
                <span class="label-text">English</span>
              </label>
              <label class="label cursor-pointer">
                <input 
                  v-model="calendarLanguage" 
                  type="radio" 
                  value="romaji" 
                  class="radio radio-primary"
                />
                <span class="label-text">Romaji</span>
              </label>
              <label class="label cursor-pointer">
                <input 
                  v-model="calendarLanguage" 
                  type="radio" 
                  value="native" 
                  class="radio radio-primary"
                />
                <span class="label-text">Native</span>
              </label>
            </div>
          </div>

          <div v-if="calendarError" class="alert alert-error mt-4">
            {{ calendarError }}
          </div>

          <div v-if="itemsInCalendar.length > 0" class="mt-4">
            <h3 class="font-bold mb-2">Items in Calendar</h3>
            <div class="overflow-y-auto max-h-[400px] min-h-[400px] p-2 border rounded">
              <div class="flex flex-col gap-1">
                <div 
                  v-for="item in itemsInCalendar" 
                  :key="item.id" 
                  class="card bg-base-100 shadow-sm border"
                >
                  <div class="card-body p-3">
                    <div class="flex items-start gap-2">
                      <div class="flex-shrink-0">
                        <div v-if="item.cover_image?.medium" class="bg-gray-200 border rounded w-16 h-20 overflow-hidden">
                          <img 
                            :src="item.cover_image.medium" 
                            :alt="item.title.romaji" 
                            class="w-full h-full object-cover"
                            @error="onImageError"
                            @load="onImageLoad"
                          />
                        </div>
                        <div v-else class="bg-gray-200 border rounded w-16 h-20 flex items-center justify-center">
                          <span class="text-xs">No image</span>
                        </div>
                      </div>
                      <div class="flex-grow">
                        <h4 class="font-bold line-clamp-1">{{ getSelectedItemTitle(item) }}</h4>
                        <div class="badge badge-secondary mt-1">{{ item.media_type }}</div>
                        <p v-if="item.media_type === 'ANIME'" class="text-xs mt-1">Episodes: {{ item.episode_duration }}</p>
                      </div>
                    </div>
                    <button @click="removeItemFromCalendar(item.id)" class="btn btn-sm btn-error mt-2">
                      Remove
                    </button>
                  </div>
                </div>
              </div>
            </div>
            
            <!-- Recommendations Section -->
            <div v-if="recommendations.length > 0" class="mt-6">
              <h3 class="font-bold mb-2">Recommended Items</h3>
              <div class="carousel w-full gap-4">
                <div 
                  v-for="item in recommendations" 
                  :key="item.id" 
                  class="card bg-base-100 shadow-sm border carousel-item w-64"
                >
                  <div class="card-body p-3">
                    <div class="flex items-start gap-2">
                      <div class="flex-shrink-0">
                        <div v-if="item.cover_image?.medium" class="bg-gray-200 border rounded w-16 h-20 overflow-hidden">
                          <img 
                            :src="item.cover_image.medium" 
                            :alt="item.title.romaji" 
                            class="w-full h-full object-cover"
                            @error="onImageError"
                            @load="onImageLoad"
                          />
                        </div>
                        <div v-else class="bg-gray-200 border rounded w-16 h-20 flex items-center justify-center">
                          <span class="text-xs">No image</span>
                        </div>
                      </div>
                      <div class="flex-grow">
                        <h4 class="font-bold line-clamp-1 text-sm">{{ getSelectedItemTitle(item) }}</h4>
                        <div class="badge badge-secondary mt-1 text-xs">{{ item.media_type }}</div>
                        <div class="mt-1">
                          <button 
                            @click.stop="addItemToCalendarSingle(item)"
                            class="btn btn-primary btn-xs absolute bottom-2 right-2"
                          >
                            Add
                          </button>
                        </div>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </div>

            <div class="mt-4">
              <button @click="clearCalendar" class="btn btn-warning w-full" :disabled="itemsInCalendar.length === 0">
                Clear Calendar
              </button>
            </div>
          </div>

          <div class="mt-4">
            <button @click="submitCalendar" :disabled="loading || itemsInCalendar.length === 0" class="btn btn-success w-full">
              {{ loading ? 'Submitting...' : 'Submit Calendar' }}
            </button>
          </div>
        </div>
      </div>
    </div>

      <!-- Desktop/Tablet Layout -->
      <div v-else class="grid grid-cols-1 lg:grid-cols-2 gap-6 max-w-6xl mx-auto">
        <!-- Calendar Block -->
        <div class="card bg-base-100 shadow-md">
          <div class="card-body">
            <h2 class="card-title">Calendar</h2>
            <div class="form-control">
              <label class="label">
                <span class="label-text">Calendar Name:</span>
              </label>
              <input 
                id="calendarName" 
                v-model="calendarName" 
                type="text" 
                placeholder="Enter calendar name"
                class="input input-bordered ml-2"
              />
            </div>

            <div class="form-control">
              <label class="label mb-2">
                <span class="label-text">Language:</span>
              </label>
              <div class="flex gap-4">
                <label class="label cursor-pointer">
                  <input 
                    v-model="calendarLanguage" 
                    type="radio" 
                    value="english" 
                    class="radio radio-primary"
                  />
                  <span class="label-text">English</span>
                </label>
                <label class="label cursor-pointer">
                  <input 
                    v-model="calendarLanguage" 
                    type="radio" 
                    value="romaji" 
                    class="radio radio-primary"
                  />
                  <span class="label-text">Romaji</span>
                </label>
                <label class="label cursor-pointer">
                  <input 
                    v-model="calendarLanguage" 
                    type="radio" 
                    value="native" 
                    class="radio radio-primary"
                  />
                  <span class="label-text">Native</span>
                </label>
              </div>
            </div>

            <div class="mt-4">
              <button @click="submitCalendar" :disabled="loading || itemsInCalendar.length === 0" class="btn btn-success">
                {{ loading ? 'Submitting...' : 'Submit Calendar' }}
              </button>
            </div>

            <div v-if="calendarError" class="alert alert-error mt-4">
              {{ calendarError }}
            </div>
          </div>
          
          <div class="card-body">
            <div class="flex flex-col gap-4">
              <h3 class="font-bold">Items in Calendar</h3>
              <div class="overflow-y-auto max-h-[400px] min-h-[400px] p-2 border rounded">
                <div class="flex flex-col gap-1">
                  <div 
                    v-for="item in itemsInCalendar" 
                    :key="item.id" 
                    class="card bg-base-100 shadow-sm border"
                  >
                    <div class="card-body p-3">
                      <div class="flex items-start gap-2">
                        <div class="flex-shrink-0">
                          <div v-if="item.cover_image?.medium" class="bg-gray-200 border rounded w-16 h-20 overflow-hidden">
                            <img 
                              :src="item.cover_image.medium" 
                              :alt="item.title.romaji" 
                              class="w-full h-full object-cover"
                              @error="onImageError"
                              @load="onImageLoad"
                            />
                          </div>
                          <div v-else class="bg-gray-200 border rounded w-16 h-20 flex items-center justify-center">
                            <span class="text-xs">No image</span>
                          </div>
                        </div>
                        <div class="flex-grow">
                          <h4 class="font-bold line-clamp-1">{{ getSelectedItemTitle(item) }}</h4>
                          <div class="badge badge-secondary mt-1">{{ item.media_type }}</div>
                          <p v-if="item.media_type === 'ANIME'" class="text-xs mt-1">Episodes: {{ item.episode_duration }}</p>
                        </div>
                      </div>
                      <button @click="removeItemFromCalendar(item.id)" class="btn btn-sm btn-error mt-2">
                        Remove
                      </button>
                    </div>
                  </div>
                </div>
              </div>
              
              <button @click="clearCalendar" class="btn btn-warning w-full" :disabled="itemsInCalendar.length === 0">
                Clear Calendar
              </button>
            </div>
          </div>
        </div>

        <!-- Search Block -->
        <div class="card bg-base-100 shadow-md">
          <div class="card-body">
            <h2 class="card-title">Search</h2>
            <div class="form-control">
              <label class="label">
                <span class="label-text">Item Name:</span>
              </label>
              <input 
                id="nameInput" 
                v-model="nameInput" 
                type="text" 
                placeholder="Enter item name"
                class="input input-bordered ml-2"
              />
            </div>
            
            <div class="form-control">
              <label class="label mb-2">
                <span class="label-text">Media Type:</span>
              </label>
              <div class="flex gap-4">
                <label class="label cursor-pointer">
                  <input 
                    v-model="mediaType" 
                    type="radio" 
                    value="" 
                    class="radio radio-primary"
                  />
                  <span class="label-text">Any</span>
                </label>
                <label class="label cursor-pointer">
                  <input 
                    v-model="mediaType" 
                    type="radio" 
                    value="ANIME" 
                    class="radio radio-primary"
                  />
                  <span class="label-text">Anime</span>
                </label>
                <label class="label cursor-pointer">
                  <input 
                    v-model="mediaType" 
                    type="radio" 
                    value="MANGA" 
                    class="radio radio-primary"
                  />
                  <span class="label-text">Manga</span>
                </label>
              </div>
            </div>
            
            <div class="mt-4">
              <button @click="fetchItems" :disabled="loading" class="btn btn-primary">
                {{ loading ? 'Fetching...' : 'Fetch Items' }}
              </button>
            </div>
            
            <div v-if="searchError" class="alert alert-error mt-4">
              {{ searchError }}
            </div>
          </div>
          
          <div class="card-body">
            <div class="flex flex-col gap-4">
              <h3 class="font-bold">Fetched Items</h3>
              <div class="overflow-y-auto max-h-[400px] min-h-[400px] p-2 border rounded">
                <div class="flex flex-col gap-1">
                  <div 
                    v-for="item in fetchedItems" 
                    :key="item.id" 
                    class="card bg-base-100 shadow-sm border"
                    :class="{ 
                      'border-primary': selectedItems.includes(item.id),
                      'border-success': itemsInCalendar.some(calendarItem => calendarItem.id === item.id)
                    }"
                    @click="!itemsInCalendar.some(calendarItem => calendarItem.id === item.id) && toggleItemSelection(item.id)"
                  >
                    <div class="card-body p-3">
                      <div class="flex items-start gap-2">
                        <div class="flex-shrink-0">
                          <div v-if="item.cover_image?.medium" class="bg-gray-200 border rounded w-16 h-20 overflow-hidden">
                            <img 
                              :src="item.cover_image.medium" 
                              :alt="item.title.romaji" 
                              class="w-full h-full object-cover"
                              @error="onImageError"
                              @load="onImageLoad"
                            />
                          </div>
                          <div v-else class="bg-gray-200 border rounded w-16 h-20 flex items-center justify-center">
                            <span class="text-xs">No image</span>
                          </div>
                        </div>
                        <div class="flex-grow">
                          <h4 class="font-bold line-clamp-1">{{ getSelectedItemTitle(item) }}</h4>
                          <div class="badge badge-secondary mt-1">{{ item.media_type }}</div>
                          <p v-if="item.media_type === 'ANIME'" class="text-xs mt-1">Episodes: {{ item.episode_duration }}</p>
                        </div>
                      </div>
                      <div v-if="itemsInCalendar.some(calendarItem => calendarItem.id === item.id)" class="absolute top-2 right-2 bg-success text-white text-xs px-2 py-1 rounded">
                        Already in calendar
                      </div>
                      <!-- Background image for selected items -->
                      <div 
                        v-if="selectedItems.includes(item.id) && item.banner_image"
                        class="absolute inset-0 transition-all duration-300 ease-in-out"
                        :class="{ 
                          'opacity-0': !selectedItems.includes(item.id),
                          'opacity-100': selectedItems.includes(item.id)
                        }"
                        :style="{ 
                          'background-image': `url(${item.banner_image})`,
                          'background-size': 'cover',
                          'background-position': 'center',
                          'background-repeat': 'no-repeat',
                          'mask-image': 'linear-gradient(to right, rgba(0,0,0,0) 0%, rgba(0,0,0,0) 50%, rgba(0,0,0,0.2) 65%, rgba(0,0,0,1) 95%)',
                          'border-radius': '8px 8px 8px 8px'
                        }"
                      >
                      </div>
                      <div
                        v-if="!item.banner_image && item.cover_image?.medium && selectedItems.includes(item.id)"
                        class="absolute inset-0 transition-all duration-300 ease-in-out"
                        :class="{ 
                          'opacity-0': !selectedItems.includes(item.id),
                          'opacity-100': selectedItems.includes(item.id)
                        }"
                        :style="{ 
                          'background-image': `url(${item.cover_image?.medium})`,
                          'background-size': 'fit',
                          'background-position': 'right',
                          'background-repeat': 'no-repeat',
                          'mask-image': 'linear-gradient(to right, rgba(0,0,0,0) 0%, rgba(0,0,0,0.0) 83.5%, rgba(0,0,0,1) 95%)',
                          'border-radius': '8px 8px 8px 8px'
                        }"
                      >
                      </div>
                    </div>
                  </div>
                </div>
              </div>
              <button @click="addItemToCalendar" :disabled="selectedItems.length === 0" class="btn btn-primary w-full">
                Add Selected Items to Calendar
              </button>
            </div>
          </div>
        </div>
      </div>
    <div class="max-w-6xl mx-auto">
      <div v-if="!isMobile" class="card bg-base-100 shadow-md mt-4">
        <div class="card-body">
          <h2 class="card-title">Recommended Items</h2>
          <!-- Recommendations Section -->
          <div v-if="recommendations.length > 0" class="mt-4">
            <!-- DaisyUI Carousel for Desktop Layout Only -->
            <div class="hidden lg:block">
              <div class="carousel carousel-center w-full gap-1">
                <div 
                  v-for="item in recommendations" 
                  :key="item.id" 
                  class="card bg-base-100 shadow-sm border carousel-item w-[calc(20%-6px)]"
                >
                  <div class="card-body p-3">
                    <div class="flex items-start gap-2">
                      <div class="flex-shrink-0">
                        <div v-if="item.cover_image?.medium" class="bg-gray-200 border rounded w-16 h-20 overflow-hidden">
                          <img 
                            :src="item.cover_image.medium" 
                            :alt="item.title.romaji" 
                            class="w-full h-full object-cover"
                            @error="onImageError"
                            @load="onImageLoad"
                          />
                        </div>
                        <div v-else class="bg-gray-200 border rounded w-16 h-20 flex items-center justify-center">
                          <span class="text-xs">No image</span>
                        </div>
                      </div>
                      <div class="flex-grow">
                        <h4 class="font-bold line-clamp-1 text-sm">{{ getSelectedItemTitle(item) }}</h4>
                        <div class="badge badge-secondary mt-1 text-xs">{{ item.media_type }}</div>
                      </div>
                      <button 
                            @click.stop="addItemToCalendarSingle(item)"
                            class="btn btn-primary btn-xs absolute bottom-2 right-2"
                          >
                            Add
                      </button>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>
          <div v-else-if="itemsInCalendar.length > 0" class="flex justify-center items-center h-full">
            <p>No recommendations for your current selected items.</p>
          </div>
          <div v-else class="flex justify-center items-center h-full">
            <p>Add items to your calendar to see recommendations.</p>
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
import { toastService } from '@/services/toastService'
import { useUserSettingsStore } from '@/stores/userSettingsStore'

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
const searchError = ref<string | null>(null)
const calendarError = ref<string | null>(null)
const recommendations = ref<Item[]>([])
const userSettingsStore = useUserSettingsStore()

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
    calendarError.value = null
    
    try {
      const response = await api.get(`/calendars/${route.params.id}`)
      const calendar: Calendar = response.data
      
      // Pre-populate form with calendar data
      calendarName.value = calendar.name
      calendarLanguage.value = calendar.language
      
      // Load items into the calendar
      itemsInCalendar.value = calendar.items
      currentCalendar.value = calendar
      
      // Calculate recommendations when loading an existing calendar
      calculateRecommendations()
    } catch (err) {
      calendarError.value = err instanceof Error ? err.message : 'Failed to load calendar'
      console.error('Failed to load calendar:', err)
    } finally {
      loading.value = false
    }
  } else { // If new calendar, use user setting
    const settings = await userSettingsStore.fetchSettings()
    switch (settings.title_language_preference) {
      case 'English':
        calendarLanguage.value = 'english'
        break
      case 'Romaji':
        calendarLanguage.value = 'romaji'
        break
      case 'Native':
        calendarLanguage.value = 'native'
        break
      default:
        calendarLanguage.value = 'english'
    }
  }
})

// Calculate recommendations from items in calendar
const calculateRecommendations = () => {
  if (itemsInCalendar.value.length === 0) {
    recommendations.value = []
    return
  }

  // Create a map to count frequency of each recommended item
  const recommendationCounts: Map<number, { count: number; totalRating: number; item: Item }> = new Map()

  // Iterate through all items in the calendar
  itemsInCalendar.value.forEach(item => {
    if (item.recommendations && item.recommendations.length > 0) {
      item.recommendations.forEach(recommendation => {
        const mediaId = recommendation.media.id
        // Skip if this recommendation is already in the calendar
        if (!itemsInCalendar.value.some(calendarItem => calendarItem.id === mediaId)) {
          if (recommendationCounts.has(mediaId)) {
            const existing = recommendationCounts.get(mediaId)!
            existing.count += 1
            existing.totalRating += recommendation.rating
          } else {
            recommendationCounts.set(mediaId, {
              count: 1,
              totalRating: recommendation.rating,
              item: {
                id: recommendation.media.id,
                id_mal: recommendation.media.id_mal,
                title: recommendation.media.title,
                media_type: item.media_type, // Use the same type as the source item
                episode_duration: 0, // Default value, could be improved
                airing_schedule: [],
                cover_image: recommendation.media.cover_image,
                banner_image: '', // Default value
                recommendations: [] // No nested recommendations
              }
            })
          }
        }
      })
    }
  })

  // Convert map to array and sort by frequency (descending), then by average rating (descending)
  const sortedRecommendations = Array.from(recommendationCounts.values())
    .sort((a, b) => {
      // First sort by frequency (count) descending
      if (b.count !== a.count) {
        return b.count - a.count
      }
      // Then sort by average rating descending
      const avgRatingA = a.totalRating / a.count
      const avgRatingB = b.totalRating / b.count
      return avgRatingB - avgRatingA
    })
    .slice(0, 5) // Take top 5
    .map(item => item.item)

  recommendations.value = sortedRecommendations
}

// Fetch items by name
const fetchItems = async () => {
  if (!nameInput.value) {
    searchError.value = 'Please enter a name'
    return
  }

  loading.value = true
  searchError.value = null
  
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
    searchError.value = err instanceof Error ? err.message : 'Failed to fetch items'
    console.error('Failed to fetch items:', err)
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
    
    // Recalculate recommendations when items are added
    if (itemsInCalendar.value.length > 0) {
      calculateRecommendations()
    }
  }
}

// Add single item to calendar
const addItemToCalendarSingle = (item: Item) => {
  // Check if item is already in calendar
  if (itemsInCalendar.value.some(calendarItem => calendarItem.id === item.id)) {
    return
  }
  
  // Add item to calendar
  itemsInCalendar.value.push(item)
  
  // Recalculate recommendations when item is added
  if (itemsInCalendar.value.length > 0) {
    calculateRecommendations()
  }
}

// Remove item from calendar
const removeItemFromCalendar = (itemId: number) => {
  itemsInCalendar.value = itemsInCalendar.value.filter(item => item.id !== itemId)
  
  // Recalculate recommendations when items are removed
  if (itemsInCalendar.value.length > 0) {
    calculateRecommendations()
  } else {
    recommendations.value = []
  }
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
    calendarError.value = 'Please enter a name for the calendar'
    return
  }

  if (itemsInCalendar.value.length === 0) {
    calendarError.value = 'Please add at least one item to the calendar'
    return
  }

  loading.value = true
  calendarError.value = null

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
    
    toastService.success(`${response.data.name} updated`)
    
    // Reset form
    calendarName.value = ''
    itemsInCalendar.value = []
    
    // Redirect to my-calendars page
    router.push('/my-calendars')
  } catch (err) {
    calendarError.value = err instanceof Error ? err.message : 'Failed to submit calendar'
  } finally {
    loading.value = false
  }
}

// Clear all items from calendar
const clearCalendar = () => {
  itemsInCalendar.value = []
  recommendations.value = []
}

const onImageError = (event: Event) => {
  const img = event.target as HTMLImageElement
  img.style.display = 'none'
  console.error('Image failed to load:', (event.target as HTMLImageElement).src)
}

const onImageLoad = (event: Event) => {
  console.log('Image loaded successfully', (event.target as HTMLImageElement).src)
}

// Scroll recommendations carousel
const scrollRecommendations = (direction: 'left' | 'right') => {
  const carousel = document.querySelector('.overflow-x-auto')
  if (carousel) {
    const scrollAmount = 300 // Adjust scroll amount as needed
    if (direction === 'right') {
      carousel.scrollBy({ left: scrollAmount, behavior: 'smooth' })
    } else {
      carousel.scrollBy({ left: -scrollAmount, behavior: 'smooth' })
    }
  }
}
</script>

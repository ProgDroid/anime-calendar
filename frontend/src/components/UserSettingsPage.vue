<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useAuthStore } from '@/stores/auth'
import { toastService } from '@/services/toastService'
import type { UserSettings } from '@/types/userSettings'
import { useUserSettingsStore } from '@/stores/userSettingsStore'
import { applySettings } from '@/services/applySettings'

const authStore = useAuthStore()
const userSettingsStore = useUserSettingsStore()
const loading = ref(true)
const error = ref<string | null>(null)
const settings = ref<UserSettings>(userSettingsStore.getDefaultSettings())

const fetchUserSettings = async () => {
  try {
    loading.value = true
    error.value = null
    
    let fetchedSettings = await userSettingsStore.fetchSettings()
    console.log('Fetched settings:', fetchedSettings)
    settings.value = fetchedSettings
  } catch (err) {
    error.value = 'Failed to fetch user settings'
    console.error('Error fetching user settings:', err)
  } finally {
    loading.value = false
  }
}

const handleUpdateSettings = async () => {
  try {
    await userSettingsStore.updateSettings(settings.value)
    
    // Apply the theme immediately after updating
    applySettings(settings.value)
    
    // Show success notification
    toastService.success('Settings updated successfully!')
  } catch (err) {
    error.value = 'Failed to update user settings'
    console.error('Error updating user settings:', err)
  }
}

const handleSave = async () => {
  await handleUpdateSettings()
}

onMounted(() => {
  if (!authStore.isAuthenticated()) {
    // Redirect to login if not authenticated
    return
  }
  fetchUserSettings()
})
</script>

<template>
  <div class="min-h-[calc(100vh-6.2rem)] bg-base-200 p-4">
    <div class="max-w-2xl mx-auto">
      <div class="card bg-base-100 shadow-xl">
        <div class="card-body">
          <h1 class="card-title text-2xl">User Settings</h1>
          
          <div v-if="loading" class="flex justify-center items-center py-8">
            <span class="loading loading-spinner"></span>
            <span class="ml-2">Loading settings...</span>
          </div>
          
          <div v-if="error" class="alert alert-error mb-4">
            {{ error }}
          </div>
          
          <div v-if="!loading && settings" class="space-y-6">
            <!-- Theme Preference -->
            <div class="form-control">
              <label class="label">
                <span class="label-text">Theme</span>
              </label>
              <div class="mt-2 flex space-x-4">
                <label class="flex items-center space-x-2 cursor-pointer">
                  <input 
                    type="radio" 
                    name="theme" 
                    class="radio radio-primary"
                    v-model="settings.theme_preference" 
                    value="light"
                  />
                  <span>Light</span>
                </label>
                <label class="flex items-center space-x-2 cursor-pointer">
                  <input 
                    type="radio" 
                    name="theme" 
                    class="radio radio-primary"
                    v-model="settings.theme_preference" 
                    value="dark"
                  />
                  <span>Dark</span>
                </label>
              </div>
            </div>
            
            <!-- Language Preference -->
            <div class="form-control">
              <label class="label">
                <span class="label-text">Language</span>
              </label>
              <select 
                v-model="settings.language_preference" 
                class="ml-2 select select-bordered w-full max-w-xs"
              >
                <option value="en">English</option>
                <option value="pt">Portuguese</option>
              </select>
            </div>
            
            <!-- Title Language Preference -->
            <div class="form-control">
              <label class="label">
                <span class="label-text">Title Language</span>
              </label>
              <select 
                v-model="settings.title_language_preference" 
                class="ml-2 select select-bordered w-full max-w-xs"
              >
                <option value="English">English</option>
                <option value="Romaji">Romaji</option>
                <option value="Native">Native</option>
              </select>
            </div>

            <!-- Timezone -->
            <div class="form-control">
              <label class="label">
                <span class="label-text">Timezone</span>
              </label>
              <input 
                v-model="settings.timezone" 
                type="text" 
                class="ml-2 input input-bordered w-full max-w-xs"
                placeholder="e.g., UTC, Europe/London, America/New_York"
              />
            </div>
            
            <!-- Save Button -->
            <div class="flex justify-end pt-4">
              <button 
                @click="handleSave"
                class="btn btn-primary"
              >
                Save Settings
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
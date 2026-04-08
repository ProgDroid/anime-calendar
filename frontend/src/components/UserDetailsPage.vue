<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { useRouter } from 'vue-router'
import { useAuthStore } from '@/stores/auth'
import api from '@/config/api'
import { toastService } from '@/services/toastService'
import { useUserSettingsStore } from '@/stores/userSettingsStore'
import { applySettings } from '@/services/applySettings'
import { useI18n } from 'vue-i18n'
import ConfirmModal from '@/components/shared/ConfirmModal.vue'

const { t } = useI18n()

interface User {
  username: string
  email: string
  is_oauth: boolean
}

const router = useRouter()
const authStore = useAuthStore()
const userSettingsStore = useUserSettingsStore()

const user = ref<User | null>(null)
const loading = ref(true)
const error = ref<string | null>(null)
const isEditing = ref(false)
const updatedUsername = ref('')
const updatedEmail = ref('')
const isDeleting = ref(false)
const showPasswordUpdate = ref(false)
const currentPassword = ref('')
const newPassword = ref('')
const confirmPassword = ref('')
const isUpdatingPassword = ref(false)
const confirmDeleteOpen = ref(false)

// Computed properties to handle localStorage access safely
const userAvatar = computed(() => {
  return window.localStorage.getItem('avatar') || ''
})

const userName = computed(() => {
  return localStorage.getItem('name') || ''
})

const fetchUserDetails = async () => {
  try {
    loading.value = true
    error.value = null
    
    const response = await api.get('/user/details')
    user.value = response.data
    updatedUsername.value = response.data.username
    updatedEmail.value = response.data.email
  } catch (err) {
    error.value = t('userDetails.failedToFetch')
    console.error('Error fetching user details:', err)
  } finally {
    loading.value = false
  }
}

const handleUpdate = async (e: Event) => {
  e.preventDefault()
  
  try {
    const response = await api.put('/user', {
      username: updatedUsername.value,
      email: updatedEmail.value
    })
    
    user.value = response.data
    isEditing.value = false
    // Update the auth store with new user data
    authStore.user = response.data
    
    // Show success notification
    toastService.success(t('userDetails.updateSuccess'))
  } catch (err) {
    error.value = t('userDetails.updateFailed')
    console.error('Error updating user:', err)
  }
}

const handleUpdatePassword = async (e: Event) => {
  e.preventDefault()
  
  // Validate passwords
  if (newPassword.value !== confirmPassword.value) {
    error.value = t('userDetails.passwordsDontMatch')
    return
  }

  if (newPassword.value.length < 12) {
    error.value = t('userDetails.passwordLength', {min: 12})
    return
  }

  // Check password strength requirements
  if (!/(?=.*[a-z])(?=.*[A-Z])(?=.*\d)(?=.*[!@#$%^&*(),.?":{}|<>]).+/.test(newPassword.value)) {
    error.value = t('userDetails.passwordContent')
    return
  }

  // Check that new password is different from current password
  if (newPassword.value === currentPassword.value) {
    error.value = t('userDetails.passwordMustBeDifferent')
    return
  }

  try {
    isUpdatingPassword.value = true
    error.value = null
    
    await api.post('/user/password', {
      current_password: currentPassword.value,
      new_password: newPassword.value
    })
    
    // Clear form
    currentPassword.value = ''
    newPassword.value = ''
    confirmPassword.value = ''
    showPasswordUpdate.value = false
    
    // Show success notification
    toastService.success(t('userDetails.passwordUpdateSuccess'))
  } catch (err: any) {
    if (err.response && err.response.status === 401) {
      error.value = t('userDetails.currentPasswordIncorrect')
    } else {
      error.value = t('userDetails.passwordUpdateFailed')
    }
    console.error('Error updating password:', err)
  } finally {
    isUpdatingPassword.value = false
  }
}

const handleDelete = async () => {
  confirmDeleteOpen.value = false
  try {
    isDeleting.value = true
    await api.delete('/user')
    authStore.logout()
    userSettingsStore.clearCache()
    applySettings(userSettingsStore.getDefaultSettings())
    router.push('/login')
  } catch {
    error.value = t('userDetails.accountDeleteFailed')
  } finally {
    isDeleting.value = false
  }
}

onMounted(() => {
  if (!authStore.isAuthenticated()) {
    router.push('/login')
    return
  }
  fetchUserDetails()
})
</script>

<template>
  <div class="min-h-[calc(100vh-6.2rem)] bg-base-200 p-4">
    <ConfirmModal
      :open="confirmDeleteOpen"
      :title="$t('userDetails.accountDeleteButton')"
      :message="$t('userDetails.accountDeleteWarning')"
      :confirm-label="$t('userDetails.accountDeleteButton')"
      :danger="true"
      @confirm="handleDelete"
      @cancel="confirmDeleteOpen = false"
    />
    <div class="max-w-2xl mx-auto">
      <div class="card bg-base-100 shadow-xl">
        <div class="card-body">
          <h1 class="card-title text-2xl">{{ $t('userDetails.title') }}</h1>
          
          <div v-if="loading" class="flex justify-center items-center py-8">
            <span class="loading loading-spinner"></span>
            <span class="ml-2">{{ $t('userDetails.loading') }}</span>
          </div>
          
          <div v-if="error" class="alert alert-error mb-4">
            {{ error }}
          </div>
          
          <div v-if="user" class="space-y-6">
            <div v-if="user.is_oauth" class="flex justify-center mb-6">
              <div class="avatar">
                <div class="w-24 h-24 rounded-full">
                  <img :src="userAvatar" :alt="userName + ' avatar'" />
                </div>
              </div>
            </div>
            
            <div v-if="!isEditing">
              <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                <div class="form-control">
                  <label class="label mb-2">
                    <span class="label-text">{{ $t('userDetails.name') }}</span>
                  </label>
                  <input 
                    type="text" 
                    class="input input-bordered"
                    :value="user.is_oauth ? userName : user.username"
                    :disabled="true"
                  />
                </div>
                
                <div class="form-control">
                  <label class="label mb-2">
                    <span class="label-text">{{ $t('userDetails.email') }}</span>
                  </label>
                  <input 
                    type="email" 
                    class="input input-bordered"
                    :value="user.email"
                    :disabled="true"
                  />
                </div>
              </div>
              
              <!-- Edit button only for non-OAuth users -->
              <div v-if="!user.is_oauth" class="flex justify-end space-x-3 mt-6">
                <button 
                  @click="isEditing = true"
                  class="btn btn-primary w-full"
                >
                  {{ $t('userDetails.editDetails') }}
                </button>
              </div>
              
              <div class="flex justify-end space-x-3 mt-4">
                <RouterLink to="/user/settings" class="btn btn-outline w-full">
                  {{ $t('userDetails.viewSettings') }}
                </RouterLink>
              </div>
            </div>
            
            <div v-else class="space-y-4">
              <form @submit="handleUpdate" class="space-y-4">
                <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                  <div class="form-control">
                    <label class="label">
                      <span class="label-text">{{ $t('userDetails.username') }}</span>
                    </label>
                    <input 
                      v-model="updatedUsername"
                      type="text" 
                      class="input input-bordered"
                      required
                    />
                  </div>

                  <div class="form-control">
                    <label class="label">
                      <span class="label-text">{{ $t('userDetails.email') }}</span>
                    </label>
                    <input 
                      v-model="updatedEmail"
                      type="email" 
                      class="input input-bordered"
                      required
                    />
                  </div>
                </div>
                
                <div class="flex justify-end space-x-3 mt-4">
                  <button 
                    type="button"
                    @click="isEditing = false"
                    class="btn btn-ghost"
                  >
                    {{ $t('userDetails.cancel') }}
                  </button>
                  <button 
                    type="submit"
                    class="btn btn-primary"
                  >
                    {{ $t('userDetails.update')}}
                  </button>
                </div>
              </form>
            </div>
            
            <div v-if="!user.is_oauth" class="divider"></div>
          
            <div class="space-y-6">
              <!-- Password update section hidden for OAuth users -->
              <div v-if="!user.is_oauth" class="flex justify-between items-center">
                <h2 class="card-title">{{ $t('userDetails.changePassword') }}</h2>
                <button 
                  @click="showPasswordUpdate = !showPasswordUpdate"
                  class="btn btn-outline"
                >
                  {{ showPasswordUpdate ? $t('userDetails.cancel') : $t('userDetails.changePassword') }}
                </button>
              </div>
              
              <div v-if="!user.is_oauth && showPasswordUpdate" class="card bg-base-200 p-6 rounded-lg shadow-md">
                <form @submit="handleUpdatePassword" class="space-y-6">
                  <div class="form-control">
                    <label class="label mb-2">
                      <span class="label-text">{{ $t('userDetails.currentPassword') }}</span>
                    </label>
                    <input 
                      v-model="currentPassword"
                      type="password" 
                      class="input input-bordered w-full"
                      required
                    />
                  </div>

                  <div class="form-control">
                    <label class="label mb-2">
                      <span class="label-text">{{ $t('userDetails.newPassword') }}</span>
                    </label>
                    <input 
                      v-model="newPassword"
                      type="password" 
                      class="input input-bordered w-full"
                      required
                    />
                    <label class="label mb-2">
                      <span class="label-text text-sm text-wrap text-center">{{ $t('userDetails.passwordHint') }}</span>
                    </label>
                  </div>

                  <div class="form-control">
                    <label class="label mb-2">
                      <span class="label-text">{{ $t('userDetails.confirmPassword') }}</span>
                    </label>
                    <input 
                      v-model="confirmPassword"
                      type="password" 
                      class="input input-bordered w-full"
                      required
                    />
                  </div>

                  <div class="flex justify-end space-x-3 mt-4">
                    <button 
                      type="button"
                      @click="showPasswordUpdate = false"
                      class="btn btn-ghost"
                    >
                      {{ $t('userDetails.cancel') }}
                    </button>
                    <button 
                      type="submit"
                      :disabled="isUpdatingPassword"
                      class="btn btn-primary"
                    >
                      <span v-if="isUpdatingPassword">{{ $t('userDetails.passwordUpdating') }}</span>
                      <span v-else>{{ $t('userDetails.passwordUpdate') }}</span>
                    </button>
                  </div>
                </form>
              </div>
              
              <div class="divider"></div>
            
              <div class="alert alert-warning">
                <svg xmlns="http://www.w3.org/2000/svg" class="stroke-current shrink-0 h-6 w-6" fill="none" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" /></svg>
                <div>
                  <h3 class="font-bold">{{ $t('userDetails.accountDeleteButton') }}</h3>
                  <p>{{ $t('userDetails.accountDeleteWarning') }}</p>
                </div>
              </div>
              
              <div class="flex justify-end">
                <button
                  @click="confirmDeleteOpen = true"
                  :disabled="isDeleting"
                  class="btn btn-error w-full"
                >
                  <span v-if="isDeleting">{{ $t('userDetails.accountDeleting') }}</span>
                  <span v-else>{{ $t('userDetails.accountDeleteButton') }}</span>
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

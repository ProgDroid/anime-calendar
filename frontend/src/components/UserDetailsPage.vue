<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { useAuthStore } from '@/stores/auth'
import api from '@/config/api'
import { toastService } from '@/services/toastService'

interface User {
  username: string
  email: string
}

const router = useRouter()
const authStore = useAuthStore()

const user = ref<User | null>(null)
const loading = ref(true)
const error = ref<string | null>(null)
const isEditing = ref(false)
const updatedUsername = ref('')
const updatedEmail = ref('')
const isDeleting = ref(false)

const fetchUserDetails = async () => {
  try {
    loading.value = true
    error.value = null
    
    const response = await api.get('/user/details')
    user.value = response.data
    updatedUsername.value = response.data.username
    updatedEmail.value = response.data.email
  } catch (err) {
    error.value = 'Failed to fetch user details'
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
    toastService.success('User details updated successfully!')
  } catch (err) {
    error.value = 'Failed to update user details'
    console.error('Error updating user:', err)
  }
}

const handleDelete = async () => {
  if (!confirm('Are you sure you want to delete your account? This action cannot be undone.')) {
    return
  }

  try {
    isDeleting.value = true
    await api.delete('/user')
    authStore.logout()
    router.push('/login')
  } catch (err) {
    error.value = 'Failed to delete account'
    console.error('Error deleting account:', err)
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
  <div class="min-h-screen bg-base-200 p-4">
    <div class="max-w-2xl mx-auto">
      <div class="card bg-base-100 shadow-xl">
        <div class="card-body">
          <h1 class="card-title text-2xl">User Details</h1>
          
          <div v-if="loading" class="flex justify-center items-center py-8">
            <span class="loading loading-spinner"></span>
            <span class="ml-2">Loading user details...</span>
          </div>
          
          <div v-else-if="error" class="alert alert-error">
            {{ error }}
          </div>
          
          <div v-else-if="user" class="space-y-6">
            <div v-if="!isEditing">
              <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                <div class="form-control">
                  <label class="label mb-2">
                    <span class="label-text">Username</span>
                  </label>
                  <input 
                    type="text" 
                    class="input input-bordered"
                    :value="user.username"
                    disabled
                  />
                </div>
                
                <div class="form-control">
                  <label class="label mb-2">
                    <span class="label-text">Email</span>
                  </label>
                  <input 
                    type="email" 
                    class="input input-bordered"
                    :value="user.email"
                    disabled
                  />
                </div>
              </div>
              
              <div class="flex justify-end space-x-3 mt-6">
                <button 
                  @click="isEditing = true"
                  class="btn btn-primary"
                >
                  Edit Details
                </button>
              </div>
            </div>
            
            <div v-else class="space-y-4">
              <form @submit="handleUpdate" class="space-y-4">
                <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                  <div class="form-control">
                    <label class="label">
                      <span class="label-text">Username</span>
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
                      <span class="label-text">Email</span>
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
                    Cancel
                  </button>
                  <button 
                    type="submit"
                    class="btn btn-primary"
                  >
                    Save Changes
                  </button>
                </div>
              </form>
            </div>
            
            <div class="divider"></div>
            
            <div class="alert alert-warning">
              <svg xmlns="http://www.w3.org/2000/svg" class="stroke-current shrink-0 h-6 w-6" fill="none" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" /></svg>
              <div>
                <h3 class="font-bold">Delete Account</h3>
                <p>Deleting your account will remove all your data permanently. This action cannot be undone.</p>
              </div>
            </div>
            
            <div class="flex justify-end">
              <button 
                @click="handleDelete"
                :disabled="isDeleting"
                class="btn btn-error"
              >
                <span v-if="isDeleting">Deleting...</span>
                <span v-else>Delete Account</span>
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

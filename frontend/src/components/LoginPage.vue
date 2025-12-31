<script setup lang="ts">
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import { useAuthStore } from '@/stores/auth'

const router = useRouter()
const authStore = useAuthStore()

const email = ref('')
const password = ref('')
const username = ref('')
const isRegistering = ref(false)
const loading = ref(false)
const error = ref<string | null>(null)

const handleSubmit = async (e: Event) => {
  e.preventDefault()
  loading.value = true
  error.value = null

  try {
    if (isRegistering.value) {
      await authStore.register(username.value, email.value, password.value)
    } else {
      await authStore.login(email.value, password.value)
    }
    router.push('/my-calendars')
  } catch (err) {
    error.value = err instanceof Error ? err.message : 'An error occurred'
  } finally {
    loading.value = false
  }
}

const toggleMode = () => {
  isRegistering.value = !isRegistering.value
  error.value = null
}
</script>

<template>
  <div class="min-h-screen bg-base-200 flex items-center justify-center p-4">
    <div class="card bg-base-100 w-full max-w-md shadow-xl">
      <div class="card-body">
        <h2 class="card-title">{{ isRegistering ? 'Register' : 'Login' }}</h2>
        <form @submit="handleSubmit" class="space-y-4">
          <div v-if="isRegistering" class="form-control w-full">
            <label class="label mb-2">
              <span class="label-text">Username</span>
            </label>
            <input 
              id="username" 
              v-model="username" 
              type="text" 
              required 
              class="input input-bordered w-full"
              placeholder="Enter your username"
            />
          </div>
          
          <div class="form-control w-full">
            <label class="label mb-2">
              <span class="label-text">Email</span>
            </label>
            <input 
              id="email" 
              v-model="email" 
              type="email" 
              required 
              class="input input-bordered w-full"
              placeholder="Enter your email"
            />
          </div>
          
          <div class="form-control w-full">
            <label class="label mb-2">
              <span class="label-text">Password</span>
            </label>
            <input 
              id="password" 
              v-model="password" 
              type="password" 
              required 
              class="input input-bordered w-full"
              placeholder="Enter your password"
            />
          </div>
          
          <button 
            type="submit" 
            :disabled="loading" 
            class="btn btn-primary w-full"
          >
            {{ loading ? (isRegistering ? 'Registering...' : 'Logging in...') : (isRegistering ? 'Register' : 'Login') }}
          </button>
          
          <div v-if="error" class="alert alert-error mt-4">
            {{ error }}
          </div>
        </form>
        <div class="card-actions justify-center mt-4">
          <p class="text-center">
            {{ isRegistering ? 'Already have an account?' : "Don't have an account?" }}
            <button @click="toggleMode" class="link link-primary">
              {{ isRegistering ? 'Login' : 'Register' }}
            </button>
          </p>
        </div>
      </div>
    </div>
  </div>
</template>

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
  <div class="login-page">
    <div class="login-container">
      <h1>{{ isRegistering ? 'Register' : 'Login' }}</h1>
      
      <form @submit="handleSubmit" class="login-form">
        <div v-if="isRegistering" class="form-group">
          <label for="username">Username:</label>
          <input 
            id="username" 
            v-model="username" 
            type="text" 
            required 
            placeholder="Enter your username"
          />
        </div>
        
        <div class="form-group">
          <label for="email">Email:</label>
          <input 
            id="email" 
            v-model="email" 
            type="email" 
            required 
            placeholder="Enter your email"
          />
        </div>
        
        <div class="form-group">
          <label for="password">Password:</label>
          <input 
            id="password" 
            v-model="password" 
            type="password" 
            required 
            placeholder="Enter your password"
          />
        </div>
        
        <button 
          type="submit" 
          :disabled="loading" 
          class="btn btn-primary"
        >
          {{ loading ? (isRegistering ? 'Registering...' : 'Logging in...') : (isRegistering ? 'Register' : 'Login') }}
        </button>
        
        <div v-if="error" class="error">
          {{ error }}
        </div>
      </form>
      
      <div class="switch-mode">
        <p>
          {{ isRegistering ? 'Already have an account?' : "Don't have an account?" }}
          <button @click="toggleMode" class="btn btn-link">
            {{ isRegistering ? 'Login' : 'Register' }}
          </button>
        </p>
      </div>
    </div>
  </div>
</template>

<style scoped>
.login-page {
  display: flex;
  justify-content: center;
  align-items: center;
  min-height: 100vh;
  background-color: #f8f9fa;
}

.login-container {
  background-color: white;
  padding: 30px;
  border-radius: 8px;
  box-shadow: 0 2px 10px rgba(0,0,0,0.1);
  width: 100%;
  max-width: 400px;
}

.login-container h1 {
  text-align: center;
  margin-bottom: 20px;
  color: #333;
}

.login-form {
  display: flex;
  flex-direction: column;
}

.form-group {
  margin-bottom: 15px;
}

.form-group label {
  display: block;
  margin-bottom: 5px;
  font-weight: bold;
  color: #333;
}

.form-group input {
  width: 100%;
  padding: 10px;
  border: 1px solid #ddd;
  border-radius: 4px;
  box-sizing: border-box;
}

.btn {
  padding: 12px;
  border: none;
  border-radius: 4px;
  cursor: pointer;
  font-size: 1em;
  transition: background-color 0.2s;
}

.btn-primary {
  background-color: #007bff;
  color: white;
}

.btn-primary:hover {
  background-color: #0056b3;
}

.btn-link {
  background: none;
  color: #007bff;
  text-decoration: underline;
  cursor: pointer;
  padding: 0;
  margin-left: 5px;
}

.error {
  color: #dc3545;
  padding: 10px;
  background-color: #f8d7da;
  border: 1px solid #f5c6cb;
  border-radius: 4px;
  margin-top: 15px;
}

.switch-mode {
  text-align: center;
  margin-top: 20px;
}

.switch-mode p {
  margin: 0;
  color: #333;
}
</style>

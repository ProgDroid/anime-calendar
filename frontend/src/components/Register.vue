<template>
  <div class="register-container">
    <h2>Register</h2>
    <form @submit.prevent="handleRegister">
      <div>
        <label for="username">Username:</label>
        <input
          id="username"
          v-model="registerForm.username"
          type="text"
          required
        />
      </div>
      <div>
        <label for="email">Email:</label>
        <input
          id="email"
          v-model="registerForm.email"
          type="email"
          required
        />
      </div>
      <div>
        <label for="password">Password:</label>
        <input
          id="password"
          v-model="registerForm.password"
          type="password"
          required
        />
      </div>
      <button type="submit" :disabled="loading">Register</button>
      <div v-if="error" class="error">{{ error }}</div>
    </form>
    <p>
      Already have an account?
      <router-link to="/login">Login</router-link>
    </p>
  </div>
</template>

<script setup>
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import { useAuthStore } from '../stores/auth'

const router = useRouter()
const authStore = useAuthStore()

const registerForm = ref({
  username: '',
  email: '',
  password: ''
})

const loading = ref(false)
const error = ref('')

const handleRegister = async () => {
  loading.value = true
  error.value = ''

  try {
    await authStore.register(registerForm.value.username, registerForm.value.email, registerForm.value.password)
    router.push('/calendar')
  } catch (err) {
    error.value = err.message || 'Registration failed'
  } finally {
    loading.value = false
  }
}
</script>

<style scoped>
.register-container {
  max-width: 400px;
  margin: 50px auto;
  padding: 20px;
  border: 1px solid #ddd;
  border-radius: 5px;
}

.register-container h2 {
  text-align: center;
}

.register-container div {
  margin-bottom: 15px;
}

.register-container label {
  display: block;
  margin-bottom: 5px;
}

.register-container input {
  width: 100%;
  padding: 8px;
  box-sizing: border-box;
}

.register-container button {
  width: 100%;
  padding: 10px;
  background-color: #28a745;
  color: white;
  border: none;
  border-radius: 5px;
  cursor: pointer;
}

.register-container button:disabled {
  background-color: #ccc;
  cursor: not-allowed;
}

.error {
  color: red;
  margin-top: 10px;
}
</style>

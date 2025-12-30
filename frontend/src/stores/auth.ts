import { defineStore } from 'pinia'
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import api from '../config/api'

export const useAuthStore = defineStore('auth', () => {
  const user = ref(null)
  const token = ref('')
  const router = useRouter()

  const isAuthenticated = () => {
    return !!token.value
  }

  const login = async (email: string, password: string) => {
    try {
      const response = await api.post('/login', {
        email,
        password
      })
      
      const { token: authToken, user: userData } = response.data
      token.value = authToken
      user.value = userData
      
      // Store token in localStorage
      localStorage.setItem('authToken', authToken)
      
      return response.data
    } catch (error: any) {
      throw new Error(error.response?.data?.message || 'Login failed')
    }
  }

  const register = async (username: string, email: string, password: string) => {
    try {
      const response = await api.post('/register', {
        username,
        email,
        password
      })
      
      const { token: authToken, user: userData } = response.data
      token.value = authToken
      user.value = userData
      
      // Store token in localStorage
      localStorage.setItem('authToken', authToken)
      
      return response.data
    } catch (error: any) {
      throw new Error(error.response?.data?.message || 'Registration failed')
    }
  }

  const logout = () => {
    token.value = ''
    user.value = null
    localStorage.removeItem('authToken')
    router.push('/login')
  }

  const getCurrentUser = async () => {
    if (!token.value) {
      throw new Error('No authentication token')
    }
    
    try {
      const response = await api.get('/me')
      user.value = response.data
      return response.data
    } catch (error: any) {
      logout()
      throw new Error('Failed to get user data')
    }
  }

  const initAuth = async () => {
    const storedToken = localStorage.getItem('authToken')
    if (storedToken) {
      token.value = storedToken
      try {
        await getCurrentUser()
      } catch (error: any) {
        // If we can't get the user, clear the token
        logout()
      }
    }
  }

  return {
    user,
    token,
    isAuthenticated,
    login,
    register,
    logout,
    getCurrentUser,
    initAuth
  }
})

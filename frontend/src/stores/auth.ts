import { defineStore } from 'pinia'
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import axios from 'axios'
import api from '../config/api'
import { invalidateSettingsCache } from '@/services/userSettingsService'

export const useAuthStore = defineStore('auth', () => {
  const user = ref('')
  const token = ref('')
  const name = ref('')
  const user_avatar = ref('')
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
      
      const { token: authToken, username: userData } = response.data
      token.value = authToken
      user.value = userData
      
      // Store token in localStorage
      localStorage.setItem('authToken', authToken)
      
      return response.data
    } catch (err) {
      if (axios.isAxiosError(err)) {
        throw new Error(err.response?.data?.error || 'Login failed')
      }
      throw err
    }
  }

  const register = async (username: string, email: string, password: string) => {
    try {
      const response = await api.post('/register', {
        username,
        email,
        password
      })

      const { token: authToken, username: userData } = response.data
      token.value = authToken
      user.value = userData

      // Store token in localStorage
      localStorage.setItem('authToken', authToken)

      return response.data
    } catch (err) {
      if (axios.isAxiosError(err)) {
        throw new Error(err.response?.data?.error || 'Registration failed')
      }
      throw err
    }
  }

  // Add OAuth login function
  const oauthLogin = async (provider: 'google', token_string: string) => {
    try {
      const response = await api.post(`/auth/${provider}`, {
        token: token_string
      })
      
      const { token: authToken, username: userData, avatar: avatarUrl} = response.data
      token.value = authToken
      user.value = userData
      name.value = userData
      user_avatar.value = avatarUrl
      
      // Store token in localStorage
      localStorage.setItem('authToken', authToken)
      localStorage.setItem('name', userData)
      localStorage.setItem('avatar', avatarUrl)
      
      return response.data
    } catch (err) {
      if (axios.isAxiosError(err)) {
        throw new Error(err.response?.data?.error || `${provider} login failed`)
      }
      throw err
    }
  }

  const logout = () => {
    token.value = ''
    user.value = ''
    localStorage.removeItem('authToken')
    localStorage.removeItem('name')
    localStorage.removeItem('avatar')
    invalidateSettingsCache()
    router.push('/login')
  }

  const getCurrentUser = async () => {
    if (!token.value) {
      throw new Error('No authentication token')
    }
    
    try {
      const response = await api.get('/user')
      user.value = response.data
      return response.data
    } catch {
      logout()
      throw new Error('Failed to get user data')
    }
  }

  const initAuth = async () => {
    const storedToken = localStorage.getItem('authToken')
    if (storedToken) {
      token.value = storedToken
      try {
        // First verify the token is valid
        const isValid = await checkAuth()
        if (!isValid) {
          // Token is invalid, clear it
          logout()
        } else {
          // Token is valid, get user data
          await getCurrentUser()
        }
      } catch {
        // If we can't verify the token, clear it
        logout()
      }
    }
  }

  // Check if user is authenticated using the verify endpoint
  const checkAuth = async () => {
    const storedToken = localStorage.getItem('authToken')
    
    if (storedToken) {
      try {
        await api.post('/auth/verify', {
          token: storedToken
        })

        token.value = storedToken
        // Don't set user here, we'll get user data separately if needed
        return true
      } catch {
        // Token is invalid, remove it
        localStorage.removeItem('authToken')
        return false
      }
    }
    
    return false
  }

  return {
    user,
    token,
    user_avatar,
    name,
    isAuthenticated,
    login,
    register,
    oauthLogin,
    logout,
    getCurrentUser,
    initAuth,
    checkAuth
  }
})
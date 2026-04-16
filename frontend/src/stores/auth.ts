import { defineStore } from 'pinia'
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import axios from 'axios'
import api from '../config/api'
import { invalidateSettingsCache } from '@/services/userSettingsService'

export const useAuthStore = defineStore('auth', () => {
  const user = ref('')
  const name = ref('')
  const user_avatar = ref('')
  const router = useRouter()

  let initPromise: Promise<void> | null = null
  let initialized = false

  const isAuthenticated = () => user.value !== ''

  /**
   * Rehydrate auth state from the server on page load.
   * Calls GET /user; on success populates user.value, on failure clears it.
   * Idempotent — subsequent calls return immediately.
   * Concurrent calls return the same in-flight Promise.
   */
  const initAuth = async (): Promise<void> => {
    if (initialized) return
    if (initPromise) return initPromise

    initPromise = (async () => {
      try {
        const response = await api.get('/user')
        user.value = response.data.username ?? ''
      } catch {
        user.value = ''
      } finally {
        initialized = true
        initPromise = null
      }
    })()

    return initPromise
  }

  const login = async (email: string, password: string) => {
    try {
      const response = await api.post('/login', { email, password })
      user.value = response.data.username
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
      const response = await api.post('/register', { username, email, password })
      // No cookie is issued — user must verify email before logging in.
      return response.data
    } catch (err) {
      if (axios.isAxiosError(err)) {
        throw new Error(err.response?.data?.error || 'Registration failed')
      }
      throw err
    }
  }

  const verifyEmail = async (token: string): Promise<string> => {
    try {
      const response = await api.post('/auth/verify-email', { token })
      // Backend issues the auth cookie; set local state
      user.value = response.data.username
      return response.data.username as string
    } catch (err) {
      if (axios.isAxiosError(err)) {
        throw new Error(err.response?.data?.error || 'Verification failed')
      }
      throw err
    }
  }

  const oauthLogin = async (provider: 'google', token_string: string) => {
    try {
      const response = await api.post(`/auth/${provider}`, { token: token_string })
      const { username: userData, avatar: avatarUrl } = response.data
      user.value = userData
      name.value = userData
      user_avatar.value = avatarUrl
      // Non-sensitive display data only — no auth token in localStorage
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

  const logout = async () => {
    try {
      await api.post('/auth/logout')
    } catch {
      // Clear local state regardless of server response
    }
    user.value = ''
    name.value = ''
    user_avatar.value = ''
    initialized = false
    localStorage.removeItem('name')
    localStorage.removeItem('avatar')
    invalidateSettingsCache()
    router.push('/login')
  }

  return {
    user,
    user_avatar,
    name,
    isAuthenticated,
    login,
    register,
    verifyEmail,
    oauthLogin,
    logout,
    initAuth,
  }
})

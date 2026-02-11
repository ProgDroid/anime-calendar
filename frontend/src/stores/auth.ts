import { defineStore } from 'pinia'
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import api from '../config/api'

export const useAuthStore = defineStore('auth', () => {
  const user = ref('')
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

  // Add OAuth login function
  const oauthLogin = async (provider: 'google', token_string: string) => {
    try {
      const response = await api.post(`/auth/${provider}`, {
        token: token_string
      })
      
      const { token: authToken, user: userData } = response.data
      token.value = authToken
      user.value = userData
      
      // Store token in localStorage
      localStorage.setItem('authToken', authToken)
      
      return response.data
    } catch (error: any) {
      throw new Error(error.response?.data?.message || `${provider} login failed`)
    }
  }

  const logout = () => {
    token.value = ''
    user.value = ''
    localStorage.removeItem('authToken')
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
        // First verify the token is valid
        const isValid = await checkAuth()
        if (!isValid) {
          // Token is invalid, clear it
          logout()
        } else {
          // Token is valid, get user data
          await getCurrentUser()
        }
      } catch (error: any) {
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
        const response = await api.post('/auth/verify', {
          token: storedToken
        })
        
        token.value = storedToken
        // Don't set user here, we'll get user data separately if needed
        return true
      } catch (error) {
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

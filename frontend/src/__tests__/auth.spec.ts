import { describe, it, expect, beforeEach, vi } from 'vitest'
import { useAuthStore } from '../stores/auth'
import api from '../config/api'

// Mock the api calls
vi.mock('../config/api', () => ({
  default: {
    post: vi.fn(),
    get: vi.fn()
  }
}))

describe('Auth Store', () => {
  beforeEach(() => {
    // Clear all mocks before each test
    vi.clearAllMocks()
    // Reset the store by creating a new instance
    const store = useAuthStore()
    // Manually reset the store state
    store.token = ''
    store.user = null
  })

  it('should initialize with empty user and token', () => {
    const store = useAuthStore()
    expect(store.user).toBeNull()
    expect(store.token).toBe('')
  })

  it('should login successfully', async () => {
    const mockResponse = {
      data: {
        token: 'mock-jwt-token',
        user: {
          id: 1,
          username: 'testuser',
          email: 'test@example.com'
        }
      }
    }
    
    vi.mocked(api.post).mockResolvedValue(mockResponse)
    
    const store = useAuthStore()
    const result = await store.login('test@example.com', 'password123')
    
    expect(result).toEqual(mockResponse.data)
    expect(store.token).toBe('mock-jwt-token')
    expect(store.user).toEqual(mockResponse.data.user)
    expect(api.post).toHaveBeenCalledWith('/login', {
      email: 'test@example.com',
      password: 'password123'
    })
  })

  it('should handle login failure', async () => {
    vi.mocked(api.post).mockRejectedValue({
      response: {
        data: {
          message: 'Invalid credentials'
        }
      }
    })
    
    const store = useAuthStore()
    
    try {
      await store.login('test@example.com', 'wrongpassword')
      expect.fail('Should have thrown an error')
    } catch (error: any) {
      expect(error.message).toBe('Invalid credentials')
    }
  })

  it('should register successfully', async () => {
    const mockResponse = {
      data: {
        token: 'mock-jwt-token',
        user: {
          id: 1,
          username: 'testuser',
          email: 'test@example.com'
        }
      }
    }
    
    vi.mocked(api.post).mockResolvedValue(mockResponse)
    
    const store = useAuthStore()
    const result = await store.register('testuser', 'test@example.com', 'password123')
    
    expect(result).toEqual(mockResponse.data)
    expect(store.token).toBe('mock-jwt-token')
    expect(store.user).toEqual(mockResponse.data.user)
    expect(api.post).toHaveBeenCalledWith('/register', {
      username: 'testuser',
      email: 'test@example.com',
      password: 'password123'
    })
  })

  it('should logout correctly', () => {
    const store = useAuthStore()
    store.token = 'mock-token'
    store.user = { id: 1, username: 'testuser', email: 'test@example.com' }
    
    store.logout()
    
    expect(store.token).toBe('')
    expect(store.user).toBeNull()
  })

  it('should get current user', async () => {
    const mockResponse = {
      data: {
        id: 1,
        username: 'testuser',
        email: 'test@example.com'
      }
    }
    
    vi.mocked(api.get).mockResolvedValue(mockResponse)
    
    const store = useAuthStore()
    store.token = 'mock-token'
    
    const result = await store.getCurrentUser()
    
    expect(result).toEqual(mockResponse.data)
    expect(api.get).toHaveBeenCalledWith('/me')
  })

  it('should handle get current user failure', async () => {
    vi.mocked(api.get).mockRejectedValue({
      response: {
        status: 401
      }
    })
    
    const store = useAuthStore()
    store.token = 'mock-token'
    
    try {
      await store.getCurrentUser()
      expect.fail('Should have thrown an error')
    } catch (error: any) {
      expect(store.token).toBe('')
      expect(store.user).toBeNull()
    }
  })
})

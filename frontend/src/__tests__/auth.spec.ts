import { describe, it, expect, beforeEach, vi } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { useAuthStore } from '../stores/auth'
import api from '../config/api'

vi.mock('../config/api', () => ({
  default: {
    post: vi.fn(),
    get: vi.fn()
  }
}))

vi.mock('vue-router', () => ({
  useRouter: () => ({ push: vi.fn() })
}))

describe('Auth Store', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
    vi.stubGlobal('localStorage', {
      getItem: vi.fn().mockReturnValue(null),
      setItem: vi.fn(),
      removeItem: vi.fn(),
      clear: vi.fn(),
    })
  })

  it('should initialize with empty user', () => {
    const store = useAuthStore()
    expect(store.user).toBe('')
    expect(store.isAuthenticated()).toBe(false)
  })

  it('should login successfully', async () => {
    const mockResponse = {
      data: {
        username: 'testuser'
      }
    }

    vi.mocked(api.post).mockResolvedValue(mockResponse)

    const store = useAuthStore()
    const result = await store.login('test@example.com', 'password123')

    expect(result).toEqual(mockResponse.data)
    expect(store.user).toBe('testuser')
    expect(store.isAuthenticated()).toBe(true)
    expect(api.post).toHaveBeenCalledWith('/login', {
      email: 'test@example.com',
      password: 'password123'
    })
  })

  it('should handle login failure', async () => {
    vi.mocked(api.post).mockRejectedValue({
      isAxiosError: true,
      response: {
        data: {
          error: 'Invalid credentials'
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
        username: 'testuser'
      }
    }

    vi.mocked(api.post).mockResolvedValue(mockResponse)

    const store = useAuthStore()
    const result = await store.register('testuser', 'test@example.com', 'password123')

    expect(result).toEqual(mockResponse.data)
    // No cookie is issued on register — user must verify email first, so store.user stays empty
    expect(store.user).toEqual('')
    expect(api.post).toHaveBeenCalledWith('/register', {
      username: 'testuser',
      email: 'test@example.com',
      password: 'password123'
    })
  })

  it('should logout correctly', async () => {
    vi.mocked(api.post).mockResolvedValue({})
    const store = useAuthStore()
    store.user = 'username'

    await store.logout()

    expect(store.user).toBe('')
    expect(store.isAuthenticated()).toBe(false)
  })

  it('should initAuth — populates user on success', async () => {
    vi.mocked(api.get).mockResolvedValue({ data: { username: 'testuser' } })

    const store = useAuthStore()
    await store.initAuth()

    expect(store.user).toBe('testuser')
    expect(api.get).toHaveBeenCalledWith('/user')
  })

  it('should initAuth — clears user on failure', async () => {
    vi.mocked(api.get).mockRejectedValue({ response: { status: 401 } })

    const store = useAuthStore()
    store.user = 'stale'
    await store.initAuth()

    expect(store.user).toBe('')
  })
})

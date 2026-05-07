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

// NOTE: Do NOT vi.mock('vue-router') here. The store's logout() action accepts
// an optional `push` callback so tests can inject a spy without mocking
// vue-router (which leaks across vitest workers — see project memory
// `feedback_vue_router_mock_leaks_across_workers`).

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
        username: 'testuser',
        user_id: 42,
      }
    }

    vi.mocked(api.post).mockResolvedValue(mockResponse)

    const store = useAuthStore()
    const result = await store.login('test@example.com', 'password123')

    expect(result).toEqual(mockResponse.data)
    expect(store.user).toBe('testuser')
    expect(store.userId).toBe(42)
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
    } catch (error) {
      expect((error as Error).message).toBe('Invalid credentials')
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
    const pushMock = vi.fn()
    const store = useAuthStore()
    store.user = 'username'
    store.userId = 7

    await store.logout(pushMock)

    expect(store.user).toBe('')
    expect(store.userId).toBeNull()
    expect(store.isAuthenticated()).toBe(false)
    expect(pushMock).toHaveBeenCalledWith('/login')
  })

  it('should initAuth — populates user on success', async () => {
    vi.mocked(api.get).mockResolvedValue({ data: { username: 'testuser', user_id: 1 } })

    const store = useAuthStore()
    await store.initAuth()

    expect(store.user).toBe('testuser')
    expect(store.userId).toBe(1)
    expect(api.get).toHaveBeenCalledWith('/user')
  })

  it('should initAuth — clears user on failure', async () => {
    vi.mocked(api.get).mockRejectedValue({ response: { status: 401 } })

    const store = useAuthStore()
    store.user = 'stale'
    store.userId = 99
    await store.initAuth()

    expect(store.user).toBe('')
    expect(store.userId).toBeNull()
  })
})

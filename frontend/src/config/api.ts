import axios from 'axios'

const api = axios.create({
  baseURL: '/api',
  withCredentials: true,
})

/**
 * Returns an absolute URL for a backend path.
 * Used for subscription/export links that are pasted into external calendar clients.
 * Must be absolute so Google Calendar / Apple Calendar can fetch them.
 */
export const getApiUrl = (path: string): string => {
  return `${window.location.origin}/api${path}`
}

// 401 interceptor: attempt a silent token refresh, then retry the original request
// once. On refresh failure redirect to /login so the user can re-authenticate.
api.interceptors.response.use(
  (response) => response,
  async (error) => {
    const originalRequest = error.config as typeof error.config & { _retried?: boolean }
    if (
      axios.isAxiosError(error) &&
      error.response?.status === 401 &&
      !originalRequest._retried
    ) {
      originalRequest._retried = true
      try {
        await api.post('/auth/refresh')
        return api(originalRequest)
      } catch {
        window.location.href = '/login'
      }
    }
    return Promise.reject(error)
  },
)

export default api

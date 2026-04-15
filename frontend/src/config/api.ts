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

export default api

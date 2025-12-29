import { ref } from 'vue'
import toml from 'toml'
import axios from 'axios'

// Default configuration
const config = ref({
  host: '127.0.0.1',
  port: 8080,
  protocol: 'http'
})

const api = axios.create({
  baseURL: 'http://localhost:8080/api',
})

export const getApiUrl = (path: string): string => {
  return `${api.defaults.baseURL}${path}`
}

export const loadConfig = async () => {
  try {
    const response = await fetch('/config.toml')
    if (!response.ok) {
      throw new Error(`Failed to load config: ${response.status} ${response.statusText}`)
    }
    const tomlContent = await response.text()
    const parsed = toml.parse(tomlContent)
    if (parsed.api) {
      config.value = {
        ...config.value,
        ...parsed.api
      }
    }
  } catch (error) {
    console.warn('Failed to load API configuration, using defaults:', error)
  }
}

export default api

import { ref } from 'vue'
import toml from 'toml'

// Default configuration
const config = ref({
  host: '127.0.0.1',
  port: 8080,
  protocol: 'http'
})

// Load configuration from TOML file
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

// Get the full API URL
export const getApiUrl = (endpoint: string) => {
  return `${config.value.protocol}://${config.value.host}:${config.value.port}${endpoint}`
}

// Export the config for use in other modules
export default config.value

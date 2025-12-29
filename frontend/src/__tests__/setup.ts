import { vi } from 'vitest'
import { createApp } from 'vue'
import { createPinia } from 'pinia'

// Mock the api calls
vi.mock('../config/api', () => ({
    default: {
        post: vi.fn(),
        get: vi.fn()
    }
}))

// Setup Pinia for tests
const app = createApp({})
const pinia = createPinia()
app.use(pinia)

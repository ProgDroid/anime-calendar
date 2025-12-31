import { createApp } from 'vue'
import Toast from '@/components/Toast.vue'

interface ToastOptions {
  message: string
  type: 'success' | 'error' | 'warning' | 'info'
  duration?: number
}

// Create a container for toasts
const toastContainer = document.createElement('div')
toastContainer.id = 'toast-container'
toastContainer.className = 'fixed bottom-4 left-4 z-50 space-y-2 w-full max-w-xs'
document.body.appendChild(toastContainer)

// Global toast state
const toasts: Array<{ id: number; component: any }> = []

// Generate unique ID
let toastId = 0
const generateId = () => ++toastId

// Toast service
export const toastService = {
  show(options: ToastOptions) {
    const id = generateId()
    
    // Create toast component instance
    const toastComponent = createApp(Toast, {
      message: options.message,
      type: options.type,
      duration: options.duration,
      onClose: () => {
        this.hide(id)
      }
    })
    
    // Mount to container
    const toastElement = document.createElement('div')
    toastContainer.appendChild(toastElement)
    toastComponent.mount(toastElement)
    
    // Store reference
    toasts.push({ id, component: toastComponent })
    
    // Auto-hide if duration is specified
    if (options.duration !== 0) {
      const duration = options.duration || 3000
      setTimeout(() => {
        this.hide(id)
      }, duration)
    }
    
    return id
  },
  
  success(message: string, options?: Omit<ToastOptions, 'message' | 'type'>) {
    return this.show({
      message,
      type: 'success',
      ...options
    })
  },
  
  error(message: string, options?: Omit<ToastOptions, 'message' | 'type'>) {
    return this.show({
      message,
      type: 'error',
      ...options
    })
  },
  
  warning(message: string, options?: Omit<ToastOptions, 'message' | 'type'>) {
    return this.show({
      message,
      type: 'warning',
      ...options
    })
  },
  
  info(message: string, options?: Omit<ToastOptions, 'message' | 'type'>) {
    return this.show({
      message,
      type: 'info',
      ...options
    })
  },
  
  hide(id: number) {
    const index = toasts.findIndex(t => t.id === id)
    if (index !== -1) {
      toasts[index]?.component.unmount()
      toasts.splice(index, 1)
    }
  },
  
  hideAll() {
    toasts.forEach(toast => toast.component.unmount())
    toasts.length = 0
  }
}

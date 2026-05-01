import { ref } from 'vue'

type ToastType = 'success' | 'error' | 'warning' | 'info'
type ToastVariant = 'success' | 'danger' | 'warning' | 'info'

interface ToastOptions {
  message: string
  type: ToastType
  duration?: number
}

export interface ActiveToast {
  id: number
  message: string
  variant: ToastVariant
  duration: number
}

const typeToVariant: Record<ToastType, ToastVariant> = {
  success: 'success',
  error: 'danger',
  warning: 'warning',
  info: 'info',
}

export const activeToasts = ref<ActiveToast[]>([])

let toastId = 0
const generateId = () => ++toastId

export const toastService = {
  show(options: ToastOptions) {
    const id = generateId()
    const duration = options.duration ?? 3000

    activeToasts.value.push({
      id,
      message: options.message,
      variant: typeToVariant[options.type],
      duration,
    })

    if (duration !== 0) {
      setTimeout(() => {
        this.hide(id)
      }, duration)
    }

    return id
  },

  success(message: string, options?: Omit<ToastOptions, 'message' | 'type'>) {
    return this.show({ message, type: 'success', ...options })
  },

  error(message: string, options?: Omit<ToastOptions, 'message' | 'type'>) {
    return this.show({ message, type: 'error', ...options })
  },

  warning(message: string, options?: Omit<ToastOptions, 'message' | 'type'>) {
    return this.show({ message, type: 'warning', ...options })
  },

  info(message: string, options?: Omit<ToastOptions, 'message' | 'type'>) {
    return this.show({ message, type: 'info', ...options })
  },

  hide(id: number) {
    const index = activeToasts.value.findIndex((t) => t.id === id)
    if (index !== -1) {
      activeToasts.value.splice(index, 1)
    }
  },

  hideAll() {
    activeToasts.value = []
  },
}

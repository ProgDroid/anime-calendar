<script setup lang="ts">
defineOptions({ name: 'AppToast' })
interface ToastProps {
  message: string
  type: 'success' | 'error' | 'warning' | 'info'
  duration?: number
}

defineProps<ToastProps>()
const emit = defineEmits(['close'])

const toastClasses = {
  success: 'alert-success',
  error: 'alert-error',
  warning: 'alert-warning',
  info: 'alert-info'
}

const handleClose = () => {
  emit('close')
}
</script>

<template>
  <div 
    class="alert shadow-lg mb-2 transition-all duration-300 ease-in-out transform"
    :class="toastClasses[type]"
  >
    <div class="flex items-center">
      <div class="flex-shrink-0">
        <svg 
          v-if="type === 'success'" 
          xmlns="http://www.w3.org/2000/svg" 
          class="stroke-current shrink-0 h-6 w-6" 
          fill="none" 
          viewBox="0 0 24 24"
        >
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
        </svg>
        <svg 
          v-else-if="type === 'error'" 
          xmlns="http://www.w3.org/2000/svg" 
          class="stroke-current shrink-0 h-6 w-6" 
          fill="none" 
          viewBox="0 0 24 24"
        >
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
        </svg>
        <svg 
          v-else-if="type === 'warning'" 
          xmlns="http://www.w3.org/2000/svg" 
          class="stroke-current shrink-0 h-6 w-6" 
          fill="none" 
          viewBox="0 0 24 24"
        >
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
        </svg>
        <svg 
          v-else 
          xmlns="http://www.w3.org/2000/svg" 
          class="stroke-current shrink-0 h-6 w-6" 
          fill="none" 
          viewBox="0 0 24 24"
        >
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
        </svg>
      </div>
      <div class="ml-3">
        <span>{{ message }}</span>
      </div>
    </div>
    <button @click="handleClose" class="btn btn-sm btn-ghost">
      <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
      </svg>
    </button>
  </div>
</template>

<style scoped>
.alert {
  animation: slideIn 0.3s ease-out;
}

@keyframes slideIn {
  from {
    transform: translateY(-100%);
    opacity: 0;
  }
  to {
    transform: translateY(0);
    opacity: 1;
  }
}
</style>

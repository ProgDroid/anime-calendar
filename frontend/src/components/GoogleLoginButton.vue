<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { useAuthStore } from '@/stores/auth'
import { i18n } from '@/plugins/i18n'

const { t } = i18n.global

const router = useRouter()
const authStore = useAuthStore()

const loading = ref(false)
const error = ref<string | null>(null)

interface GoogleResponse {
  client_id: string,
  credential: string,
}

// Define the callback function to be used by Google Sign-In
const handleGoogleLogin = async (data: GoogleResponse) => {
  loading.value = true
  error.value = null

  try {
    const googleToken = data.credential
    
    // Use the auth store's oauthLogin function
    await authStore.oauthLogin('google', googleToken)
    
    router.push('/my-calendars')
  } catch {
    error.value = t('auth.google.failed')
  } finally {
    loading.value = false
  }
}

// Initialize Google Sign-In properly
onMounted(() => {
  // Make the function globally available for Google's GSI library
  (window as any).handleGoogleLogin = handleGoogleLogin;
})
</script>

<template>
  <component is="script" src="https://accounts.google.com/gsi/client" async />
  
  <div class="flex flex-col items-center">
    <div id="g_id_onload"
      data-client_id="269915074579-d4gsd3elouqus0e5vplqnc74gr3ujag1.apps.googleusercontent.com"
      data-context="signin"
      data-ux_mode="popup"
      data-callback="handleGoogleLogin"
      data-itp_support="true">
    </div>

    <div class="g_id_signin"
      data-type="standard"
      data-shape="rectangular"
      data-theme="filled_black"
      data-text="continue_with"
      data-size="large"
      data-logo_alignment="left">
    </div>
  </div>
</template>

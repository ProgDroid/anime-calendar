<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount } from 'vue'
import { useRouter } from 'vue-router'
import { useAuthStore } from '@/stores/auth'
import { i18n } from '@/plugins/i18n'
import { getPublicConfig } from '@/services/publicConfig'

const googleClientId = getPublicConfig().googleClientId

const GSI_SRC = 'https://accounts.google.com/gsi/client'
const GSI_SCRIPT_ID = 'google-gsi-client'

interface GoogleResponse {
  client_id: string,
  credential: string,
}

declare global {
  interface Window {
    handleGoogleLogin?: (data: GoogleResponse) => void
  }
}

const { t } = i18n.global

const router = useRouter()
const authStore = useAuthStore()

const loading = ref(false)
const error = ref<string | null>(null)

const handleGoogleLogin = async (data: GoogleResponse) => {
  loading.value = true
  error.value = null

  try {
    const googleToken = data.credential
    await authStore.oauthLogin('google', googleToken)
    router.push('/my-calendars')
  } catch {
    error.value = t('auth.google.failed')
  } finally {
    loading.value = false
  }
}

onMounted(() => {
  window.handleGoogleLogin = handleGoogleLogin

  // Inject the GSI client programmatically. Avoids `<component :is="'script'">`,
  // which triggers Vue's "reserved HTML element as component id" warning.
  if (!document.getElementById(GSI_SCRIPT_ID)) {
    const s = document.createElement('script')
    s.id = GSI_SCRIPT_ID
    s.src = GSI_SRC
    s.async = true
    s.defer = true
    document.head.appendChild(s)
  }
})

onBeforeUnmount(() => {
  delete window.handleGoogleLogin
})
</script>

<template>
  <div class="flex flex-col items-center">
    <div id="g_id_onload"
      :data-client_id="googleClientId"
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

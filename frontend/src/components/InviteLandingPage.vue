<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useAuthStore } from '@/stores/auth'
import { sharingService } from '@/services/sharingService'
import type { InvitationPreview } from '@/types/sharing'
import UiButton from '@/components/ui/UiButton.vue'
import axios from 'axios'

defineOptions({ name: 'InviteLandingPage' })

const { t } = useI18n()
const route = useRoute()
const router = useRouter()
const auth = useAuthStore()
const token = route.params.token as string

type State = 'loading' | 'invalid' | 'unauth' | 'ready' | 'mismatch'
const state = ref<State>('loading')
const preview = ref<InvitationPreview | null>(null)
const accepting = ref(false)

onMounted(async () => {
  try {
    preview.value = await sharingService.preview(token)
    state.value = auth.isAuthenticated() ? 'ready' : 'unauth'
  } catch {
    state.value = 'invalid'
  }
})

async function accept() {
  accepting.value = true
  try {
    const result = await sharingService.accept(token)
    router.push(`/calendars/${result.calendar_id}`)
  } catch (err) {
    if (axios.isAxiosError(err) && err.response?.status === 403) {
      state.value = 'mismatch'
    }
    // other errors: stay in ready state; toast could be added later
  } finally {
    accepting.value = false
  }
}

async function decline() {
  await sharingService.decline(token)
  router.push('/my-calendars')
}
</script>

<template>
  <main class="min-h-screen bg-bg-0 flex items-center justify-center p-4">
    <!-- Loading -->
    <div
      v-if="state === 'loading'"
      data-testid="invite-loading"
      role="status"
      aria-live="polite"
      class="flex flex-col items-center gap-4"
    >
      <span
        class="inline-block w-8 h-8 border-2 border-fg-3 border-t-accent-1 rounded-full animate-spin"
        aria-hidden="true"
      />
      <p class="text-sm text-fg-2">{{ t('sharing.landing.loading') }}</p>
    </div>

    <!-- Invalid -->
    <div
      v-else-if="state === 'invalid'"
      data-testid="invite-invalid"
      class="max-w-md w-full bg-bg-1 border border-line rounded-xl p-8 text-center"
    >
      <p class="text-fg-1">{{ t('sharing.landing.invalid') }}</p>
    </div>

    <!-- Unauthenticated -->
    <div
      v-else-if="state === 'unauth'"
      data-testid="invite-unauth"
      class="max-w-md w-full bg-bg-1 border border-line rounded-xl p-8 flex flex-col gap-6"
    >
      <div class="flex flex-col gap-2 text-center">
        <h1 class="text-xl font-semibold text-fg-1">{{ t('sharing.landing.unauth_heading') }}</h1>
        <p class="text-sm text-fg-2">{{ t('sharing.landing.unauth_subtext') }}</p>
      </div>

      <div v-if="preview" class="bg-bg-2 border border-line rounded-lg p-4 flex flex-col gap-1">
        <p class="font-medium text-fg-1">
          {{ t('sharing.landing.calendarPreview', { name: preview.calendar_name, count: preview.item_count }) }}
        </p>
        <p class="text-sm text-fg-2">
          {{ t('sharing.landing.ownedBy', { owner: preview.owner_display }) }}
        </p>
      </div>

      <div class="flex flex-col gap-3">
        <router-link
          :to="`/login?redirect=/invite/${token}`"
          data-testid="invite-sign-in"
          class="inline-flex items-center justify-center h-10 px-4 rounded-md font-medium bg-accent-1 text-bg-0 hover:-translate-y-[0.5px] transition-all"
        >
          {{ t('sharing.landing.signIn') }}
        </router-link>
        <router-link
          :to="`/register?redirect=/invite/${token}`"
          data-testid="invite-sign-up"
          class="inline-flex items-center justify-center h-10 px-4 rounded-md font-medium bg-bg-2 text-fg-1 border border-line hover:-translate-y-[0.5px] transition-all"
        >
          {{ t('sharing.landing.signUp') }}
        </router-link>
      </div>
    </div>

    <!-- Ready (authenticated) -->
    <div
      v-else-if="state === 'ready'"
      data-testid="invite-ready"
      class="max-w-md w-full bg-bg-1 border border-line rounded-xl p-8 flex flex-col gap-6"
    >
      <div v-if="preview" class="flex flex-col gap-2">
        <p class="font-medium text-fg-1">
          {{ t('sharing.landing.calendarPreview', { name: preview.calendar_name, count: preview.item_count }) }}
        </p>
        <p class="text-sm text-fg-2">
          {{ t('sharing.landing.ownedBy', { owner: preview.owner_display }) }}
        </p>
      </div>

      <div class="flex gap-3">
        <UiButton
          data-testid="invite-accept"
          :loading="accepting"
          @click="accept"
        >
          {{ accepting ? t('sharing.landing.accepting') : t('sharing.landing.accept') }}
        </UiButton>
        <UiButton
          data-testid="invite-decline"
          variant="secondary"
          :disabled="accepting"
          @click="decline"
        >
          {{ t('sharing.landing.decline') }}
        </UiButton>
      </div>
    </div>

    <!-- Mismatch -->
    <div
      v-else-if="state === 'mismatch'"
      data-testid="invite-mismatch"
      class="max-w-md w-full bg-bg-1 border border-line rounded-xl p-8 flex flex-col gap-4"
    >
      <p class="text-fg-1">
        {{ t('sharing.landing.mismatch', { email: preview?.masked_email ?? '' }) }}
      </p>
      <router-link
        :to="`/login?redirect=/invite/${token}`"
        data-testid="invite-sign-in"
        class="inline-flex items-center justify-center h-10 px-4 rounded-md font-medium bg-accent-1 text-bg-0 hover:-translate-y-[0.5px] transition-all w-fit"
      >
        {{ t('sharing.landing.signIn') }}
      </router-link>
    </div>
  </main>
</template>

<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import UiButton from './ui/UiButton.vue'
import IconSparkle from './ui/icons/IconSparkle.vue'
import { getMySubscription } from '@/services/subscription'
import { toastService } from '@/services/toastService'

defineOptions({ name: 'UpgradeSuccessPage' })

const { t } = useI18n()
const route = useRoute()
const router = useRouter()

type State = 'polling' | 'flipped' | 'timeout'

const state = ref<State>('polling')

// Poll the entitlement endpoint up to ~10s; back off slightly across attempts
// so the first hit is fast (often the webhook has already landed) but later
// attempts don't slam the server.
const POLL_INTERVAL_MS = 1000
const MAX_ATTEMPTS = 10
let timer: number | null = null
let attempt = 0

function clearTimer() {
  if (timer !== null) {
    window.clearTimeout(timer)
    timer = null
  }
}

async function pollOnce() {
  attempt += 1
  const sessionId = typeof route.query.session_id === 'string' ? route.query.session_id : undefined
  try {
    const ent = await getMySubscription(sessionId)
    if (ent.tier === 'paid') {
      state.value = 'flipped'
      toastService.success(t('upgrade.success.toast'))
      // Brief delay so the user sees the flip before redirect.
      window.setTimeout(() => router.replace('/my-calendars'), 800)
      return
    }
  } catch {
    // Swallow individual errors; the polling loop is the resilience strategy.
  }
  if (attempt >= MAX_ATTEMPTS) {
    state.value = 'timeout'
    return
  }
  timer = window.setTimeout(pollOnce, POLL_INTERVAL_MS)
}

function manualRefresh() {
  attempt = 0
  state.value = 'polling'
  pollOnce()
}

onMounted(() => {
  // Without a session id we still poll — the webhook may have already written
  // the row before the user landed here.
  pollOnce()
})

onBeforeUnmount(clearTimer)
</script>

<template>
  <main id="main" class="mx-auto flex w-full max-w-md flex-col items-center px-6 py-16 text-center">
    <span
      class="flex h-16 w-16 items-center justify-center rounded-full bg-accent-1-soft text-accent-1-text"
      aria-hidden="true"
    >
      <IconSparkle />
    </span>

    <h1 class="mt-6 font-display text-3xl text-fg-1">
      {{ state === 'flipped' ? t('upgrade.success.heading') : t('upgrade.success.processingHeading') }}
    </h1>

    <p class="mt-3 text-fg-2">
      <template v-if="state === 'polling'">{{ t('upgrade.success.polling') }}</template>
      <template v-else-if="state === 'flipped'">{{ t('upgrade.success.body') }}</template>
      <template v-else>{{ t('upgrade.success.timeout') }}</template>
    </p>

    <div class="mt-8 flex flex-col gap-3">
      <UiButton
        v-if="state === 'timeout'"
        variant="primary"
        size="md"
        type="button"
        data-testid="upgrade-success-refresh"
        @click="manualRefresh"
      >
        {{ t('upgrade.success.refresh') }}
      </UiButton>
      <UiButton
        v-if="state === 'timeout'"
        variant="ghost"
        size="md"
        type="button"
        @click="router.push('/my-calendars')"
      >
        {{ t('upgrade.success.back') }}
      </UiButton>
    </div>
  </main>
</template>

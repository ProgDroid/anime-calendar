<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'
import UiButton from './ui/UiButton.vue'

defineOptions({ name: 'UpgradeCanceledPage' })

const { t } = useI18n()
const router = useRouter()
</script>

<template>
  <main
    id="main"
    class="mx-auto flex w-full max-w-sm sm:max-w-md flex-col items-center px-6 text-center
           min-h-screen min-h-dvh pt-[max(54px,calc(env(safe-area-inset-top)+12px))] pb-16"
  >
    <h1 class="font-display text-3xl sm:text-4xl text-fg-1">{{ t('upgrade.canceled.heading') }}</h1>
    <p class="mt-3 text-fg-2">{{ t('upgrade.canceled.body') }}</p>
    <div class="mt-8 flex flex-col gap-3 w-full sm:w-auto">
      <UiButton
        variant="primary"
        size="md"
        type="button"
        class="w-full sm:w-auto"
        data-testid="upgrade-canceled-retry"
        @click="router.push('/upgrade')"
      >
        {{ t('upgrade.canceled.retry') }}
      </UiButton>
      <UiButton
        variant="ghost"
        size="md"
        type="button"
        class="w-full sm:w-auto"
        @click="router.push('/my-calendars')"
      >
        {{ t('upgrade.canceled.back') }}
      </UiButton>
    </div>
  </main>
</template>

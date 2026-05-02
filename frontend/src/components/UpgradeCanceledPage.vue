<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'
import UiButton from './ui/UiButton.vue'

defineOptions({ name: 'UpgradeCanceledPage' })

const { t } = useI18n()
const router = useRouter()
</script>

<template>
  <main id="main" class="mx-auto flex w-full max-w-md flex-col items-center px-6 py-16 text-center">
    <h1 class="font-display text-3xl text-fg-1">{{ t('upgrade.canceled.heading') }}</h1>
    <p class="mt-3 text-fg-2">{{ t('upgrade.canceled.body') }}</p>
    <div class="mt-8 flex flex-col gap-3">
      <UiButton
        variant="primary"
        size="md"
        type="button"
        data-testid="upgrade-canceled-retry"
        @click="router.push('/upgrade')"
      >
        {{ t('upgrade.canceled.retry') }}
      </UiButton>
      <UiButton
        variant="ghost"
        size="md"
        type="button"
        @click="router.push('/my-calendars')"
      >
        {{ t('upgrade.canceled.back') }}
      </UiButton>
    </div>
  </main>
</template>

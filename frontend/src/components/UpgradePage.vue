<script setup lang="ts">
import { ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import axios from 'axios'
import UiButton from './ui/UiButton.vue'
import UiSegmented from './ui/UiSegmented.vue'
import UiChip from './ui/UiChip.vue'
import IconCheck from './ui/icons/IconCheck.vue'
import IconSparkle from './ui/icons/IconSparkle.vue'
import { createCheckoutSession, type BillingInterval } from '@/services/subscription'

defineOptions({ name: 'UpgradePage' })

const { t } = useI18n()

const interval = ref<BillingInterval>('monthly')
const submitting = ref(false)
const errorMessage = ref('')

const intervalOptions = computed(() => [
  { value: 'monthly', label: t('pricing.interval.monthly') },
  { value: 'annual', label: t('pricing.interval.annual') },
])

const FEATURE_KEYS = [
  'pricing.features.proAccents',
  'pricing.features.unlimitedCalendars',
  'pricing.features.priorityRefresh',
  'pricing.features.exportFlexibility',
  'pricing.features.support',
] as const

async function startCheckout() {
  errorMessage.value = ''
  submitting.value = true
  try {
    const { url } = await createCheckoutSession(interval.value)
    window.location.href = url
  } catch (err) {
    submitting.value = false
    if (axios.isAxiosError(err) && err.response?.status === 401) {
      errorMessage.value = t('upgrade.errors.unauthorized')
    } else {
      errorMessage.value = t('upgrade.errors.checkoutFailed')
    }
  }
}
</script>

<template>
  <main id="main" class="mx-auto w-full max-w-2xl px-6 py-12">
    <header class="text-center">
      <p class="text-sm uppercase tracking-[0.18em] text-accent-1-text">
        {{ t('pricing.eyebrow') }}
      </p>
      <h1 class="mt-3 font-display text-4xl text-fg-1 sm:text-5xl">
        {{ t('pricing.heading') }}
      </h1>
      <p class="mt-3 text-fg-2">{{ t('pricing.subtitle') }}</p>
    </header>

    <section
      class="mt-10 rounded-lg border border-line bg-bg-1 p-8 shadow-sm"
      :aria-labelledby="'plan-pro-heading'"
    >
      <div class="flex items-start justify-between gap-4">
        <div>
          <div class="flex items-center gap-2">
            <IconSparkle class="text-accent-1" />
            <h2 id="plan-pro-heading" class="text-2xl font-semibold text-fg-1">
              {{ t('pricing.plan.title') }}
            </h2>
          </div>
          <p class="mt-1 text-sm text-fg-2">{{ t('pricing.plan.subtitle') }}</p>
        </div>
        <UiSegmented
          v-model="interval"
          :options="intervalOptions"
          :aria-label="t('pricing.interval.ariaLabel')"
          variant="tab"
        />
      </div>

      <div class="mt-6 flex items-baseline gap-3">
        <span class="text-4xl font-semibold text-fg-1">
          {{ interval === 'monthly' ? t('pricing.price.monthly') : t('pricing.price.annual') }}
        </span>
        <span class="text-sm text-fg-2">
          {{
            interval === 'monthly'
              ? t('pricing.price.perMonth')
              : t('pricing.price.perYear')
          }}
        </span>
        <UiChip v-if="interval === 'annual'" variant="pro">
          {{ t('pricing.price.savings') }}
        </UiChip>
      </div>

      <ul class="mt-8 space-y-3" :aria-label="t('pricing.features.ariaLabel')">
        <li
          v-for="key in FEATURE_KEYS"
          :key="key"
          class="flex items-start gap-3 text-fg-1"
        >
          <span
            class="mt-0.5 flex h-5 w-5 shrink-0 items-center justify-center rounded-full bg-accent-1-soft text-accent-1-text"
          >
            <IconCheck />
          </span>
          <span>{{ t(key) }}</span>
        </li>
      </ul>

      <p
        v-if="errorMessage"
        role="alert"
        class="mt-6 rounded-md border border-danger bg-danger/10 px-4 py-3 text-sm text-danger-text"
      >
        {{ errorMessage }}
      </p>

      <div class="mt-8 flex flex-col gap-4">
        <UiButton
          variant="primary"
          size="lg"
          :loading="submitting"
          :disabled="submitting"
          type="button"
          data-testid="upgrade-cta"
          @click="startCheckout"
        >
          {{ t('upgrade.cta.startTrial') }}
        </UiButton>
        <p class="text-center text-xs text-fg-3">{{ t('pricing.footer') }}</p>
      </div>
    </section>
  </main>
</template>

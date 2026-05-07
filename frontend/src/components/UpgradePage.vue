<script setup lang="ts">
import { ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import axios from 'axios'
import UiButton from './ui/UiButton.vue'
import UiSegmented from './ui/UiSegmented.vue'
import UiChip from './ui/UiChip.vue'
import IconCheck from './ui/icons/IconCheck.vue'
import IconX from './ui/icons/IconX.vue'
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

interface FeatureRow {
  ok: boolean
  textKey: string
}

// Free tier — mix of ✓ and ✗ to set up the comparison.
const FREE_FEATURES: readonly FeatureRow[] = [
  { ok: true, textKey: 'pricing.tiers.free.features.tracking' },
  { ok: true, textKey: 'pricing.tiers.free.features.limitedCalendars' },
  { ok: true, textKey: 'pricing.tiers.free.features.limitedShows' },
  { ok: true, textKey: 'pricing.tiers.free.features.icsExport' },
  { ok: true, textKey: 'pricing.tiers.free.features.freeAccents' },
  { ok: false, textKey: 'pricing.tiers.free.features.noProAccents' },
  { ok: false, textKey: 'pricing.tiers.free.features.noLiveSubscribe' },
  { ok: false, textKey: 'pricing.tiers.free.features.noCustomReminders' },
] as const

// Pro tier — v1 enforced feature set. Only ships what's actually built.
const PRO_FEATURE_KEYS = [
  'pricing.features.unlimitedCalendars',
  'pricing.features.unlimitedShows',
  'pricing.features.liveSubscribeUrl',
  'pricing.features.customisableReminders',
  'pricing.features.allAccents',
  'pricing.features.shareCalendars',
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
  <main id="main" class="relative mx-auto w-full max-w-6xl px-6 py-12">
    <!-- Atmosphere: paired radial gradients behind the hero. Decorative only. -->
    <div
      aria-hidden="true"
      class="pointer-events-none absolute inset-x-0 top-0 h-[360px] -z-10"
      :style="{
        background:
          'radial-gradient(60% 80% at 30% 0%, var(--accent-1-glow), transparent 65%), radial-gradient(50% 80% at 80% 0%, var(--accent-1-soft), transparent 65%)',
      }"
    />

    <header class="mx-auto max-w-2xl text-center">
      <p class="text-sm uppercase tracking-[0.18em] text-accent-1-text">
        {{ t('pricing.eyebrow') }}
      </p>
      <i18n-t
        keypath="pricing.heading"
        tag="h1"
        class="mt-3 font-display text-4xl leading-[1.05] text-fg-1 sm:text-6xl"
      >
        <template #emphasis>
          <i>{{ t('pricing.headingEmphasis') }}</i>
        </template>
      </i18n-t>
      <p class="mx-auto mt-4 max-w-xl text-fg-2">{{ t('pricing.subtitle') }}</p>

      <div class="mt-6 flex items-center justify-center gap-3">
        <UiSegmented
          v-model="interval"
          :options="intervalOptions"
          :aria-label="t('pricing.interval.ariaLabel')"
          variant="tab"
        />
        <UiChip v-if="interval === 'annual'" variant="pro">
          {{ t('pricing.price.savings') }}
        </UiChip>
      </div>
    </header>

    <section
      class="mt-12 grid grid-cols-1 gap-6 lg:grid-cols-2"
      :aria-label="t('pricing.tiers.ariaLabel')"
    >
      <!-- Free tier — comparison only, CTA disabled. -->
      <article
        class="rounded-lg border border-line-soft bg-bg-1 p-6 shadow-sm"
        :aria-labelledby="'tier-free-name'"
      >
        <h2 id="tier-free-name" class="text-2xl font-semibold text-fg-1">
          {{ t('pricing.tiers.free.name') }}
        </h2>
        <p class="mt-1 text-sm text-fg-2">{{ t('pricing.tiers.free.tagline') }}</p>
        <div class="mt-5 flex items-baseline gap-1.5">
          <span class="font-display text-5xl text-fg-1">
            {{ t('pricing.tiers.free.price') }}
          </span>
          <span class="text-sm text-fg-2">/{{ t('pricing.tiers.free.period') }}</span>
        </div>
        <UiButton
          variant="secondary"
          size="lg"
          type="button"
          disabled
          class="mt-5 w-full"
          data-testid="upgrade-cta-free"
        >
          {{ t('pricing.tiers.free.cta') }}
        </UiButton>
        <ul class="mt-6 space-y-3" :aria-label="t('pricing.features.ariaLabel')">
          <li
            v-for="row in FREE_FEATURES"
            :key="row.textKey"
            class="flex items-start gap-3 text-sm"
            :class="row.ok ? 'text-fg-1' : 'text-fg-3'"
          >
            <span
              v-if="row.ok"
              class="mt-0.5 flex h-4 w-4 shrink-0 items-center justify-center rounded-sm bg-bg-2 text-fg-1 [&_svg]:h-3 [&_svg]:w-3"
            >
              <IconCheck />
            </span>
            <span
              v-else
              class="mt-0.5 flex h-4 w-4 shrink-0 items-center justify-center rounded-sm border border-line text-fg-3 [&_svg]:h-3 [&_svg]:w-3"
            >
              <IconX />
            </span>
            <span>{{ t(row.textKey) }}</span>
          </li>
        </ul>
      </article>

      <!-- Pro tier — elevated, accent border + glow, "Most popular" chip. -->
      <article
        class="relative rounded-lg border border-accent-1 bg-bg-1 p-6 transition-transform duration-[var(--d-3)] ease-[var(--ease-out)] lg:-translate-y-2 lg:hover:-translate-y-3"
        :style="{ boxShadow: '0 12px 40px var(--accent-1-glow)' }"
        :aria-labelledby="'tier-pro-name'"
        data-testid="tier-pro"
      >
        <span
          class="absolute -top-3 left-1/2 inline-flex -translate-x-1/2 items-center rounded-full bg-accent-1 px-3 py-1 text-xs font-semibold whitespace-nowrap"
          :style="{ color: 'var(--accent-1-fg)' }"
        >
          {{ t('pricing.mostPopular') }}
        </span>

        <div class="flex items-center gap-2">
          <IconSparkle class="text-accent-1" />
          <h2 id="tier-pro-name" class="text-2xl font-semibold text-fg-1">
            {{ t('pricing.tiers.pro.name') }}
          </h2>
        </div>
        <p class="mt-1 text-sm text-fg-2">{{ t('pricing.tiers.pro.tagline') }}</p>

        <div class="mt-5 flex items-baseline gap-2">
          <span class="font-display text-5xl text-fg-1">
            {{ interval === 'monthly' ? t('pricing.price.monthly') : t('pricing.price.annual') }}
          </span>
          <span class="text-sm text-fg-2">
            {{
              interval === 'monthly' ? t('pricing.price.perMonth') : t('pricing.price.perYear')
            }}
          </span>
        </div>

        <p
          v-if="errorMessage"
          role="alert"
          class="mt-5 rounded-md border border-danger bg-danger/10 px-4 py-3 text-sm text-danger-text"
        >
          {{ errorMessage }}
        </p>

        <UiButton
          variant="primary"
          size="lg"
          :loading="submitting"
          :disabled="submitting"
          type="button"
          data-testid="upgrade-cta"
          class="mt-5 w-full"
          @click="startCheckout"
        >
          {{ t('upgrade.cta.startTrial') }}
        </UiButton>

        <ul class="mt-6 space-y-3" :aria-label="t('pricing.features.ariaLabel')">
          <li
            v-for="key in PRO_FEATURE_KEYS"
            :key="key"
            class="flex items-start gap-3 text-sm text-fg-1"
            data-testid="pro-feature"
          >
            <span
              class="mt-0.5 flex h-4 w-4 shrink-0 items-center justify-center rounded-sm bg-accent-1 [&_svg]:h-3 [&_svg]:w-3"
              :style="{ color: 'var(--accent-1-fg)' }"
            >
              <IconCheck />
            </span>
            <span>{{ t(key) }}</span>
          </li>
        </ul>
      </article>
    </section>

    <p class="mt-10 text-center text-xs text-fg-3">{{ t('pricing.footer') }}</p>
  </main>
</template>

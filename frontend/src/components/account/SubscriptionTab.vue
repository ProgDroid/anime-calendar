<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'
import {
  createPortalSession,
  getMySubscription,
  type Entitlement,
} from '@/services/subscription'
import { useAuthStore } from '@/stores/auth'
import UiButton from '@/components/ui/UiButton.vue'
import UiChip from '@/components/ui/UiChip.vue'

defineOptions({ name: 'SubscriptionTab' })

const { t, locale } = useI18n()
const router = useRouter()
const authStore = useAuthStore()

const loading = ref(true)
const error = ref<string | null>(null)
const portalError = ref<string | null>(null)
const portalLoading = ref(false)
const entitlement = ref<Entitlement | null>(null)

// Surface state machine. The free path lives on its own; the other three
// share the "paid layout" with banner/chip variants.
type Surface = 'free' | 'paid' | 'past_due' | 'trialing' | 'canceled'

const surface = computed<Surface>(() => {
  const e = entitlement.value
  if (!e || e.tier === 'free') return 'free'
  // Status-driven branches, falling back to plain "paid" when status is null
  // or unrecognised. The webhook handler is the source of truth for status;
  // we don't try to second-guess it here.
  switch (e.status) {
    case 'past_due':
      return 'past_due'
    case 'trialing':
      return 'trialing'
    case 'canceled':
      return 'canceled'
    default:
      return 'paid'
  }
})

const periodEndDate = computed(() => formatDate(entitlement.value?.current_period_end))
const trialEndDate = computed(() => formatDate(entitlement.value?.trial_end))

function formatDate(iso: string | null | undefined): string {
  if (!iso) return ''
  // Backend serialises NaiveDateTime → "YYYY-MM-DDTHH:MM:SS" without TZ.
  // Treat as UTC so users on different zones see a consistent rendered date
  // (matches how Stripe itself displays renewal dates).
  const d = new Date(`${iso}Z`)
  if (Number.isNaN(d.getTime())) return iso
  return d.toLocaleDateString(locale.value, { year: 'numeric', month: 'long', day: 'numeric' })
}

async function fetchEntitlement() {
  loading.value = true
  error.value = null
  try {
    entitlement.value = await getMySubscription()
  } catch {
    error.value = t('account.subscription.errors.fetchFailed')
  } finally {
    loading.value = false
  }
}

async function handleManage() {
  if (portalLoading.value) return
  portalLoading.value = true
  portalError.value = null
  try {
    const { url } = await createPortalSession()
    window.location.href = url
  } catch {
    portalError.value = t('account.subscription.errors.portalFailed')
    portalLoading.value = false
  }
}

function goToUpgrade() {
  router.push('/upgrade')
}

onMounted(() => {
  if (!authStore.isAuthenticated()) return
  fetchEntitlement()
})
</script>

<template>
  <section class="flex flex-col max-w-xl">
    <header class="mb-2">
      <p class="text-xs uppercase tracking-wider text-fg-2" data-testid="account-tab-eyebrow">
        {{ t('account.subscription.eyebrow') }}
      </p>
      <h1 class="text-3xl md:text-4xl font-medium tracking-tight mt-1 text-fg-1" data-testid="account-tab-heading">
        {{ t('account.subscription.headingLead')
        }}<span class="font-display italic"> {{ t('account.subscription.headingItalic') }}</span>
      </h1>
      <p class="text-sm text-fg-2 mt-2">{{ t('account.subscription.subtitle') }}</p>
    </header>

    <div v-if="loading" class="text-fg-2 mt-6" data-testid="subscription-loading">
      {{ t('app.loading') }}
    </div>
    <div
      v-else-if="error"
      class="text-danger-text mt-6"
      data-testid="subscription-error"
    >
      {{ error }}
    </div>

    <!-- Free user: Nudge-B upgrade affordance. -->
    <section
      v-else-if="surface === 'free'"
      data-testid="subscription-upgrade-nudge"
      class="rounded-lg border border-accent-1 bg-bg-1 p-6 mt-6 shadow-sm"
    >
      <div class="flex flex-col sm:flex-row sm:items-start sm:justify-between gap-4">
        <div class="flex flex-col gap-3">
          <UiChip variant="default">{{ t('account.subscription.upgradeNudge.tierChipFree') }}</UiChip>
          <p class="text-sm text-fg-2">
            {{ t('account.subscription.upgradeNudge.ctaSubtitle') }}
          </p>
        </div>
        <div>
          <UiButton variant="primary" data-testid="subscription-upgrade-cta" @click="goToUpgrade">
            {{ t('account.subscription.upgradeNudge.ctaFree') }}
          </UiButton>
        </div>
      </div>
    </section>

    <!-- Paid / past-due / trialing layout. Same skeleton, banner varies. -->
    <div
      v-else
      class="bg-bg-1 border border-line rounded-lg p-6 mt-6 flex flex-col gap-4"
      :data-testid="`subscription-${surface}`"
    >
      <div
        v-if="surface === 'past_due'"
        class="px-3 py-2 rounded-md bg-danger/10 text-danger-text text-sm"
        data-testid="subscription-past-due-banner"
      >
        {{ t('account.subscription.pastDueBanner') }}
      </div>
      <div
        v-else-if="surface === 'trialing'"
        class="px-3 py-2 rounded-md bg-accent-1/10 text-accent-1-text text-sm"
        data-testid="subscription-trial-banner"
      >
        {{ t('account.subscription.trialBanner') }}
      </div>

      <div class="flex items-center gap-2">
        <h2 class="text-lg font-semibold text-fg-1">
          {{ t('account.subscription.tier.paid') }}
        </h2>
        <UiChip variant="pro">{{ t('account.subscription.tier.paid') }}</UiChip>
      </div>

      <dl class="text-sm text-fg-2 flex flex-col gap-1">
        <div v-if="entitlement?.trial_end && surface === 'trialing'" class="flex gap-2">
          <dt class="font-medium text-fg-1">·</dt>
          <dd data-testid="subscription-trial-end">
            {{ t('account.subscription.paid.trialEndsOn', { date: trialEndDate }) }}
          </dd>
        </div>
        <div v-if="periodEndDate" class="flex gap-2">
          <dt class="font-medium text-fg-1">·</dt>
          <dd v-if="entitlement?.cancel_at_period_end" data-testid="subscription-cancels-on">
            {{ t('account.subscription.paid.cancelsOn', { date: periodEndDate }) }}
          </dd>
          <dd v-else data-testid="subscription-renews-on">
            {{ t('account.subscription.paid.renewsOn', { date: periodEndDate }) }}
          </dd>
        </div>
      </dl>

      <div>
        <UiButton
          variant="secondary"
          data-testid="subscription-manage-cta"
          :disabled="portalLoading"
          @click="handleManage"
        >
          {{ portalLoading
            ? t('account.subscription.paid.managing')
            : t('account.subscription.paid.manage') }}
        </UiButton>
      </div>
      <div
        v-if="portalError"
        class="text-danger-text text-sm"
        data-testid="subscription-portal-error"
      >
        {{ portalError }}
      </div>
    </div>
  </section>
</template>

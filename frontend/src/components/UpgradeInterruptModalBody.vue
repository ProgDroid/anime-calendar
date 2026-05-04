<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'
import type { UpgradeReason } from '@/composables/useUpgradeInterrupt'
import UiButton from '@/components/ui/UiButton.vue'
import IconSparkle from '@/components/ui/icons/IconSparkle.vue'

defineOptions({ name: 'UpgradeInterruptModalBody' })

interface Props {
  /**
   * Selects the heading + description copy. Default `pro_accent` mirrors
   * the backend's behaviour when it omits the reason on a 402.
   */
  reason?: UpgradeReason
}
const props = withDefaults(defineProps<Props>(), { reason: 'pro_accent' })

const emit = defineEmits<{
  close: []
}>()

const { t } = useI18n()
const router = useRouter()

const headingKey = computed(() => `interrupt.heading.${props.reason}`)
const descriptionKey = computed(() => `interrupt.description.${props.reason}`)

function close() {
  emit('close')
}

function goToUpgrade() {
  close()
  void router.push('/upgrade')
}
</script>

<template>
  <div class="p-4">
    <div class="flex items-center gap-2">
      <span class="text-accent-1-text [&_svg]:h-4 [&_svg]:w-4"><IconSparkle /></span>
      <h2 class="text-lg font-semibold text-fg-1" data-testid="upgrade-interrupt-heading">
        {{ t(headingKey) }}
      </h2>
    </div>

    <p class="mt-3 text-sm text-fg-2" data-testid="upgrade-interrupt-description">
      {{ t(descriptionKey) }}
    </p>
    <ul class="mt-3 flex flex-col gap-1.5 text-sm text-fg-1">
      <li class="flex gap-2">
        <span aria-hidden="true">·</span>
        <span>{{ t('interrupt.features.accents') }}</span>
      </li>
      <li class="flex gap-2">
        <span aria-hidden="true">·</span>
        <span>{{ t('interrupt.features.priority') }}</span>
      </li>
      <li class="flex gap-2">
        <span aria-hidden="true">·</span>
        <span>{{ t('interrupt.features.fullSet') }}</span>
      </li>
    </ul>

    <div class="mt-4 flex justify-end gap-2">
      <UiButton
        variant="ghost"
        data-testid="upgrade-interrupt-later"
        @click="close"
      >
        {{ t('interrupt.maybeLater') }}
      </UiButton>
      <UiButton
        variant="primary"
        data-testid="upgrade-interrupt-cta"
        @click="goToUpgrade"
      >
        {{ t('interrupt.cta') }}
      </UiButton>
    </div>
  </div>
</template>

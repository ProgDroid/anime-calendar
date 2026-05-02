<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'
import UiModal from '@/components/ui/UiModal.vue'
import UiButton from '@/components/ui/UiButton.vue'
import IconSparkle from '@/components/ui/icons/IconSparkle.vue'

defineOptions({ name: 'UpgradeInterruptModal' })

interface Props {
  open: boolean
}
defineProps<Props>()
const emit = defineEmits<{
  'update:open': [boolean]
  close: []
}>()

const { t } = useI18n()
const router = useRouter()

function close() {
  emit('update:open', false)
  emit('close')
}

function goToUpgrade() {
  close()
  void router.push('/upgrade')
}
</script>

<template>
  <UiModal
    :open="open"
    :ariaLabel="t('interrupt.ariaLabel')"
    data-testid="upgrade-interrupt-modal"
    @update:open="(v) => emit('update:open', v)"
    @close="emit('close')"
  >
    <template #header>
      <div class="flex items-center gap-2">
        <span class="[&_svg]:w-4 [&_svg]:h-4 text-accent-1-text"><IconSparkle /></span>
        <h2 class="text-lg font-semibold text-fg-1">{{ t('interrupt.heading') }}</h2>
      </div>
    </template>

    <p class="text-sm text-fg-2">{{ t('interrupt.description') }}</p>
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

    <template #footer>
      <div class="flex justify-end gap-2">
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
    </template>
  </UiModal>
</template>

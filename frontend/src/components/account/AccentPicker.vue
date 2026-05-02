<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { Accent } from '@/types/userSettings'
import { PRO_ACCENTS } from '@/constants/proAccents'
import UiChip from '@/components/ui/UiChip.vue'
import IconCheck from '@/components/ui/icons/IconCheck.vue'

defineOptions({ name: 'AccentPicker' })

const ACCENTS: readonly Accent[] = ['coral', 'iris', 'matcha', 'sakura', 'citron'] as const

const props = withDefaults(defineProps<{ modelValue: Accent; isPaid?: boolean }>(), {
  isPaid: false,
})
const emit = defineEmits<{
  'update:modelValue': [Accent]
  // Free user clicked a Pro accent — caller is expected to surface the
  // upgrade interrupt modal. The accent is NOT applied; v-model is left
  // unchanged so the picker stays visually in its previous state.
  interrupt: [Accent]
}>()

const { t } = useI18n()

function handleClick(accent: Accent) {
  if (PRO_ACCENTS.has(accent) && !props.isPaid) {
    emit('interrupt', accent)
    return
  }
  emit('update:modelValue', accent)
}

function ariaLabel(accent: Accent): string {
  const name = t(`account.preferences.accents.${accent}`)
  return PRO_ACCENTS.has(accent) ? `${name} (${t('account.preferences.proChip')})` : name
}
</script>

<template>
  <div class="flex flex-wrap gap-3">
    <button
      v-for="accent in ACCENTS"
      :key="accent"
      type="button"
      :data-testid="`accent-swatch-${accent}`"
      :data-accent-preview="accent"
      :aria-label="ariaLabel(accent)"
      class="relative flex flex-col items-center gap-2 p-3 rounded-lg bg-bg-2 hover:bg-bg-3 transition"
      :aria-pressed="modelValue === accent"
      @click="handleClick(accent)"
    >
      <span class="accent-swatch-dot w-10 h-10 rounded-full relative">
        <span
          v-if="modelValue === accent"
          class="absolute inset-0 flex items-center justify-center text-bg-0 [&_svg]:w-5 [&_svg]:h-5"
          aria-hidden="true"
        >
          <IconCheck />
        </span>
      </span>
      <span class="text-sm font-medium text-fg-1">
        {{ t(`account.preferences.accents.${accent}`) }}
      </span>
      <UiChip
        v-if="PRO_ACCENTS.has(accent)"
        :data-testid="`accent-pro-${accent}`"
        variant="pro"
        aria-hidden="true"
      >
        {{ t('account.preferences.proChip') }}
      </UiChip>
      <span
        v-if="modelValue === accent"
        class="absolute inset-0 rounded-lg ring-2 ring-accent-1 pointer-events-none"
      />
    </button>
  </div>
</template>

<style scoped>
[data-accent-preview="coral"]  .accent-swatch-dot { background: oklch(72% 0.18 28);  }
[data-accent-preview="iris"]   .accent-swatch-dot { background: oklch(72% 0.18 285); }
[data-accent-preview="matcha"] .accent-swatch-dot { background: oklch(72% 0.18 145); }
[data-accent-preview="sakura"] .accent-swatch-dot { background: oklch(72% 0.18 350); }
[data-accent-preview="citron"] .accent-swatch-dot { background: oklch(72% 0.18 80);  }

:global([data-theme="dark"]) [data-accent-preview="coral"]  .accent-swatch-dot { background: oklch(60% 0.20 28);  }
:global([data-theme="dark"]) [data-accent-preview="iris"]   .accent-swatch-dot { background: oklch(60% 0.20 285); }
:global([data-theme="dark"]) [data-accent-preview="matcha"] .accent-swatch-dot { background: oklch(60% 0.20 145); }
:global([data-theme="dark"]) [data-accent-preview="sakura"] .accent-swatch-dot { background: oklch(60% 0.20 350); }
:global([data-theme="dark"]) [data-accent-preview="citron"] .accent-swatch-dot { background: oklch(60% 0.20 80);  }
</style>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { getRandomCoverPosters, type PosterRef } from '@/services/posters'

defineOptions({ name: 'UiAuthShellMobile' })

const { t } = useI18n()

const posters = ref<PosterRef[]>([])

/**
 * Rotation/translate offsets for the three-poster fan, ported from the
 * design handoff screens-auth-mobile.jsx spec.
 */
const FAN_TRANSFORMS: readonly string[] = [
  'rotate(-14deg) translateX(-28px) translateY(-8px)',
  'rotate(0deg) translateY(-16px)',
  'rotate(12deg) translateX(28px) translateY(-8px)',
] as const

onMounted(async () => {
  posters.value = await getRandomCoverPosters(3)
})
</script>

<template>
  <div class="relative min-h-dvh flex flex-col overflow-hidden bg-bg-0">
    <!-- Atmospheric gradient backdrop -->
    <div
      aria-hidden="true"
      class="pointer-events-none absolute inset-0 z-0"
      style="background:
        radial-gradient(80% 50% at 50% 0%, var(--accent-1-glow), transparent 70%),
        radial-gradient(60% 40% at 80% 80%, var(--accent-1-soft), transparent 70%),
        var(--bg-inset, var(--bg-0));"
    />

    <!-- Three-poster fan hero -->
    <div
      class="relative z-10 flex flex-1 items-end justify-center pb-8 pt-16"
      aria-label="t('auth.mobile.posterAlt')"
    >
      <div class="relative flex items-end justify-center" style="height: 240px; width: 280px;">
        <div
          v-for="(poster, i) in posters"
          :key="i"
          :data-testid="`auth-mobile-poster-${i}`"
          :aria-label="t('auth.mobile.posterAlt')"
          class="absolute h-[196px] w-[140px] overflow-hidden rounded-md opacity-90"
          :style="{
            background: poster.background,
            transform: FAN_TRANSFORMS[i] ?? '',
            boxShadow: '0 12px 40px rgba(0,0,0,0.35)',
            zIndex: i === 1 ? 2 : 1,
          }"
        >
          <!-- Faint scanlines texture -->
          <div
            aria-hidden="true"
            class="absolute inset-0 opacity-[0.08] mix-blend-overlay"
            style="background-image: repeating-linear-gradient(0deg, transparent 0 2px, rgba(255,255,255,0.4) 2px 3px);"
          />
          <!-- Decorative glyph -->
          <div
            aria-hidden="true"
            class="absolute inset-0 flex items-center justify-center leading-none"
            style="font-family: 'Instrument Serif', ui-serif, Georgia, serif; font-size: 72px; color: rgba(255,255,255,0.92); text-shadow: 0 4px 24px rgba(0,0,0,0.5);"
          >
            {{ poster.glyph }}
          </div>
        </div>
      </div>
    </div>

    <!-- Form slot anchored at bottom -->
    <div class="absolute bottom-0 left-0 right-0 px-6 pb-10">
      <slot />
    </div>
  </div>
</template>

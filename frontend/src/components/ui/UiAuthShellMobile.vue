<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { getRandomCoverPosters, type PosterRef } from '@/services/posters'

defineOptions({ name: 'UiAuthShellMobile' })

defineProps<{
  /** Optional eyebrow micro-text rendered above the heading. */
  eyebrow?: string
  /** Optional h1 heading rendered above the form slot. */
  heading?: string
  /** Optional subtitle rendered under the heading. */
  subtitle?: string
}>()

const posters = ref<PosterRef[]>([])

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
  <div class="relative min-h-screen min-h-dvh flex flex-col overflow-hidden bg-bg-0">
    <!-- Atmospheric gradient backdrop -->
    <div
      aria-hidden="true"
      class="pointer-events-none absolute inset-0 z-0"
      style="background:
        radial-gradient(80% 50% at 50% 0%, var(--accent-1-glow), transparent 70%),
        radial-gradient(60% 40% at 80% 80%, var(--accent-1-soft), transparent 70%),
        var(--bg-inset, var(--bg-0));"
    />

    <!-- Three-poster fan hero (purely decorative — procedural gradients, no real cover content) -->
    <div
      aria-hidden="true"
      class="relative z-10 flex flex-1 items-end justify-center pb-8 pt-16"
    >
      <div class="relative flex items-end justify-center" style="height: 240px; width: 280px;">
        <div
          v-for="(poster, i) in posters"
          :key="i"
          :data-testid="`auth-mobile-poster-${i}`"
          class="absolute h-[196px] w-[140px] overflow-hidden rounded-md opacity-90"
          :style="{
            background: poster.background,
            transform: FAN_TRANSFORMS[i] ?? '',
            boxShadow: '0 12px 40px rgba(0,0,0,0.35)',
            zIndex: i === 1 ? 2 : 1,
          }"
        >
          <div
            class="absolute inset-0 opacity-[0.08] mix-blend-overlay"
            style="background-image: repeating-linear-gradient(0deg, transparent 0 2px, rgba(255,255,255,0.4) 2px 3px);"
          />
          <div
            class="absolute inset-0 flex items-center justify-center leading-none"
            style="font-family: 'Instrument Serif', ui-serif, Georgia, serif; font-size: 72px; color: rgba(255,255,255,0.92); text-shadow: 0 4px 24px rgba(0,0,0,0.5);"
          >
            {{ poster.glyph }}
          </div>
        </div>
      </div>
    </div>

    <!-- Form anchored at bottom; safe-area-inset honored for iPhone home indicator. -->
    <div
      class="relative z-10 px-6 pb-[max(40px,env(safe-area-inset-bottom))] flex flex-col gap-4"
    >
      <div
        v-if="eyebrow || heading || subtitle"
        class="flex flex-col gap-2"
      >
        <p
          v-if="eyebrow"
          data-testid="auth-eyebrow"
          class="text-xs uppercase tracking-wider text-fg-2"
        >
          {{ eyebrow }}
        </p>
        <h1
          v-if="heading"
          data-testid="auth-heading"
          class="text-3xl font-medium tracking-tight text-fg-1"
        >
          {{ heading }}
        </h1>
        <p
          v-if="subtitle"
          data-testid="auth-subtitle"
          class="text-sm text-fg-2"
        >
          {{ subtitle }}
        </p>
      </div>

      <slot />
    </div>
  </div>
</template>

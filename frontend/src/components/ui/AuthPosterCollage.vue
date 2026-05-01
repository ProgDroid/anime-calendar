<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import Wordmark from '../shared/Wordmark.vue'

defineOptions({ name: 'AuthPosterCollage' })

const { t } = useI18n()

/* Palettes ported verbatim from design_handoff_anime_calendar/posters.jsx */
const palettes: ReadonlyArray<readonly [string, string, string]> = [
  ['#1a1130', '#46225a', '#d96b6b'],
  ['#0f1f2e', '#1d4256', '#7fc6c1'],
  ['#2a1024', '#6b1f4d', '#f0a3a3'],
  ['#1a2a18', '#3b6a3a', '#dceb7b'],
  ['#241a30', '#4d3a72', '#b89bff'],
  ['#2c1612', '#82382a', '#f4c87a'],
  ['#0e1726', '#27407a', '#9bb6ff'],
  ['#2c1622', '#7c2a55', '#ffb9d4'],
  ['#1a1f0e', '#4a5523', '#f7e36a'],
  ['#1f1023', '#5a2470', '#ff8ab8'],
] as const

const glyphs = ['◐', '✦', '▲', '◇', '◍', '✿', '☄', '✺', '◉', '▽'] as const

function posterStyle(seed: number): Record<string, string> {
  const p = palettes[seed % palettes.length]!
  const angle = (seed * 37) % 360
  const cx = 20 + ((seed * 7) % 60)
  const cy = 10 + ((seed * 11) % 40)
  return {
    background: `radial-gradient(120% 80% at ${cx}% ${cy}%, ${p[2]}55, transparent 60%), linear-gradient(${angle}deg, ${p[0]}, ${p[1]} 60%, ${p[2]}aa 130%)`,
  }
}

function posterGlyph(seed: number): string {
  return glyphs[seed % glyphs.length]!
}

/* Fixed offsets, ported from screens-auth-list.jsx */
type PosterSpec = { seed: number; top: string; left: string; rotate: number }
const posters: ReadonlyArray<PosterSpec> = [
  { seed: 1, top: '18%', left: '54%', rotate: -8 },
  { seed: 4, top: '38%', left: '18%', rotate: 6 },
  { seed: 7, top: '55%', left: '60%', rotate: -4 },
  { seed: 9, top: '8%', left: '20%', rotate: 4 },
] as const

const items = computed(() =>
  posters.map((p) => ({
    ...p,
    style: {
      ...posterStyle(p.seed),
      top: p.top,
      left: p.left,
      transform: `rotate(${p.rotate}deg)`,
    },
    glyph: posterGlyph(p.seed),
  })),
)
</script>

<template>
  <div class="relative flex h-full flex-col justify-between gap-8">
    <!-- Wordmark in upper-left -->
    <div class="relative z-10 flex items-center gap-2">
      <Wordmark size="md" />
    </div>

    <!-- Floating posters -->
    <div
      data-testid="auth-poster-stack"
      class="pointer-events-none absolute inset-0"
      aria-hidden="true"
    >
      <div
        v-for="(p, i) in items"
        :key="i"
        :data-testid="`auth-poster-${p.seed}`"
        class="absolute h-[196px] w-[140px] overflow-hidden rounded-md shadow-[0_16px_40px_rgba(8,6,18,0.18),0_2px_6px_rgba(8,6,18,0.06)] opacity-85"
        :style="p.style"
      >
        <!-- Faint scanlines -->
        <div
          class="absolute inset-0 opacity-[0.08] mix-blend-overlay"
          style="background-image: repeating-linear-gradient(0deg, transparent 0 2px, rgba(255,255,255,0.4) 2px 3px);"
        />
        <!-- Glyph -->
        <div
          class="absolute inset-0 flex items-center justify-center font-display leading-none"
          style="font-size: 80px; color: rgba(255,255,255,0.92); text-shadow: 0 4px 24px rgba(0,0,0,0.5);"
        >
          {{ p.glyph }}
        </div>
      </div>
    </div>

    <!-- Tagline -->
    <div class="relative z-10 max-w-[420px]">
      <div
        class="font-display text-4xl leading-[1.05] text-fg-0 md:text-5xl"
      >
        {{ t('auth.tagline.headlineLead') }}
        <span class="italic">{{ t('auth.tagline.headlineItalic') }}</span>
      </div>
      <p class="mt-3 max-w-[360px] text-base text-fg-1">
        {{ t('auth.tagline.body') }}
      </p>
    </div>
  </div>
</template>

<script setup lang="ts">
import AuthPosterCollage from './AuthPosterCollage.vue'
import UiAuthShellMobile from './UiAuthShellMobile.vue'
import { useViewportLayout } from '@/composables/useViewportLayout'

defineOptions({ name: 'UiAuthShell' })

defineProps<{
  /** Test id applied to the left poster collage pane. Pane is hidden when omitted. */
  posterTestid?: string
  /** Optional test id applied to the inner card wrapper. */
  cardTestid?: string
  /** Optional eyebrow micro-text rendered above the heading on the right pane. */
  eyebrow?: string
  /** Optional h1 heading rendered on the right pane. */
  heading?: string
  /** Optional subtitle rendered under the heading on the right pane. */
  subtitle?: string
}>()

const { isMobile } = useViewportLayout()
</script>

<template>
  <!-- Mobile fan-poster shell -->
  <UiAuthShellMobile v-if="isMobile">
    <slot />
  </UiAuthShellMobile>

  <!-- Desktop two-pane layout -->
  <div v-else data-testid="auth-shell-desktop" class="min-h-[calc(100vh-6rem)] grid lg:grid-cols-2 bg-bg-0">
    <!-- Left pane: atmospheric collage -->
    <div
      v-if="posterTestid"
      :data-testid="posterTestid"
      class="hidden lg:flex relative flex-col p-14 overflow-hidden bg-bg-2"
      style="background:
        radial-gradient(80% 60% at 30% 20%, var(--accent-1-glow), transparent 70%),
        radial-gradient(70% 60% at 80% 80%, var(--accent-2-soft), transparent 70%),
        var(--bg-inset);"
    >
      <AuthPosterCollage />
    </div>

    <!-- Right pane: form -->
    <div class="flex items-center justify-center p-6 lg:p-14">
      <div
        :data-testid="cardTestid"
        class="w-full max-w-[420px] flex flex-col gap-6"
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
            class="text-3xl md:text-4xl font-medium tracking-tight text-fg-1"
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
  </div>
</template>

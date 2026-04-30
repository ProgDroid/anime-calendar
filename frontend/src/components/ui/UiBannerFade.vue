<script setup lang="ts">
import { computed } from 'vue'

interface Props { selected: boolean; posterUrl: string | null }
const props = defineProps<Props>()
defineOptions({ name: 'UiBannerFade' })

const MASK = 'linear-gradient(to right, rgba(0,0,0,0) 0%, rgba(0,0,0,0) 50%, rgba(0,0,0,0.2) 65%, rgba(0,0,0,1) 95%)'

const bgStyle = computed(() => {
  if (!props.posterUrl) {
    return { background: 'var(--accent-1-soft)' }
  }
  return {
    backgroundImage: `linear-gradient(135deg, var(--accent-1-soft), transparent), url(${props.posterUrl})`,
    backgroundSize: 'cover',
    backgroundPosition: 'center',
  }
})

const maskStyle = computed(() => ({
  maskImage: MASK,
  WebkitMaskImage: MASK,
}))
</script>

<template>
  <div class="relative w-full h-full overflow-hidden">
    <div
      data-testid="banner"
      :class="[
        'absolute inset-0 transition-opacity duration-[var(--d-3)] ease-[var(--ease-out)]',
        selected ? 'opacity-100' : 'opacity-0'
      ]"
      :style="{ ...bgStyle, ...maskStyle }"
    />
    <div class="relative z-10"><slot /></div>
  </div>
</template>

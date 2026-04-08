import { ref, onMounted, onUnmounted, computed } from 'vue'

export function useWindowSize() {
  const width = ref(window.innerWidth)
  const height = ref(window.innerHeight)

  const onResize = () => {
    width.value = window.innerWidth
    height.value = window.innerHeight
  }

  onMounted(() => window.addEventListener('resize', onResize))
  onUnmounted(() => window.removeEventListener('resize', onResize))

  const isMobile = computed(() => width.value < 768)

  return { width, height, isMobile }
}

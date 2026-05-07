<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useAuthStore } from '@/stores/auth'
import type { Viewer } from '@/types/sharing'

const props = defineProps<{ viewers: Viewer[] }>()

const { t } = useI18n()
const auth = useAuthStore()
const open = ref(false)

const currentUserId = computed(() => auth.user?.id ?? null)
const others = computed(() => props.viewers.filter((v) => v.user_id !== currentUserId.value))
const includesSelf = computed(() => props.viewers.some((v) => v.user_id === currentUserId.value))

const label = computed(() => {
  const n = others.value.length
  if (n === 0) return t('sharing.presence.justYou')
  return includesSelf.value
    ? t('sharing.presence.youPlusN', n)
    : t('sharing.presence.nViewing', { n })
})
</script>

<template>
  <div class="relative inline-block">
    <button
      data-testid="presence-chip"
      class="px-2 py-1 rounded-full text-xs font-medium bg-bg-2 text-fg-2 hover:bg-bg-3 transition-colors"
      @click="open = !open"
    >
      {{ label }}
    </button>
    <div
      v-if="open"
      data-testid="presence-popover"
      class="absolute top-full mt-1 right-0 z-10 min-w-[10rem] rounded-lg bg-bg-1 border border-bg-3 shadow-lg p-2"
    >
      <div
        v-for="viewer in viewers"
        :key="viewer.user_id"
        class="px-2 py-1 text-sm text-fg-1"
      >
        {{ viewer.display }}
      </div>
    </div>
  </div>
</template>

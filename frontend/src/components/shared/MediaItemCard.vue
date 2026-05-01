<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import type { Item } from '@/types/item'
import UiBannerFade from '@/components/ui/UiBannerFade.vue'
import UiChip from '@/components/ui/UiChip.vue'

const { t } = useI18n()

const props = defineProps<{
  item: Item
  displayTitle: string
  isSelected: boolean
  isInCalendar: boolean
  compact?: boolean
}>()

const emit = defineEmits<{
  click: []
}>()

const onImageError = (e: Event) => {
  const img = e.target as HTMLImageElement
  img.style.display = 'none'
}

const posterUrl = computed<string | null>(() => {
  if (props.item.banner_image) return props.item.banner_image
  if (props.item.cover_image?.medium) return props.item.cover_image.medium
  return null
})
</script>

<template>
  <div
    class="bg-bg-1 border border-line rounded-md shadow-sm relative overflow-hidden cursor-pointer transition-colors"
    :class="{
      'border-accent-1': isSelected && !isInCalendar,
      'border-success': isInCalendar
    }"
    @click="emit('click')"
  >
    <UiBannerFade :selected="isSelected && !isInCalendar" :poster-url="posterUrl">
      <div class="p-3">
        <div class="flex items-start gap-2">
          <div class="flex-shrink-0">
            <div
              v-if="item.cover_image?.medium"
              class="bg-bg-2 border border-line rounded w-16 h-20 overflow-hidden"
            >
              <img
                :src="item.cover_image.medium"
                :alt="item.title.romaji"
                class="w-full h-full object-cover"
                loading="lazy"
                @error="onImageError"
              />
            </div>
            <div
              v-else
              class="bg-bg-2 border border-line rounded w-16 h-20 flex items-center justify-center"
            >
              <span class="text-xs text-fg-2">{{ t('calendar.noImage') }}</span>
            </div>
          </div>
          <div class="flex-grow min-w-0">
            <h3 class="font-semibold line-clamp-1 text-fg-1" :class="compact ? 'text-sm' : ''">
              {{ displayTitle }}
            </h3>
            <UiChip
              :variant="item.media_type === 'MANGA' ? 'manga' : 'anime'"
              :size="compact ? 'sm' : 'sm'"
              class="mt-1"
            >
              {{ item.media_type === 'MANGA' ? t('calendar.mediaTypeManga') : t('calendar.mediaTypeAnime') }}
            </UiChip>
            <p v-if="item.media_type === 'ANIME' && !compact" class="text-xs text-fg-2 mt-1">
              {{ t('calendar.episodes') }}: {{ item.episode_duration }}
            </p>
          </div>
        </div>

        <!-- "Already in calendar" badge -->
        <div
          v-if="isInCalendar"
          class="absolute top-2 right-2 bg-success text-bg-0 text-xs px-2 py-1 rounded-md"
        >
          {{ t('calendar.alreadyInCalendar') }}
        </div>

        <slot />
      </div>
    </UiBannerFade>
  </div>
</template>

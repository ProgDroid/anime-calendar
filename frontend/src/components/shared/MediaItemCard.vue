<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { Item } from '@/types/item'

const { t } = useI18n()

defineProps<{
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
</script>

<template>
  <div
    class="card bg-base-100 shadow-sm border relative overflow-hidden cursor-pointer"
    :class="{
      'border-primary': isSelected && !isInCalendar,
      'border-success': isInCalendar
    }"
    @click="emit('click')"
  >
    <div class="card-body p-3">
      <div class="flex items-start gap-2">
        <div class="flex-shrink-0">
          <div
            v-if="item.cover_image?.medium"
            class="bg-base-300 border rounded w-16 h-20 overflow-hidden"
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
            class="bg-base-300 border rounded w-16 h-20 flex items-center justify-center"
          >
            <span class="text-xs">{{ t('calendar.noImage') }}</span>
          </div>
        </div>
        <div class="flex-grow min-w-0">
          <h4 class="font-bold line-clamp-1" :class="compact ? 'text-sm' : ''">
            {{ displayTitle }}
          </h4>
          <div class="badge badge-secondary mt-1" :class="compact ? 'text-xs' : ''">
            {{ item.media_type }}
          </div>
          <p v-if="item.media_type === 'ANIME' && !compact" class="text-xs mt-1">
            {{ t('calendar.episodes') }}: {{ item.episode_duration }}
          </p>
        </div>
      </div>

      <!-- "Already in calendar" badge -->
      <div
        v-if="isInCalendar"
        class="absolute top-2 right-2 bg-success text-success-content text-xs px-2 py-1 rounded"
      >
        {{ t('calendar.alreadyInCalendar') }}
      </div>

      <!-- Background image overlay when selected -->
      <div
        v-show="isSelected && !!item.banner_image"
        class="absolute inset-0 pointer-events-none transition-opacity duration-300"
        :style="{
          backgroundImage: `url(${item.banner_image})`,
          backgroundSize: 'cover',
          backgroundPosition: 'center',
          backgroundRepeat: 'no-repeat',
          maskImage: 'linear-gradient(to right, rgba(0,0,0,0) 0%, rgba(0,0,0,0) 50%, rgba(0,0,0,0.2) 65%, rgba(0,0,0,1) 95%)'
        }"
      />
      <div
        v-show="isSelected && !item.banner_image && !!item.cover_image?.medium"
        class="absolute inset-0 pointer-events-none transition-opacity duration-300"
        :style="{
          backgroundImage: `url(${item.cover_image?.medium})`,
          backgroundSize: 'auto 100%',
          backgroundPosition: 'right',
          backgroundRepeat: 'no-repeat',
          maskImage: 'linear-gradient(to right, rgba(0,0,0,0) 0%, rgba(0,0,0,0.0) 83.5%, rgba(0,0,0,1) 95%)'
        }"
      />

      <slot />
    </div>
  </div>
</template>

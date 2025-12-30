<template>
  <div class="cover-cascade" v-if="items && items.length > 0">
    <div class="cover-image-container" v-for="(item, index) in items" :key="item.id">
      <img 
        :src="item.cover_image?.medium" 
        :alt="item.title.romaji" 
        v-if="item.cover_image?.medium"
        @error="onImageError"
        @load="onImageLoad"
      />
      <div v-else class="no-image-placeholder">
        No image
      </div>
    </div>
  </div>
  <div v-else class="no-items-placeholder">
    No items to display
  </div>
</template>

<script setup lang="ts">
import type { Item } from '@/types/item'

const props = defineProps<{
  items: Item[]
}>()

const onImageError = (event: Event) => {
  const img = event.target as HTMLImageElement
  img.style.display = 'none'
  console.error('Image failed to load:', (event.target as HTMLImageElement).src)
}

const onImageLoad = (event: Event) => {
  console.log('Image loaded successfully', (event.target as HTMLImageElement).src)
}
</script>

<style scoped>
.cover-cascade {
  display: flex;
  flex-direction: row;
  margin-bottom: 20px;
  position: relative;
  overflow: hidden;
  height: 150px;
  width: 100%;
}

.cover-image-container {
  width: 100px;
  height: 150px;
  transition: transform 0.3s ease;
  box-shadow: 0 4px 8px rgba(0,0,0,0.2);
  border-radius: 8px;
  overflow: hidden;
  position: relative;
  border: 1px solid #ddd;
}

.cover-image-container img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
}

.no-image-placeholder {
  width: 100px;
  height: 150px;
  background-color: #f0f0f0;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #999;
  border-radius: 8px;
  border: 1px dashed #ccc;
}

.no-items-placeholder {
  padding: 20px;
  text-align: center;
  color: #999;
  border: 1px dashed #ccc;
  border-radius: 8px;
}
</style>

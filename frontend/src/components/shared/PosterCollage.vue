<script setup lang="ts">
import { computed } from 'vue';

defineOptions({ name: 'PosterCollage' });

const props = defineProps<{ urls: string[] }>();

const slots = computed(() =>
  Array.from({ length: 4 }, (_, i) => props.urls[i] ?? null),
);
</script>

<template>
  <div
    class="grid grid-cols-2 gap-1 aspect-square w-full overflow-hidden rounded-lg bg-bg-2"
  >
    <template v-for="(url, i) in slots" :key="i">
      <img
        v-if="url"
        :src="url"
        data-testid="poster-tile"
        class="object-cover w-full h-full"
        loading="lazy"
        alt=""
      />
      <div v-else data-testid="poster-empty" class="bg-bg-3" />
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import api from '@/config/api';
import type { PageCalendar } from '@/types/calendar';
import type { Item } from '@/types/item';
import PosterCollage from './PosterCollage.vue';
import UiAvatar from '@/components/ui/UiAvatar.vue';

defineOptions({ name: 'CalendarTile' });

const props = defineProps<{
  calendar: PageCalendar;
  ownerAvatarUrl?: string;
}>();

defineEmits<{ click: [] }>();

const { t } = useI18n();

const coverUrls = ref<string[]>([]);

watch(
  () => props.calendar.recent_item_ids,
  async (ids) => {
    if (!ids || ids.length === 0) {
      coverUrls.value = [];
      return;
    }
    try {
      const params = ids.map((id) => `id=${id}`).join('&');
      const response = await api.get<Item[]>(`/items?${params}`);
      coverUrls.value = response.data
        .map((item) => item.cover_image?.large)
        .filter((u): u is string => Boolean(u));
    } catch {
      coverUrls.value = [];
    }
  },
  { immediate: true },
);
</script>

<template>
  <button
    data-testid="calendar-tile"
    class="flex flex-col gap-3 p-4 rounded-xl bg-bg-1 border border-line hover:bg-bg-2 transition text-left"
    @click="$emit('click')"
  >
    <PosterCollage :urls="coverUrls" />
    <div class="flex items-center justify-between gap-2">
      <div class="flex flex-col min-w-0">
        <span data-testid="calendar-tile-name" class="font-medium text-fg-1 truncate">
          {{ calendar.name }}
        </span>
        <span data-testid="calendar-tile-count" class="text-sm text-fg-3">
          {{ t('calendars.tile.itemCount', { count: calendar.item_count }) }}
        </span>
      </div>
      <UiAvatar
        v-if="ownerAvatarUrl"
        data-testid="calendar-tile-avatar"
        :src="ownerAvatarUrl"
        :alt="calendar.name"
        size="sm"
      />
    </div>
  </button>
</template>

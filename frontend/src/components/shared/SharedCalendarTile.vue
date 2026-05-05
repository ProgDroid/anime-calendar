<script setup lang="ts">
import { ref, watch, computed } from 'vue';
import { useI18n } from 'vue-i18n';
import api from '@/config/api';
import type { SharedPageCalendar } from '@/types/calendar';
import type { Item } from '@/types/item';

defineOptions({ name: 'SharedCalendarTile' });

const props = defineProps<{ calendar: SharedPageCalendar }>();

const emit = defineEmits<{
  open: [];
}>();

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

const updatedLabel = computed(() =>
  t('calendars.tile.updatedLabel', {
    date: new Date(props.calendar.updated_at).toLocaleDateString(),
  }),
);

const overflowCount = computed(() => {
  const total = props.calendar.item_count ?? 0;
  return Math.max(0, total - 3);
});

const initial = computed(() =>
  (props.calendar.owner.display || '?').charAt(0).toUpperCase(),
);
</script>

<template>
  <div
    data-testid="shared-calendar-tile"
    class="flex flex-col overflow-hidden rounded-xl border border-line bg-bg-1 hover:border-line-strong transition"
  >
    <button
      data-testid="shared-calendar-tile-body"
      class="text-left flex flex-col cursor-pointer"
      @click="emit('open')"
    >
      <!-- 96px header strip -->
      <div
        class="relative h-24 overflow-hidden bg-bg-2"
        data-testid="shared-tile-header-strip"
      >
        <img
          v-if="coverUrls[0]"
          :src="coverUrls[0]"
          class="absolute inset-0 w-full h-full object-cover scale-110 blur-md opacity-60"
          alt=""
          aria-hidden="true"
        />
        <div
          class="absolute inset-0"
          style="background: linear-gradient(180deg, transparent 30%, var(--bg-1) 100%);"
        />
        <div class="absolute top-3 left-3 right-3 flex items-end gap-1">
          <template v-for="(url, i) in coverUrls.slice(0, 3)" :key="i">
            <img
              :src="url"
              :data-testid="`shared-tile-mini-poster-${i}`"
              class="w-7 h-[38px] rounded-sm object-cover ring-1 ring-white/20"
              loading="lazy"
              alt=""
            />
          </template>
          <span
            v-if="overflowCount > 0"
            data-testid="shared-tile-overflow"
            class="ml-1 text-[11px] font-mono text-white drop-shadow self-end"
          >
            +{{ overflowCount }}
          </span>
        </div>
      </div>

      <div class="flex flex-col min-w-0 p-4 gap-1">
        <span data-testid="shared-calendar-tile-name" class="font-medium text-fg-1 truncate">
          {{ calendar.name }}
        </span>

        <div
          data-testid="shared-calendar-tile-owner"
          class="inline-flex items-center gap-2 text-xs text-fg-2"
        >
          <span
            v-if="calendar.owner.avatar"
            class="inline-block"
          >
            <img
              :src="calendar.owner.avatar"
              :alt="calendar.owner.display"
              class="w-5 h-5 rounded-full object-cover"
            />
          </span>
          <span
            v-else
            data-testid="shared-calendar-tile-owner-initial"
            class="inline-flex w-5 h-5 items-center justify-center rounded-full bg-bg-2 text-[10px] font-medium text-fg-1"
            aria-hidden="true"
          >
            {{ initial }}
          </span>
          <span class="truncate">
            {{ t('sharing.owner', { name: calendar.owner.display }) }}
          </span>
        </div>

        <div class="flex items-center gap-2 text-sm text-fg-2">
          <span data-testid="shared-calendar-tile-count">
            {{ t('calendars.tile.itemCount', { count: calendar.item_count }) }}
          </span>
          <span aria-hidden="true">·</span>
          <template v-if="(calendar.airing_count ?? 0) > 0">
            <span
              data-testid="shared-calendar-tile-airing"
              class="inline-flex items-center gap-1 text-warning"
            >
              <span
                class="w-1.5 h-1.5 rounded-full bg-warning"
                aria-hidden="true"
              />
              {{ t('calendars.tile.airing', { count: calendar.airing_count }) }}
            </span>
            <span aria-hidden="true">·</span>
          </template>
          <span data-testid="shared-calendar-tile-updated">{{ updatedLabel }}</span>
        </div>
      </div>
    </button>
  </div>
</template>

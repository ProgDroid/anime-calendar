<script setup lang="ts">
import { ref, watch, computed } from 'vue';
import { useI18n } from 'vue-i18n';
import api from '@/config/api';
import type { PageCalendar } from '@/types/calendar';
import type { Item } from '@/types/item';
import PosterCollage from './PosterCollage.vue';
import UiButton from '@/components/ui/UiButton.vue';
import UiMenu from '@/components/ui/UiMenu.vue';
import IconEdit from '@/components/ui/icons/IconEdit.vue';
import IconDownload from '@/components/ui/icons/IconDownload.vue';
import IconDown from '@/components/ui/icons/IconDown.vue';
import IconLink from '@/components/ui/icons/IconLink.vue';
import IconExternal from '@/components/ui/icons/IconExternal.vue';
import IconTrash from '@/components/ui/icons/IconTrash.vue';
import IconMoreVertical from '@/components/ui/icons/IconMoreVertical.vue';

defineOptions({ name: 'CalendarTile' });

const props = defineProps<{ calendar: PageCalendar }>();

const emit = defineEmits<{
  open: [];
  edit: [];
  delete: [];
  'export-ics': [];
  'copy-link': [];
  'open-google': [];
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
</script>

<template>
  <div
    data-testid="calendar-tile"
    class="flex flex-col rounded-xl border border-line bg-bg-1 overflow-hidden hover:border-line-strong transition"
  >
    <button
      data-testid="calendar-tile-body"
      class="text-left flex flex-col gap-3 p-4 cursor-pointer"
      @click="emit('open')"
    >
      <PosterCollage :urls="coverUrls" />
      <div class="flex flex-col min-w-0">
        <span data-testid="calendar-tile-name" class="font-medium text-fg-1 truncate">
          {{ calendar.name }}
        </span>
        <div class="flex items-center gap-2 text-sm text-fg-3">
          <span data-testid="calendar-tile-count">
            {{ t('calendars.tile.itemCount', { count: calendar.item_count }) }}
          </span>
          <span aria-hidden="true">·</span>
          <span data-testid="calendar-tile-updated">{{ updatedLabel }}</span>
        </div>
      </div>
    </button>

    <!-- Desktop action row -->
    <div class="hidden sm:flex items-center gap-2 px-4 pb-4">
      <UiButton
        data-testid="calendar-tile-edit"
        variant="secondary"
        size="sm"
        class="flex-1"
        @click="emit('edit')"
      >
        <IconEdit class="w-3 h-3" /> {{ $t('calendars.edit') }}
      </UiButton>
      <UiMenu align="right">
        <template #trigger>
          <UiButton
            data-testid="calendar-tile-export-trigger"
            variant="secondary"
            size="sm"
          >
            <IconDownload class="w-3 h-3" /> {{ $t('calendars.export') }}
            <IconDown class="w-2.5 h-2.5" />
          </UiButton>
        </template>
        <button
          data-testid="calendar-tile-export-ics"
          class="flex items-center gap-2 px-3 py-2 text-sm text-fg-1 hover:bg-bg-2 rounded text-left"
          @click="emit('export-ics')"
        >
          <IconDownload class="w-3 h-3" /> {{ $t('calendars.exportDownload') }}
        </button>
        <button
          data-testid="calendar-tile-copy-link"
          class="flex items-center gap-2 px-3 py-2 text-sm text-fg-1 hover:bg-bg-2 rounded text-left"
          @click="emit('copy-link')"
        >
          <IconLink class="w-3 h-3" /> {{ $t('calendars.copyLink') }}
        </button>
        <button
          data-testid="calendar-tile-open-google"
          class="flex items-center gap-2 px-3 py-2 text-sm text-fg-1 hover:bg-bg-2 rounded text-left"
          @click="emit('open-google')"
        >
          <IconExternal class="w-3 h-3" /> {{ $t('calendars.openInGoogle') }}
        </button>
      </UiMenu>
      <UiButton
        data-testid="calendar-tile-delete"
        variant="ghost"
        size="sm"
        :aria-label="$t('calendars.delete')"
        @click="emit('delete')"
      >
        <IconTrash class="w-3 h-3" />
      </UiButton>
    </div>

    <!-- Mobile kebab -->
    <div class="flex sm:hidden items-center justify-end px-4 pb-4">
      <UiMenu align="right">
        <template #trigger>
          <UiButton
            data-testid="calendar-tile-kebab-trigger"
            variant="ghost"
            size="sm"
            :aria-label="$t('calendars.tile.moreActions')"
          >
            <IconMoreVertical class="w-4 h-4" />
          </UiButton>
        </template>
        <button
          data-testid="calendar-tile-kebab-edit"
          class="flex items-center gap-2 px-3 py-2 text-sm text-fg-1 hover:bg-bg-2 rounded text-left"
          @click="emit('edit')"
        >
          <IconEdit class="w-3 h-3" /> {{ $t('calendars.edit') }}
        </button>
        <button
          data-testid="calendar-tile-kebab-export-ics"
          class="flex items-center gap-2 px-3 py-2 text-sm text-fg-1 hover:bg-bg-2 rounded text-left"
          @click="emit('export-ics')"
        >
          <IconDownload class="w-3 h-3" /> {{ $t('calendars.exportDownload') }}
        </button>
        <button
          data-testid="calendar-tile-kebab-copy-link"
          class="flex items-center gap-2 px-3 py-2 text-sm text-fg-1 hover:bg-bg-2 rounded text-left"
          @click="emit('copy-link')"
        >
          <IconLink class="w-3 h-3" /> {{ $t('calendars.copyLink') }}
        </button>
        <button
          data-testid="calendar-tile-kebab-open-google"
          class="flex items-center gap-2 px-3 py-2 text-sm text-fg-1 hover:bg-bg-2 rounded text-left"
          @click="emit('open-google')"
        >
          <IconExternal class="w-3 h-3" /> {{ $t('calendars.openInGoogle') }}
        </button>
        <button
          data-testid="calendar-tile-kebab-delete"
          class="flex items-center gap-2 px-3 py-2 text-sm text-danger hover:bg-bg-2 rounded text-left"
          @click="emit('delete')"
        >
          <IconTrash class="w-3 h-3" /> {{ $t('calendars.delete') }}
        </button>
      </UiMenu>
    </div>
  </div>
</template>

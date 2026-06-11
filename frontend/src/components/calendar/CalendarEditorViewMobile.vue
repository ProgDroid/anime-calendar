<script setup lang="ts">
import { ref, computed, nextTick } from 'vue'
import { onBeforeRouteLeave, onBeforeRouteUpdate, RouterLink } from 'vue-router'
import { useI18n } from 'vue-i18n'
import type { Item } from '@/types/item'
import { useEditorSelectionStore } from '@/stores/editorSelection'
import { useCalendarEditor } from '@/composables/useCalendarEditor'
import UiSegmented from '@/components/ui/UiSegmented.vue'
import IconPlus from '@/components/ui/icons/IconPlus.vue'
import EditorItemsPanelMobile from './EditorItemsPanelMobile.vue'
import EditorSearchPanelMobile from './EditorSearchPanelMobile.vue'
import CalendarSettingsForm from './CalendarSettingsForm.vue'
import PresenceChip from '@/components/shared/PresenceChip.vue'

defineOptions({ name: 'CalendarEditorViewMobile' })

const { t } = useI18n()
const selection = useEditorSelectionStore()

const {
  isExistingCalendar,
  calendarName,
  calendarLanguage,
  calendarEventStyle,
  itemsInCalendar,
  submitLoading,
  calendarError,
  currentCalendar,
  isFreeTier,
  isOwner,
  editorShowCount,
  addItems,
  removeItemFromCalendar,
  clearCalendar,
  submitCalendar,
  viewers,
  showCollisionBanner,
  reloadPage,
  recommendations,
} = useCalendarEditor()

const loading = computed(() => submitLoading.value)
const itemCount = computed(() => itemsInCalendar.value.length)

// Tab state
type Tab = 'items' | 'search'
const tab = ref<Tab>('items')
const searchPanelRef = ref<InstanceType<typeof EditorSearchPanelMobile> | null>(null)

const tabOptions = computed(() => [
  { value: 'items', label: t('mobile.editor.itemsTab', { count: itemCount.value }) },
  { value: 'search', label: t('mobile.editor.searchTab') },
])

function jumpToSearch() {
  tab.value = 'search'
  void nextTick(() => {
    searchPanelRef.value?.focus()
  })
}

const addItemFromSearch = (items: Item[]) => {
  const before = itemsInCalendar.value.length
  addItems(items)
  selection.clear()
  // Switch back to the items tab only if something was actually added.
  if (itemsInCalendar.value.length > before) tab.value = 'items'
}

const addRecommendation = (item: Item) => addItems([item])

// Clear selection on route changes
onBeforeRouteUpdate(() => selection.clear())
onBeforeRouteLeave(() => selection.clear())
</script>

<template>
  <div class="relative flex flex-col min-h-screen bg-bg-0">
    <!-- Top bar -->
    <header class="bg-bg-0 px-4 pb-2 border-b border-line-soft"
      :style="{ paddingTop: 'max(16px, calc(env(safe-area-inset-top) + 12px))' }"
    >
      <div class="flex items-center gap-3">
        <RouterLink
          to="/my-calendars"
          class="text-fg-1 shrink-0 p-1 -ml-1"
          :aria-label="t('app.back')"
        >
          ‹
        </RouterLink>
        <span class="font-semibold text-fg-1 truncate flex-1">
          {{ calendarName || t('calendar.edit') }}
        </span>
        <PresenceChip v-if="isExistingCalendar && viewers.length > 0" :viewers="viewers" />
      </div>
    </header>

    <!-- Collision banner -->
    <div
      v-if="showCollisionBanner"
      data-testid="collision-banner"
      class="mx-4 mt-2 rounded-md bg-bg-2 border border-line px-3 py-2 text-sm text-fg-2 flex items-center justify-between gap-2"
    >
      <span>{{ $t('sharing.collisionWarning') }}</span>
      <button
        class="text-accent-1 font-medium hover:underline shrink-0"
        @click="reloadPage"
      >
        {{ $t('sharing.reloadButton') }}
      </button>
    </div>

    <!-- Settings form (always visible at top on mobile) -->
    <div class="px-4 pt-3">
      <CalendarSettingsForm
        :name="calendarName"
        :language="calendarLanguage"
        :event-style="calendarEventStyle"
        :loading="loading"
        :can-submit="itemsInCalendar.length > 0"
        :error="calendarError"
        :is-owner="isOwner"
        :calendar-id="currentCalendar?.id"
        :is-paid="!isFreeTier"
        @update:name="calendarName = $event"
        @update:language="calendarLanguage = $event"
        @update:event-style="calendarEventStyle = $event"
        @submit="submitCalendar"
      />
    </div>

    <!-- Tab switcher — wrap in w-full div since UiSegmented is inline-flex -->
    <div class="px-4 pt-4 pb-3" data-testid="editor-tab-bar">
      <div class="w-full flex">
        <UiSegmented
          v-model="tab"
          :options="tabOptions"
          variant="tab"
          :aria-label="t('mobile.editor.tabGroupLabel')"
          class="w-full [&>button]:flex-1"
        />
      </div>
    </div>

    <!-- Tab panels: page-level scroll keeps it native and avoids nested scrollers. -->
    <div class="flex-1 min-h-0">
      <KeepAlive>
        <EditorItemsPanelMobile
          v-if="tab === 'items'"
          :items="itemsInCalendar"
          :calendar-language="calendarLanguage"
          :recommendations="recommendations"
          :show-count="editorShowCount"
          @remove="removeItemFromCalendar"
          @clear="clearCalendar"
          @add-recommendation="addRecommendation"
        />
        <EditorSearchPanelMobile
          v-else
          ref="searchPanelRef"
          :items-in-calendar="itemsInCalendar"
          :calendar-language="calendarLanguage"
          :recommendations="recommendations"
          :show-count="editorShowCount"
          @add-selected="addItemFromSearch"
          @add-recommendation="addRecommendation"
        />
      </KeepAlive>
    </div>

    <!-- FAB — Items tab only, jumps to Search -->
    <button
      v-if="tab === 'items'"
      :aria-label="t('mobile.editor.fabAdd')"
      data-testid="editor-fab"
      class="fixed z-20 flex h-14 w-14 items-center justify-center rounded-full bg-accent-1 shadow-lg transition-all duration-[var(--d-2)] motion-reduce:transition-none motion-reduce:duration-0"
      :style="{ right: '18px', bottom: '90px', color: 'var(--accent-1-fg)' }"
      @click="jumpToSearch"
    >
      <IconPlus />
    </button>
  </div>
</template>

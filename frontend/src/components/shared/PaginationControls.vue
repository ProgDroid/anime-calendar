<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()

const props = defineProps<{
  page: number
  page_size: number
  total: number
  total_pages: number
}>()

const emit = defineEmits<{
  'page-change': [page: number]
}>()

const paginationRange = computed(() => {
  const delta = 2
  const start = Math.max(1, props.page - delta)
  const end = Math.min(props.total_pages, props.page + delta)
  const range: number[] = []
  for (let i = start; i <= end; i++) range.push(i)
  return range
})

const onPageChange = (newPage: number) => {
  if (newPage >= 1 && newPage <= props.total_pages) {
    emit('page-change', newPage)
  }
}
</script>

<template>
  <div v-if="total_pages > 1">
    <div class="join mt-8 flex justify-center">
      <button
        class="join-item btn"
        :disabled="page === 1"
        @click="onPageChange(page - 1)"
      >
        {{ t('calendars.pagePrevious') }}
      </button>

      <button
        v-for="p in paginationRange"
        :key="p"
        class="join-item btn"
        :class="{ 'btn-primary': p === page }"
        @click="onPageChange(p)"
      >
        {{ p }}
      </button>

      <button
        class="join-item btn"
        :disabled="page === total_pages"
        @click="onPageChange(page + 1)"
      >
        {{ t('calendars.pageNext') }}
      </button>
    </div>

    <div class="text-center mt-4 text-sm text-base-content/60">
      {{
        t('calendars.paginationText', {
          first: page_size * (page - 1) + 1,
          last: Math.min(page_size * page, total),
          total
        })
      }}
    </div>
  </div>
</template>

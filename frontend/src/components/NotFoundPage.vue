<script setup lang="ts">
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import UiAuthShell from '@/components/ui/UiAuthShell.vue'
import UiEmptyState from '@/components/ui/UiEmptyState.vue'
import UiButton from '@/components/ui/UiButton.vue'

defineOptions({ name: 'NotFoundPage' })
const router = useRouter()
const { t } = useI18n()
</script>

<template>
  <UiAuthShell>
    <div data-testid="not-found">
      <UiEmptyState :title="t('errors.notFound.title')" :body="t('errors.notFound.body')">
        <template #illustration>
          <div
            data-testid="not-found-numeral"
            class="flex items-baseline text-7xl md:text-8xl font-medium tracking-tight text-fg-2 leading-none"
            aria-hidden="true"
          >
            <span>4</span>
            <span class="font-display italic text-accent-1 mx-1">0</span>
            <span>4</span>
          </div>
        </template>
        <template #action>
          <UiButton variant="primary" class="w-full sm:w-auto" @click="router.push('/')">
            {{ t('errors.notFound.cta') }}
          </UiButton>
        </template>
      </UiEmptyState>
    </div>
  </UiAuthShell>
</template>

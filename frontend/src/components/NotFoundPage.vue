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
        <template #action>
          <UiButton variant="primary" @click="router.push('/')">
            {{ t('errors.notFound.cta') }}
          </UiButton>
        </template>
      </UiEmptyState>
    </div>
  </UiAuthShell>
</template>

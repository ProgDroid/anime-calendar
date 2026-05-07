<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'

defineOptions({ name: 'TermsOfServicePage' })

const { t } = useI18n()
const router = useRouter()
</script>

<template>
  <div class="max-w-2xl mx-auto py-10 px-2">
    <button
      type="button"
      class="mb-6 text-sm text-fg-2 hover:text-fg-1 transition-colors"
      @click="router.back()"
    >
      <span aria-hidden="true">←</span> {{ t('app.back') }}
    </button>
    <h1 class="text-2xl font-semibold mb-4">{{ t('legal.terms.title') }}</h1>
    <p class="text-fg-2 text-sm leading-relaxed">{{ t('legal.terms.placeholder') }}</p>
  </div>
</template>

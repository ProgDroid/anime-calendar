<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import UiButton from '@/components/ui/UiButton.vue'
import UiInput from '@/components/ui/UiInput.vue'

const { t } = useI18n()

defineProps<{
  name: string
  language: 'english' | 'romaji' | 'native'
  loading: boolean
  canSubmit: boolean
  error?: string | null
}>()

const emit = defineEmits<{
  'update:name': [value: string]
  'update:language': [value: 'english' | 'romaji' | 'native']
  submit: []
}>()
</script>

<template>
  <div class="flex flex-col gap-4">
    <div class="relative">
      <UiInput
        :model-value="name"
        :label="t('calendar.name')"
        :placeholder="t('calendar.namePlaceholder')"
        :maxlength="100"
        @update:model-value="emit('update:name', $event)"
      />
      <span class="absolute right-3 bottom-2 text-sm text-fg-2 pointer-events-none">
        {{ name.length }}/100
      </span>
    </div>

    <div class="flex flex-col gap-2">
      <label class="text-sm text-fg-2">{{ t('calendar.language') }}</label>
      <div class="flex gap-4 flex-wrap">
        <label class="inline-flex items-center gap-2 cursor-pointer text-fg-1">
          <input
            type="radio"
            :checked="language === 'english'"
            value="english"
            class="accent-accent-1"
            @change="emit('update:language', 'english')"
          />
          <span class="text-sm">{{ t('calendar.english') }}</span>
        </label>
        <label class="inline-flex items-center gap-2 cursor-pointer text-fg-1">
          <input
            type="radio"
            :checked="language === 'romaji'"
            value="romaji"
            class="accent-accent-1"
            @change="emit('update:language', 'romaji')"
          />
          <span class="text-sm">{{ t('calendar.romaji') }}</span>
        </label>
        <label class="inline-flex items-center gap-2 cursor-pointer text-fg-1">
          <input
            type="radio"
            :checked="language === 'native'"
            value="native"
            class="accent-accent-1"
            @change="emit('update:language', 'native')"
          />
          <span class="text-sm">{{ t('calendar.native') }}</span>
        </label>
      </div>
    </div>

    <div v-if="error" class="text-sm text-danger-text bg-danger/10 border border-danger/30 rounded-md px-3 py-2">{{ error }}</div>

    <UiButton
      data-testid="submit-btn"
      variant="primary"
      :disabled="loading || !canSubmit"
      @click="emit('submit')"
    >
      {{ loading ? t('calendar.submitting') : t('calendar.submit') }}
    </UiButton>
  </div>
</template>

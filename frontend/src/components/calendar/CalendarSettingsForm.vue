<script setup lang="ts">
import { useI18n } from 'vue-i18n'

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
    <div class="form-control">
      <label class="label">
        <span class="label-text">{{ t('calendar.name') }}:</span>
      </label>
      <div class="relative mt-2">
        <input
          type="text"
          :value="name"
          :placeholder="t('calendar.namePlaceholder')"
          class="input input-bordered w-full pr-16"
          maxlength="100"
          @input="emit('update:name', ($event.target as HTMLInputElement).value)"
        />
        <span class="absolute right-3 top-1/2 -translate-y-1/2 text-sm text-base-content/50">
          {{ name.length }}/100
        </span>
      </div>
    </div>

    <div class="form-control">
      <label class="label mb-2">
        <span class="label-text">{{ t('calendar.language') }}:</span>
      </label>
      <div class="flex gap-4 flex-wrap">
        <label class="label cursor-pointer gap-2">
          <input
            type="radio"
            :checked="language === 'english'"
            value="english"
            class="radio radio-primary"
            @change="emit('update:language', 'english')"
          />
          <span class="label-text">{{ t('calendar.english') }}</span>
        </label>
        <label class="label cursor-pointer gap-2">
          <input
            type="radio"
            :checked="language === 'romaji'"
            value="romaji"
            class="radio radio-primary"
            @change="emit('update:language', 'romaji')"
          />
          <span class="label-text">{{ t('calendar.romaji') }}</span>
        </label>
        <label class="label cursor-pointer gap-2">
          <input
            type="radio"
            :checked="language === 'native'"
            value="native"
            class="radio radio-primary"
            @change="emit('update:language', 'native')"
          />
          <span class="label-text">{{ t('calendar.native') }}</span>
        </label>
      </div>
    </div>

    <div v-if="error" class="alert alert-error">{{ error }}</div>

    <button
      data-testid="submit-btn"
      :disabled="loading || !canSubmit"
      class="btn btn-success"
      @click="emit('submit')"
    >
      {{ loading ? t('calendar.submitting') : t('calendar.submit') }}
    </button>
  </div>
</template>

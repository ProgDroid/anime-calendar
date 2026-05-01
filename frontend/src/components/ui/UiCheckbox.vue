<script setup lang="ts">
import { computed, useId } from 'vue'

interface Props {
  modelValue: boolean
  label?: string
  disabled?: boolean
  id?: string
  error?: string
}

const props = defineProps<Props>()
const emit = defineEmits<{ (e: 'update:modelValue', v: boolean): void }>()

defineOptions({ name: 'UiCheckbox', inheritAttrs: false })

const generatedId = useId()
const inputId = computed(() => props.id ?? generatedId)
</script>

<template>
  <label
    :for="inputId"
    class="inline-flex items-center gap-2 text-sm text-fg-1 cursor-pointer"
    :class="{ 'opacity-50 cursor-not-allowed': disabled }"
  >
    <input
      :id="inputId"
      type="checkbox"
      :checked="modelValue"
      :disabled="disabled"
      :aria-invalid="error ? true : undefined"
      :aria-describedby="error ? `${inputId}-error` : undefined"
      class="accent-accent-1 rounded-sm focus-visible:ring-2 focus-visible:ring-accent-1-soft focus-visible:ring-offset-2 focus-visible:ring-offset-bg-0"
      v-bind="$attrs"
      @change="emit('update:modelValue', ($event.target as HTMLInputElement).checked)"
    >
    <span v-if="label || $slots.default">
      <slot>{{ label }}</slot>
    </span>
    <span
      v-if="error"
      :id="`${inputId}-error`"
      class="text-xs text-danger-text ml-2"
      role="alert"
    >
      {{ error }}
    </span>
  </label>
</template>

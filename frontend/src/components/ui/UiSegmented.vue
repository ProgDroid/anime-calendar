<script setup lang="ts">
interface Option { value: string; label: string }
interface Props {
  modelValue: string
  options: Option[]
  ariaLabel?: string
  variant?: 'radio' | 'tab'
}
const props = withDefaults(defineProps<Props>(), { variant: 'radio' })
defineOptions({ name: 'UiSegmented' })
const emit = defineEmits<{ (e: 'update:modelValue', v: string): void }>()
</script>

<template>
  <div
    :role="props.variant === 'tab' ? 'tablist' : 'radiogroup'"
    :aria-label="props.ariaLabel"
    class="inline-flex rounded-md bg-bg-2 p-1 gap-1"
  >
    <button
      v-for="o in options"
      :key="o.value"
      :role="props.variant === 'tab' ? 'tab' : 'radio'"
      type="button"
      :aria-selected="props.variant === 'tab' ? modelValue === o.value : undefined"
      :aria-checked="props.variant === 'radio' ? modelValue === o.value : undefined"
      :tabindex="props.variant === 'radio' ? (modelValue === o.value ? 0 : -1) : undefined"
      :class="[
        'h-8 px-3 text-sm rounded-sm transition-all duration-[var(--d-2)] focus-visible:outline-2 focus-visible:outline-accent-1 focus-visible:outline-offset-2',
        modelValue === o.value ? 'bg-bg-1 text-fg-1' : 'text-fg-2 hover:text-fg-1'
      ]"
      @click="emit('update:modelValue', o.value)"
    >
      {{ o.label }}
    </button>
  </div>
</template>

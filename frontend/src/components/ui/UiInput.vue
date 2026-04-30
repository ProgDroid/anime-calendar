<script setup lang="ts">
import { computed, useId } from 'vue'
import { tv } from 'tailwind-variants'

interface Props {
  modelValue: string
  type?: 'text' | 'email' | 'password'
  placeholder?: string
  label?: string
  error?: string
  disabled?: boolean
  autocomplete?: string
}
const props = withDefaults(defineProps<Props>(), {
  type: 'text',
  placeholder: '',
  label: '',
  error: '',
  disabled: false,
  autocomplete: '',
})

defineOptions({ name: 'UiInput' })

const emit = defineEmits<{ (e: 'update:modelValue', v: string): void }>()
const id = useId()
const helperId = `${id}-helper`

const input = tv({
  base: 'w-full h-10 px-3 rounded-md bg-bg-1 text-fg-1 border outline-none transition-shadow duration-[var(--d-1)] focus:shadow-[0_0_0_3px_var(--accent-1-soft)]',
  variants: {
    state: {
      default: 'border-line focus:border-accent-1',
      error: 'border-danger focus:border-danger',
    },
  },
})

const classes = computed(() => input({ state: props.error ? 'error' : 'default' }))

function onInput(ev: Event) {
  emit('update:modelValue', (ev.target as HTMLInputElement).value)
}
</script>

<template>
  <div class="flex flex-col gap-1">
    <label v-if="label" :for="id" class="text-sm text-fg-2">{{ label }}</label>
    <input
      :id="id"
      :class="classes"
      :type="type"
      :value="modelValue"
      :placeholder="placeholder"
      :disabled="disabled"
      :autocomplete="autocomplete"
      :aria-invalid="!!error"
      :aria-describedby="error ? helperId : undefined"
      @input="onInput"
    >
    <span v-if="error" :id="helperId" class="text-xs text-danger">{{ error }}</span>
  </div>
</template>

<script setup lang="ts">
import { computed, useId, useSlots } from 'vue'
import { tv } from 'tailwind-variants'

interface Props {
  modelValue: string
  type?: 'text' | 'email' | 'password'
  placeholder?: string
  label?: string
  error?: string
  disabled?: boolean
  autocomplete?: string
  id?: string
  name?: string
  required?: boolean
  maxlength?: number
}
const props = withDefaults(defineProps<Props>(), {
  type: 'text',
  placeholder: '',
  label: '',
  error: '',
  disabled: false,
  autocomplete: '',
  id: undefined,
  name: undefined,
  required: false,
  maxlength: undefined,
})

defineOptions({ name: 'UiInput' })

const emit = defineEmits<{ (e: 'update:modelValue', v: string): void }>()
const slots = useSlots()
const generatedId = useId()
const inputId = computed(() => props.id ?? generatedId)
const helperId = computed(() => `${inputId.value}-helper`)

const input = tv({
  base: 'w-full h-10 px-3 rounded-md bg-bg-1 text-fg-1 border outline-none transition-shadow duration-[var(--d-1)] focus:shadow-[0_0_0_3px_var(--accent-1-soft)]',
  variants: {
    state: {
      default: 'border-line focus:border-accent-1',
      error: 'border-danger focus:border-danger',
    },
  },
})

const inputClasses = computed(() => [
  input({ state: props.error ? 'error' : 'default' }),
  slots.iconLeft ? 'pl-9' : '',
  slots.iconRight ? 'pr-9' : '',
])

function onInput(ev: Event) {
  emit('update:modelValue', (ev.target as HTMLInputElement).value)
}
</script>

<template>
  <div class="flex flex-col gap-1">
    <label v-if="label" :for="inputId" class="text-sm text-fg-2">{{ label }}</label>
    <div class="relative">
      <span
        v-if="$slots.iconLeft"
        class="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-fg-3 [&_svg]:w-3.5 [&_svg]:h-3.5"
      >
        <slot name="iconLeft" />
      </span>
      <input
        :id="inputId"
        :name="name"
        :class="inputClasses"
        :type="type"
        :value="modelValue"
        :placeholder="placeholder"
        :disabled="disabled"
        :required="required"
        :maxlength="maxlength"
        :autocomplete="autocomplete"
        :aria-invalid="!!error"
        :aria-describedby="error ? helperId : undefined"
        @input="onInput"
      >
      <span
        v-if="$slots.iconRight"
        class="absolute right-3 top-1/2 -translate-y-1/2 text-fg-3 [&_svg]:w-3.5 [&_svg]:h-3.5"
      >
        <slot name="iconRight" />
      </span>
    </div>
    <span v-if="error" :id="helperId" class="text-xs text-danger">{{ error }}</span>
  </div>
</template>

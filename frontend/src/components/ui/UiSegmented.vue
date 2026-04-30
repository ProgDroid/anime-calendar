<script setup lang="ts">
interface Option { value: string; label: string }
interface Props { modelValue: string; options: Option[] }
defineProps<Props>()
defineOptions({ name: 'UiSegmented' })
const emit = defineEmits<{ (e: 'update:modelValue', v: string): void }>()
</script>

<template>
  <div role="tablist" class="inline-flex rounded-md bg-bg-2 p-1 gap-1">
    <button
      v-for="o in options"
      :key="o.value"
      role="tab"
      type="button"
      :aria-selected="modelValue === o.value"
      :class="[
        'h-8 px-3 text-sm rounded-sm transition-all duration-[var(--d-2)]',
        modelValue === o.value ? 'bg-bg-1 text-fg-1' : 'text-fg-2 hover:text-fg-1'
      ]"
      @click="emit('update:modelValue', o.value)"
    >
      {{ o.label }}
    </button>
  </div>
</template>

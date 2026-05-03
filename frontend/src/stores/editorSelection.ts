import { defineStore } from 'pinia'
import { ref } from 'vue'

/**
 * Lifted batch-add selection state for the calendar editor.
 *
 * Storing selection in Pinia (rather than a local component ref) means the
 * selected set survives a viewport flip across the 1024 px breakpoint — the
 * mobile and desktop editor variants are swapped by the shell but share this
 * store instance.
 */
export const useEditorSelectionStore = defineStore('editorSelection', () => {
  const selectedMediaIds = ref<Set<number>>(new Set())

  function toggle(id: number) {
    // Sets are not deeply reactive — reassign to trigger watchers
    const next = new Set(selectedMediaIds.value)
    if (next.has(id)) next.delete(id)
    else next.add(id)
    selectedMediaIds.value = next
  }

  function clear() {
    selectedMediaIds.value = new Set()
  }

  function has(id: number): boolean {
    return selectedMediaIds.value.has(id)
  }

  return { selectedMediaIds, toggle, clear, has }
})

import { defineStore } from 'pinia'
import { ref } from 'vue'

/**
 * Lifted batch-add selection state for the mobile calendar editor.
 *
 * Storing the selected set in Pinia (rather than a local component ref) keeps
 * it alive across a viewport flip: the mobile editor variant is mounted and
 * unmounted by the shell as the 1024 px breakpoint is crossed, and a fresh
 * mount re-reads the surviving store instance instead of losing the selection.
 * Only the mobile variant consumes this store — the desktop editor manages its
 * own selection locally and does not import it (F2-28).
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

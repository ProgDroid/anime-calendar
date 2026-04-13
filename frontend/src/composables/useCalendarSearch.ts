import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import api from '@/config/api'
import type { Item } from '@/types/item'

export function useCalendarSearch() {
  const { t } = useI18n()

  const fetchedItems = ref<Item[]>([])
  const selectedItems = ref<number[]>([])
  const loading = ref(false)
  const searchError = ref<string | null>(null)

  const handleSearch = async ({ name, mediaType }: { name: string; mediaType: '' | 'ANIME' | 'MANGA' }) => {
    if (!name) {
      searchError.value = t('calendar.enterName')
      return
    }
    loading.value = true
    searchError.value = null
    try {
      let url = `/search?name=${encodeURIComponent(name)}`
      if (mediaType) url += `&media_type=${mediaType}`
      const response = await api.get(url)
      fetchedItems.value = response.data
      selectedItems.value = []
    } catch {
      searchError.value = t('calendar.fetchItemsFailed')
    } finally {
      loading.value = false
    }
  }

  const toggleItemSelection = (id: number) => {
    const idx = selectedItems.value.indexOf(id)
    if (idx === -1) selectedItems.value.push(id)
    else selectedItems.value.splice(idx, 1)
  }

  return { fetchedItems, selectedItems, loading, searchError, handleSearch, toggleItemSelection }
}

import { ref } from 'vue'
import type { Item } from '@/types/item'

export function useRecommendations() {
  const recommendations = ref<Item[]>([])

  const calculateRecommendations = (itemsInCalendar: Item[]) => {
    if (itemsInCalendar.length === 0) {
      recommendations.value = []
      return
    }
    const counts = new Map<number, { count: number; totalRating: number; item: Item }>()
    itemsInCalendar.forEach(item => {
      item.recommendations?.forEach(rec => {
        const id = rec.media.id
        if (itemsInCalendar.some(c => c.id === id)) return
        if (counts.has(id)) {
          const e = counts.get(id)!
          e.count++
          e.totalRating += rec.rating
        } else {
          counts.set(id, {
            count: 1,
            totalRating: rec.rating,
            item: {
              id: rec.media.id,
              id_mal: rec.media.id_mal,
              title: rec.media.title,
              media_type: item.media_type,
              episode_duration: 0,
              airing_schedule: [],
              cover_image: rec.media.cover_image,
              banner_image: '',
              recommendations: []
            }
          })
        }
      })
    })
    recommendations.value = Array.from(counts.values())
      .sort((a, b) => b.count !== a.count ? b.count - a.count : (b.totalRating / b.count) - (a.totalRating / a.count))
      .slice(0, 5)
      .map(e => e.item)
  }

  return { recommendations, calculateRecommendations }
}

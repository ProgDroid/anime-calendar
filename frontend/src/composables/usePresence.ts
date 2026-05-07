import { ref, onMounted, onBeforeUnmount } from 'vue'
import axios from 'axios'
import type { Viewer, CalendarEvent } from '@/types/sharing'

export function usePresence(calendarId: number) {
  const viewers = ref<Viewer[]>([])
  const metaVersion = ref(0)
  const lastEvent = ref<CalendarEvent | null>(null)
  let es: EventSource | null = null
  let heartbeatTimer: ReturnType<typeof setInterval> | null = null

  const onFrame = (data: string) => {
    let frame: CalendarEvent
    try {
      frame = JSON.parse(data) as CalendarEvent
    } catch {
      return
    }
    lastEvent.value = frame
    switch (frame.type) {
      case 'meta_snapshot':
        metaVersion.value = frame.v
        break
      case 'presence':
        viewers.value = frame.viewers
        break
      case 'item_added':
      case 'item_removed':
      case 'meta_updated':
        metaVersion.value = frame.v
        break
      case 'kick':
        // host component watches lastEvent and handles redirect
        break
    }
  }

  onMounted(() => {
    es = new EventSource(`/api/calendars/${calendarId}/events`, { withCredentials: true })
    es.onmessage = (e: MessageEvent) => onFrame(e.data as string)
    heartbeatTimer = setInterval(() => {
      axios.post(`/api/calendars/${calendarId}/presence/heartbeat`).catch(() => {})
    }, 30_000)
  })

  onBeforeUnmount(() => {
    es?.close()
    if (heartbeatTimer != null) {
      clearInterval(heartbeatTimer)
    }
  })

  return { viewers, metaVersion, lastEvent }
}

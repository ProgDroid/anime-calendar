import { ref, onMounted, onBeforeUnmount } from 'vue'
import axios from 'axios'
import api from '@/config/api'
import type { Viewer, CalendarEvent } from '@/types/sharing'

const RECONNECT_BASE_DELAY_MS = 1_000
const RECONNECT_MAX_DELAY_MS = 30_000

export function usePresence(calendarId: number) {
  const viewers = ref<Viewer[]>([])
  const metaVersion = ref(0)
  const lastEvent = ref<CalendarEvent | null>(null)
  let es: EventSource | null = null
  let heartbeatTimer: ReturnType<typeof setInterval> | null = null
  let reconnectTimer: ReturnType<typeof setTimeout> | null = null
  let reconnectDelay = RECONNECT_BASE_DELAY_MS
  let disposed = false

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

  const connect = () => {
    es = new EventSource(`/api/calendars/${calendarId}/events`, { withCredentials: true })
    es.onopen = () => {
      reconnectDelay = RECONNECT_BASE_DELAY_MS
    }
    es.onmessage = (e: MessageEvent) => onFrame(e.data as string)
    es.onerror = () => {
      // The browser auto-retries dropped connections on its own (readyState
      // stays CONNECTING); only step in when the source gave up entirely
      // (CLOSED) — which is what a non-200 response produces (401 after
      // cookie expiry, 429 from the per-user connection cap), with no
      // native retry. Without this handler, live sync — including
      // downgrade-kick frames — died silently for the rest of the session.
      if (disposed || es?.readyState !== EventSource.CLOSED) return
      es.close()
      es = null
      scheduleReconnect()
    }
  }

  const attemptReconnect = async () => {
    if (disposed) return
    try {
      // EventSource cannot drive the axios 401-refresh interceptor, so ping
      // an authed endpoint first: an expired access token gets silently
      // refreshed here, and a genuinely dead session 401s (the interceptor
      // redirects to /login) instead of reconnect-looping forever.
      await api.get('/user')
    } catch (err) {
      if (axios.isAxiosError(err) && err.response?.status === 401) {
        return // session is gone — the interceptor owns what happens next
      }
      scheduleReconnect() // transient failure: keep backing off
      return
    }
    if (disposed) return
    connect()
  }

  const scheduleReconnect = () => {
    if (disposed || reconnectTimer != null) return
    const delay = reconnectDelay
    reconnectDelay = Math.min(reconnectDelay * 2, RECONNECT_MAX_DELAY_MS)
    reconnectTimer = setTimeout(() => {
      reconnectTimer = null
      void attemptReconnect()
    }, delay)
  }

  onMounted(() => {
    connect()
    heartbeatTimer = setInterval(() => {
      // Via the `api` instance (not bare axios) so an expired access token
      // triggers the silent-refresh interceptor instead of 401ing forever
      // and dropping the user from the viewers list while they're active.
      api.post(`/calendars/${calendarId}/presence/heartbeat`).catch(() => {})
    }, 30_000)
  })

  onBeforeUnmount(() => {
    disposed = true
    es?.close()
    if (heartbeatTimer != null) {
      clearInterval(heartbeatTimer)
    }
    if (reconnectTimer != null) {
      clearTimeout(reconnectTimer)
    }
  })

  return { viewers, metaVersion, lastEvent }
}

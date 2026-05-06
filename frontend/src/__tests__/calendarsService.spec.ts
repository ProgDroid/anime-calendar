import { describe, it, expect, beforeEach, vi } from 'vitest'

vi.mock('@/config/api', () => ({
  default: {
    get: vi.fn(),
    post: vi.fn(),
    delete: vi.fn(),
  },
}))

import api from '@/config/api'
import { addItem, removeItem } from '@/services/calendars'

describe('calendars service — addItem / removeItem', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  describe('addItem', () => {
    it('calls POST /calendars/:id/items with the correct body', async () => {
      vi.mocked(api.post).mockResolvedValue({ data: { affected: true } })
      const result = await addItem(42, 7)
      expect(api.post).toHaveBeenCalledWith('/calendars/42/items', { item_id: 7 })
      expect(result).toEqual({ affected: true })
    })

    it('returns affected: false when the backend reports no change', async () => {
      vi.mocked(api.post).mockResolvedValue({ data: { affected: false } })
      const result = await addItem(1, 99)
      expect(result).toEqual({ affected: false })
    })

    it('propagates errors thrown by the API', async () => {
      vi.mocked(api.post).mockRejectedValue(new Error('network error'))
      await expect(addItem(1, 2)).rejects.toThrow('network error')
    })
  })

  describe('removeItem', () => {
    it('calls DELETE /calendars/:id/items/:itemId', async () => {
      vi.mocked(api.delete).mockResolvedValue({})
      await removeItem(42, 5)
      expect(api.delete).toHaveBeenCalledWith('/calendars/42/items/5')
    })

    it('resolves to undefined on success', async () => {
      vi.mocked(api.delete).mockResolvedValue({})
      const result = await removeItem(1, 2)
      expect(result).toBeUndefined()
    })

    it('propagates errors thrown by the API', async () => {
      vi.mocked(api.delete).mockRejectedValue(new Error('forbidden'))
      await expect(removeItem(1, 2)).rejects.toThrow('forbidden')
    })
  })
})

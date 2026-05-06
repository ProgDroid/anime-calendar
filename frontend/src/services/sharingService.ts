import api from '@/config/api'
import type { MembersResponse, SharingInvitation } from '@/types/sharing'

export const sharingService = {
  async listMembers(calendarId: number): Promise<MembersResponse> {
    const { data } = await api.get(`/calendars/${calendarId}/editors`)
    return data
  },
  async invite(calendarId: number, email: string): Promise<SharingInvitation> {
    const { data } = await api.post(`/calendars/${calendarId}/invitations`, { email })
    return data
  },
  async revoke(calendarId: number, invitationId: number): Promise<void> {
    await api.delete(`/calendars/${calendarId}/invitations/${invitationId}`)
  },
  async removeEditor(calendarId: number, userId: number): Promise<void> {
    await api.delete(`/calendars/${calendarId}/editors/${userId}`)
  },
  async resend(calendarId: number, invitationId: number): Promise<SharingInvitation> {
    const { data } = await api.post(`/calendars/${calendarId}/invitations/${invitationId}/resend`)
    return data
  },
}

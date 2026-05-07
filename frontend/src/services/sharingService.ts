import api from '@/config/api'
import type { MembersResponse, SharingInvitation, InvitationPreview } from '@/types/sharing'

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
  async preview(token: string): Promise<InvitationPreview> {
    const { data } = await api.get(`/invitations/${token}`)
    return data
  },
  async accept(token: string): Promise<{ calendar_id: number }> {
    const { data } = await api.post(`/invitations/${token}/accept`)
    return data
  },
  async decline(token: string): Promise<void> {
    await api.post(`/invitations/${token}/decline`)
  },
}

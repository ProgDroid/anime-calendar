import { ref } from 'vue'
import { defineStore } from 'pinia'
import { sharingService } from '@/services/sharingService'
import type { MemberSummary, PendingInviteSummary, SharingInvitation } from '@/types/sharing'

export const useSharingStore = defineStore('sharing', () => {
  const editors = ref<MemberSummary[]>([])
  const pendingInvites = ref<PendingInviteSummary[]>([])
  const editorCap = ref(5)
  // Monotonic request id so a slower earlier load can't overwrite a newer
  // one's results (e.g. switching calendars quickly) (F2-28).
  let membersSeq = 0

  async function loadMembers(calendarId: number) {
    const seq = ++membersSeq
    const { editors: e, pending, editor_cap } = await sharingService.listMembers(calendarId)
    if (seq !== membersSeq) return
    editors.value = e
    pendingInvites.value = pending
    editorCap.value = editor_cap
  }

  async function invite(calendarId: number, email: string): Promise<SharingInvitation> {
    const inv = await sharingService.invite(calendarId, email)
    pendingInvites.value.unshift({
      id: inv.id,
      invitee_email: inv.invitee_email,
      sent_at: inv.sent_at,
      expires_at: inv.expires_at,
    })
    return inv
  }

  async function revoke(calendarId: number, invitationId: number) {
    await sharingService.revoke(calendarId, invitationId)
    pendingInvites.value = pendingInvites.value.filter(i => i.id !== invitationId)
  }

  async function removeEditor(calendarId: number, userId: number) {
    await sharingService.removeEditor(calendarId, userId)
    editors.value = editors.value.filter(e => e.user_id !== userId)
  }

  async function resend(calendarId: number, invitationId: number) {
    const fresh = await sharingService.resend(calendarId, invitationId)
    pendingInvites.value = pendingInvites.value.map(i =>
      i.id === invitationId
        ? { id: fresh.id, invitee_email: fresh.invitee_email, sent_at: fresh.sent_at, expires_at: fresh.expires_at }
        : i,
    )
  }

  return { editors, pendingInvites, editorCap, loadMembers, invite, revoke, removeEditor, resend }
})

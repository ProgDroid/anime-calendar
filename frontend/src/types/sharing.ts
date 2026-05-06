export interface MemberSummary {
  user_id: number
  display: string
  email: string
  joined_at: string
  suspended_at: string | null
}

export interface PendingInviteSummary {
  id: number
  invitee_email: string
  sent_at: string
  expires_at: string
}

export interface MembersResponse {
  editors: MemberSummary[]
  pending: PendingInviteSummary[]
  editor_cap: number
}

export interface SharingInvitation {
  id: number
  invitee_email: string
  sent_at: string
  expires_at: string
}

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

export interface Viewer {
  user_id: number
  display: string
}

export type CalendarEvent =
  | { type: 'meta_snapshot'; v: number }
  | { type: 'item_added'; media_id: number; actor: string; v: number; at: string }
  | { type: 'item_removed'; media_id: number; actor: string; v: number; at: string }
  | { type: 'meta_updated'; fields: string[]; actor: string; v: number; at: string }
  | { type: 'member_joined'; user_id: string; display: string; actor: string }
  | { type: 'member_left'; user_id: string; actor: string; reason: string }
  | { type: 'presence'; viewers: Viewer[] }
  | { type: 'kick'; reason: string }

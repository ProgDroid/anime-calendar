<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { useSharingStore } from '@/stores/sharingStore'
import { useUpgradeInterrupt } from '@/composables/useUpgradeInterrupt'
import UiAvatar from '@/components/ui/UiAvatar.vue'
import UiButton from '@/components/ui/UiButton.vue'
import InviteEditorModal from './InviteEditorModal.vue'

defineOptions({ name: 'MembersTab' })

const props = defineProps<{
  calendarId: number
  isOwner: boolean
  isPaid: boolean
}>()

const { t } = useI18n()
const sharingStore = useSharingStore()
const { openUpgradeModal } = useUpgradeInterrupt()

const loadError = ref<string | null>(null)
const inviteModalOpen = ref(false)

function fmtDate(isoStr: string): string {
  return new Date(isoStr + 'Z').toLocaleDateString()
}

function getInitials(display: string): string {
  return display
    .split(' ')
    .slice(0, 2)
    .map(w => w[0] ?? '')
    .join('')
    .toUpperCase()
}

async function handleRemoveEditor(userId: number) {
  try {
    await sharingStore.removeEditor(props.calendarId, userId)
  } catch {
    // silently ignore — backend error will not crash the tab
  }
}

async function handleRevoke(invitationId: number) {
  try {
    await sharingStore.revoke(props.calendarId, invitationId)
  } catch {
    // silently ignore
  }
}

async function handleResend(invitationId: number) {
  try {
    await sharingStore.resend(props.calendarId, invitationId)
  } catch {
    // silently ignore
  }
}

onMounted(async () => {
  if (!props.isPaid) return
  try {
    await sharingStore.loadMembers(props.calendarId)
  } catch {
    loadError.value = t('errors.generic')
  }
})
</script>

<template>
  <!-- Free tier: upgrade prompt -->
  <section
    v-if="!isPaid"
    data-testid="members-upgrade-prompt"
    class="flex flex-col items-center gap-4 py-8 text-center"
  >
    <p class="text-fg-2">{{ t('sharing.upgradePrompt') }}</p>
    <UiButton
      variant="primary"
      data-testid="members-upgrade-cta"
      @click="openUpgradeModal('share_calendar')"
    >
      {{ t('sharing.upgradeCta') }}
    </UiButton>
  </section>

  <!-- Paid owner: full members UI -->
  <div v-else data-testid="members-tab" class="flex flex-col gap-6">
    <!-- Error state -->
    <div
      v-if="loadError"
      data-testid="members-error"
      class="text-sm text-danger-text bg-danger/10 border border-danger/30 rounded-md px-3 py-2"
    >
      {{ loadError }}
    </div>

    <!-- Active editors -->
    <section class="flex flex-col gap-2">
      <h3 class="text-sm font-medium text-fg-2">{{ t('sharing.activeEditors') }}</h3>
      <ul data-testid="editors-list" class="flex flex-col gap-2">
        <li
          v-for="editor in sharingStore.editors"
          :key="editor.user_id"
          data-testid="editor-row"
          class="flex items-center gap-3 py-2 px-3 bg-bg-2 rounded-md"
        >
          <UiAvatar
            :alt="editor.display"
            :fallback="getInitials(editor.display)"
            size="sm"
          />
          <span class="flex-1 text-fg-1 text-sm truncate">{{ editor.display }}</span>
          <UiButton
            variant="ghost"
            size="sm"
            data-testid="remove-editor-btn"
            @click="handleRemoveEditor(editor.user_id)"
          >
            {{ t('sharing.remove') }}
          </UiButton>
        </li>
      </ul>
    </section>

    <!-- Pending invites -->
    <section class="flex flex-col gap-2">
      <h3 class="text-sm font-medium text-fg-2">{{ t('sharing.pendingInvites') }}</h3>
      <ul data-testid="pending-list" class="flex flex-col gap-2">
        <li
          v-for="invite in sharingStore.pendingInvites"
          :key="invite.id"
          data-testid="pending-row"
          class="flex items-center gap-3 py-2 px-3 bg-bg-2 rounded-md flex-wrap"
        >
          <span class="flex-1 text-fg-1 text-sm truncate">{{ invite.invitee_email }}</span>
          <span class="text-xs text-fg-3">{{ fmtDate(invite.sent_at) }}</span>
          <span class="text-xs text-fg-3">{{ fmtDate(invite.expires_at) }}</span>
          <div class="flex gap-1">
            <UiButton
              variant="ghost"
              size="sm"
              data-testid="resend-btn"
              @click="handleResend(invite.id)"
            >
              {{ t('sharing.resend') }}
            </UiButton>
            <UiButton
              variant="ghost"
              size="sm"
              data-testid="revoke-btn"
              @click="handleRevoke(invite.id)"
            >
              {{ t('sharing.revoke') }}
            </UiButton>
          </div>
        </li>
      </ul>
    </section>

    <!-- Invite button -->
    <div>
      <UiButton
        variant="secondary"
        data-testid="invite-btn"
        @click="inviteModalOpen = true"
      >
        {{ t('sharing.inviteButton') }}
      </UiButton>
    </div>

    <!-- Invite editor modal -->
    <InviteEditorModal
      v-model:open="inviteModalOpen"
      :calendar-id="calendarId"
    />
  </div>
</template>

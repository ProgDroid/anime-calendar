<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import axios from 'axios'
import UiModal from '@/components/ui/UiModal.vue'
import UiInput from '@/components/ui/UiInput.vue'
import UiButton from '@/components/ui/UiButton.vue'
import { useSharingStore } from '@/stores/sharingStore'

defineOptions({ name: 'InviteEditorModal' })

const props = defineProps<{
  open: boolean
  calendarId: number
}>()

const emit = defineEmits<{
  'update:open': [value: boolean]
}>()

const { t } = useI18n()
const sharingStore = useSharingStore()

const email = ref('')
const inlineError = ref<string | null>(null)
const submitting = ref(false)

const EMAIL_RE = /^[^@\s]+@[^@\s]+\.[^@\s]+$/

function close() {
  emit('update:open', false)
  email.value = ''
  inlineError.value = null
}

async function submit() {
  inlineError.value = null
  if (!EMAIL_RE.test(email.value)) {
    inlineError.value = t('sharing.errors.invalid_email')
    return
  }
  submitting.value = true
  try {
    await sharingStore.invite(props.calendarId, email.value)
    close()
  } catch (err) {
    if (axios.isAxiosError(err) && err.response?.status === 409) {
      const code = (err.response.data as { error?: string })?.error
      if (
        code === 'editor_cap_reached' ||
        code === 'invite_already_pending' ||
        code === 'self_invite'
      ) {
        inlineError.value = t(`sharing.errors.${code}`)
      } else {
        inlineError.value = t('errors.generic')
      }
    }
    // 402 bubbles to global axios interceptor
  } finally {
    submitting.value = false
  }
}
</script>

<template>
  <!-- eslint-disable-next-line vue/attribute-hyphenation -->
  <UiModal :open="open" :ariaLabel="t('sharing.inviteModal.title')" @update:open="(v) => emit('update:open', v)">
    <template #header>
      <h2 class="font-semibold text-fg-1 text-lg">{{ t('sharing.inviteModal.title') }}</h2>
    </template>

    <div class="flex flex-col gap-4">
      <UiInput
        v-model="email"
        data-testid="invite-email-input"
        type="email"
        :label="t('sharing.inviteModal.emailLabel')"
        :placeholder="t('sharing.inviteModal.emailLabel')"
        autocomplete="email"
        @keyup.enter="submit"
      />
      <div
        v-if="inlineError"
        role="alert"
        class="text-sm text-danger-text bg-danger/10 border border-danger/30 rounded-md px-3 py-2"
      >
        {{ inlineError }}
      </div>
    </div>

    <template #footer>
      <div class="flex justify-end gap-2">
        <UiButton
          variant="secondary"
          data-testid="invite-cancel-btn"
          :disabled="submitting"
          @click="close"
        >
          {{ t('sharing.inviteModal.cancel') }}
        </UiButton>
        <UiButton
          variant="primary"
          data-testid="invite-submit-btn"
          :disabled="submitting"
          @click="submit"
        >
          {{ submitting ? t('sharing.inviteModal.sending') : t('sharing.inviteModal.submit') }}
        </UiButton>
      </div>
    </template>
  </UiModal>
</template>

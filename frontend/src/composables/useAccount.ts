import { ref } from 'vue'
import { useAuthStore } from '@/stores/auth'
import client from '@/api/client'
import type { components } from '@/api/schema'

type ProfileSchemaField = components['schemas']['ProfileSchemaField']
type NotificationPreferences = components['schemas']['NotificationPreferences']
type ChannelPreference = components['schemas']['ChannelPreference']

function defaultChannelPreference(): ChannelPreference {
  return {
    enabled: true,
    categories: {
      taskUpdates: true,
      todoAssignments: true,
      aiSuggestions: true,
      mentions: true,
      systemAnnouncements: true,
    },
  }
}

export function useAccount() {
  const authStore = useAuthStore()
  const profileData = ref<Record<string, unknown>>({})
  const profileSchema = ref<ProfileSchemaField[]>([])
  const notificationPreferences = ref<NotificationPreferences>({
    channels: {
      email: defaultChannelPreference(),
      webPush: defaultChannelPreference(),
      inApp: defaultChannelPreference(),
    },
  })
  const passkeys = ref<{ id: string; name: string; createdAt: string; lastUsedAt: string | null }[]>([])
  const loading = ref(false)
  const error = ref('')

  async function fetchProfile() {
    loading.value = true
    error.value = ''
    try {
      const { data } = await client.GET('/accounts/me/profile')
      if (data) {
        profileData.value = data.profileData ?? {}
        profileSchema.value = data.profileSchema ?? []
      }
    } catch {
      error.value = 'Failed to load profile.'
    } finally {
      loading.value = false
    }
  }

  async function updateProfile(updates: Record<string, unknown>) {
    loading.value = true
    error.value = ''
    try {
      const { data } = await client.PUT('/accounts/me/profile', {
        body: { profileData: updates },
      })
      if (data) {
        profileData.value = data.profileData ?? {}
        profileSchema.value = data.profileSchema ?? []
      }
    } catch {
      error.value = 'Failed to update profile.'
    } finally {
      loading.value = false
    }
  }

  async function updateDisplayName(displayName: string) {
    loading.value = true
    error.value = ''
    try {
      const { data } = await client.PATCH('/accounts/me', {
        body: { name: displayName },
      })
      if (data) {
        await authStore.fetchCurrentUser()
      }
    } catch {
      error.value = 'Failed to update display name.'
    } finally {
      loading.value = false
    }
  }

  async function fetchNotificationPreferences() {
    try {
      const { data } = await client.GET('/accounts/me/notification-preferences')
      if (data) {
        notificationPreferences.value = data
      }
    } catch {
      error.value = 'Failed to load notification preferences.'
    }
  }

  async function updateNotificationPreferences(prefs: NotificationPreferences) {
    loading.value = true
    error.value = ''
    try {
      const { data } = await client.PUT('/accounts/me/notification-preferences', {
        body: prefs,
      })
      if (data) {
        notificationPreferences.value = data
      }
    } catch {
      error.value = 'Failed to update notification preferences.'
    } finally {
      loading.value = false
    }
  }

  async function fetchPasskeys() {
    try {
      const { data } = await client.GET('/auth/passkeys')
      if (data) {
        passkeys.value = data.passkeys
      }
    } catch {
      error.value = 'Failed to load passkeys.'
    }
  }

  async function deletePasskey(id: string) {
    loading.value = true
    error.value = ''
    try {
      await client.DELETE('/auth/passkeys/{passkeyId}', {
        params: { path: { passkeyId: id } },
      })
      passkeys.value = passkeys.value.filter((p) => p.id !== id)
    } catch {
      error.value = 'Failed to delete passkey.'
    } finally {
      loading.value = false
    }
  }

  async function registerPasskey(_name: string): Promise<boolean> {
    loading.value = true
    error.value = ''
    try {
      const { startRegistration } = await import('@simplewebauthn/browser')

      const { data: beginData } = await client.POST('/auth/passkey/register/begin')
      if (!beginData) {
        error.value = 'Failed to start passkey registration.'
        return false
      }

      const credential = await startRegistration({
        optionsJSON: beginData as Parameters<typeof startRegistration>[0]['optionsJSON'],
      })

      const { data: completeData } = await client.POST('/auth/passkey/register/complete', {
        body: credential,
      })

      if (completeData !== undefined) {
        await fetchPasskeys()
        return true
      }
      error.value = 'Failed to complete passkey registration.'
      return false
    } catch {
      error.value = 'Passkey registration failed or was cancelled.'
      return false
    } finally {
      loading.value = false
    }
  }

  return {
    profileData,
    profileSchema,
    notificationPreferences,
    passkeys,
    loading,
    error,
    fetchProfile,
    updateProfile,
    updateDisplayName,
    fetchNotificationPreferences,
    updateNotificationPreferences,
    fetchPasskeys,
    deletePasskey,
    registerPasskey,
  }
}

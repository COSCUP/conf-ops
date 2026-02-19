import { ref } from 'vue'
import { useAuthStore } from '@/stores/auth'
import client from '@/api/client'

interface Profile {
  bio?: string
  [key: string]: unknown
}

interface NotificationPreferences {
  email_notifications: boolean
  push_notifications: boolean
}

interface PasskeyItem {
  id: string
  name: string
  created_at: string
  last_used_at: string | null
}

export function useAccount() {
  const authStore = useAuthStore()
  const profile = ref<Profile>({})
  const notificationPreferences = ref<NotificationPreferences>({
    email_notifications: true,
    push_notifications: false,
  })
  const passkeys = ref<PasskeyItem[]>([])
  const loading = ref(false)
  const error = ref('')

  async function fetchProfile() {
    loading.value = true
    error.value = ''
    try {
      const { data } = await client.GET('/api/v1/accounts/me/profile' as never)
      if (data) {
        const profileData = data as { profile: Profile }
        profile.value = profileData.profile ?? {}
      }
    } catch {
      error.value = 'Failed to load profile.'
    } finally {
      loading.value = false
    }
  }

  async function updateProfile(updates: Profile) {
    loading.value = true
    error.value = ''
    try {
      const { data } = await client.PUT('/api/v1/accounts/me/profile' as never, {
        body: { profile: updates },
      } as never)
      if (data) {
        const profileData = data as { profile: Profile }
        profile.value = profileData.profile ?? {}
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
      const { data } = await client.PATCH('/api/v1/accounts/me' as never, {
        body: { display_name: displayName },
      } as never)
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
      const { data } = await client.GET(
        '/api/v1/accounts/me/notification-preferences' as never,
      )
      if (data) {
        notificationPreferences.value = data as NotificationPreferences
      }
    } catch {
      error.value = 'Failed to load notification preferences.'
    }
  }

  async function updateNotificationPreferences(prefs: NotificationPreferences) {
    loading.value = true
    error.value = ''
    try {
      const { data } = await client.PUT(
        '/api/v1/accounts/me/notification-preferences' as never,
        { body: prefs } as never,
      )
      if (data) {
        notificationPreferences.value = data as NotificationPreferences
      }
    } catch {
      error.value = 'Failed to update notification preferences.'
    } finally {
      loading.value = false
    }
  }

  async function fetchPasskeys() {
    try {
      const { data } = await client.GET('/api/v1/accounts/me/passkeys' as never)
      if (data) {
        passkeys.value = data as PasskeyItem[]
      }
    } catch {
      error.value = 'Failed to load passkeys.'
    }
  }

  async function deletePasskey(id: string) {
    loading.value = true
    error.value = ''
    try {
      await client.DELETE(`/api/v1/accounts/me/passkeys/${id}` as never)
      passkeys.value = passkeys.value.filter((p) => p.id !== id)
    } catch {
      error.value = 'Failed to delete passkey.'
    } finally {
      loading.value = false
    }
  }

  return {
    profile,
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
  }
}

import { describe, it, expect, beforeEach, vi } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { useAccount } from '../useAccount'

vi.mock('@/api/client', () => ({
  default: {
    GET: vi.fn(),
    POST: vi.fn(),
    PUT: vi.fn(),
    PATCH: vi.fn(),
    DELETE: vi.fn(),
    use: vi.fn(),
  },
  setupAuthInterceptor: vi.fn(),
}))

import client from '@/api/client'

describe('useAccount', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('fetchProfile loads profile data', async () => {
    vi.mocked(client.GET).mockResolvedValue({
      data: {
        profileData: { bio: 'Hello world' },
        profileSchema: [{ key: 'bio', label: 'Bio', description: '', type: 'text' }],
      },
    } as never)

    const { profileData, profileSchema, fetchProfile } = useAccount()
    await fetchProfile()

    expect(profileData.value).toEqual({ bio: 'Hello world' })
    expect(profileSchema.value).toHaveLength(1)
  })

  it('updateProfile sends update and refreshes', async () => {
    vi.mocked(client.PUT).mockResolvedValue({
      data: {
        profileData: { bio: 'Updated' },
        profileSchema: [],
      },
    } as never)

    const { profileData, updateProfile } = useAccount()
    await updateProfile({ bio: 'Updated' })

    expect(client.PUT).toHaveBeenCalled()
    expect(profileData.value).toEqual({ bio: 'Updated' })
  })

  it('fetchPasskeys loads passkey list', async () => {
    const mockPasskeys = [
      { id: '1', name: 'My Key', createdAt: '2024-01-01', lastUsedAt: null },
    ]
    vi.mocked(client.GET).mockResolvedValue({
      data: { passkeys: mockPasskeys },
    } as never)

    const { passkeys, fetchPasskeys } = useAccount()
    await fetchPasskeys()

    expect(passkeys.value).toEqual(mockPasskeys)
  })

  it('deletePasskey removes from list', async () => {
    vi.mocked(client.DELETE).mockResolvedValue({ data: null } as never)

    const { passkeys, deletePasskey } = useAccount()
    passkeys.value = [
      { id: '1', name: 'Key A', createdAt: '2024-01-01', lastUsedAt: null },
      { id: '2', name: 'Key B', createdAt: '2024-01-02', lastUsedAt: null },
    ]

    await deletePasskey('1')

    expect(passkeys.value).toHaveLength(1)
    expect(passkeys.value[0]?.id).toBe('2')
  })

  it('fetchNotificationPreferences loads preferences', async () => {
    vi.mocked(client.GET).mockResolvedValue({
      data: {
        channels: {
          email: {
            enabled: false,
            categories: {
              taskUpdates: true,
              todoAssignments: true,
              aiSuggestions: true,
              mentions: true,
              systemAnnouncements: true,
            },
          },
          webPush: {
            enabled: true,
            categories: {
              taskUpdates: true,
              todoAssignments: true,
              aiSuggestions: true,
              mentions: true,
              systemAnnouncements: true,
            },
          },
          inApp: {
            enabled: true,
            categories: {
              taskUpdates: true,
              todoAssignments: true,
              aiSuggestions: true,
              mentions: true,
              systemAnnouncements: true,
            },
          },
        },
      },
    } as never)

    const { notificationPreferences, fetchNotificationPreferences } = useAccount()
    await fetchNotificationPreferences()

    expect(notificationPreferences.value.channels.email?.enabled).toBe(false)
    expect(notificationPreferences.value.channels.webPush?.enabled).toBe(true)
  })

  it('sets error on fetch failure', async () => {
    vi.mocked(client.GET).mockRejectedValue(new Error('fail'))

    const { error, fetchProfile } = useAccount()
    await fetchProfile()

    expect(error.value).toBe('Failed to load profile.')
  })
})

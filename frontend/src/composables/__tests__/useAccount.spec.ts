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
      data: { profile: { bio: 'Hello world' } },
    } as never)

    const { profile, fetchProfile } = useAccount()
    await fetchProfile()

    expect(profile.value).toEqual({ bio: 'Hello world' })
  })

  it('updateProfile sends update and refreshes', async () => {
    vi.mocked(client.PUT).mockResolvedValue({
      data: { profile: { bio: 'Updated' } },
    } as never)

    const { profile, updateProfile } = useAccount()
    await updateProfile({ bio: 'Updated' })

    expect(client.PUT).toHaveBeenCalled()
    expect(profile.value).toEqual({ bio: 'Updated' })
  })

  it('fetchPasskeys loads passkey list', async () => {
    const mockPasskeys = [
      { id: '1', name: 'My Key', created_at: '2024-01-01', last_used_at: null },
    ]
    vi.mocked(client.GET).mockResolvedValue({ data: mockPasskeys } as never)

    const { passkeys, fetchPasskeys } = useAccount()
    await fetchPasskeys()

    expect(passkeys.value).toEqual(mockPasskeys)
  })

  it('deletePasskey removes from list', async () => {
    vi.mocked(client.DELETE).mockResolvedValue({ data: null } as never)

    const { passkeys, deletePasskey } = useAccount()
    passkeys.value = [
      { id: '1', name: 'Key A', created_at: '2024-01-01', last_used_at: null },
      { id: '2', name: 'Key B', created_at: '2024-01-02', last_used_at: null },
    ]

    await deletePasskey('1')

    expect(passkeys.value).toHaveLength(1)
    expect(passkeys.value[0]?.id).toBe('2')
  })

  it('fetchNotificationPreferences loads preferences', async () => {
    vi.mocked(client.GET).mockResolvedValue({
      data: { email_notifications: false, push_notifications: true },
    } as never)

    const { notificationPreferences, fetchNotificationPreferences } = useAccount()
    await fetchNotificationPreferences()

    expect(notificationPreferences.value.email_notifications).toBe(false)
    expect(notificationPreferences.value.push_notifications).toBe(true)
  })

  it('sets error on fetch failure', async () => {
    vi.mocked(client.GET).mockRejectedValue(new Error('fail'))

    const { error, fetchProfile } = useAccount()
    await fetchProfile()

    expect(error.value).toBe('Failed to load profile.')
  })
})

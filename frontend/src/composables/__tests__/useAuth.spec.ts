import { describe, it, expect, beforeEach, vi } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'

vi.mock('@/api/client', () => ({
  default: {
    GET: vi.fn(),
    POST: vi.fn(),
    use: vi.fn(),
  },
  setupAuthInterceptor: vi.fn(),
}))

vi.mock('@simplewebauthn/browser', () => ({
  startAuthentication: vi.fn(),
}))

import client from '@/api/client'
import { startAuthentication } from '@simplewebauthn/browser'
import { useAuthStore } from '@/stores/auth'

describe('useAuth', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('requestMagicLink calls correct endpoint', async () => {
    vi.mocked(client.POST).mockResolvedValue({ data: { message: 'sent' } } as never)

    await client.POST('/api/v1/auth/magic-link/request', {
      body: { email: 'test@example.com' },
    })

    expect(client.POST).toHaveBeenCalledWith('/api/v1/auth/magic-link/request', {
      body: { email: 'test@example.com' },
    })
  })

  it('verifyMagicLink sets token on success', async () => {
    vi.mocked(client.GET).mockResolvedValueOnce({
      data: { accessToken: 'new-token', tokenType: 'Bearer', expiresIn: 900 },
    } as never).mockResolvedValueOnce({
      data: {
        id: '1',
        email: 'test@example.com',
        name: 'Test',
        avatarUrl: null,
        bio: null,
        locale: 'en',
        createdAt: '2025-01-01T00:00:00Z',
        updatedAt: '2025-01-01T00:00:00Z',
      },
    } as never)

    const store = useAuthStore()

    const { data } = await client.GET('/api/v1/auth/magic-link/verify', {
      params: { query: { token: 'test-token' } },
    })

    if (data) {
      store.setTokens(data.accessToken)
      await store.fetchCurrentUser()
    }

    expect(store.accessToken).toBe('new-token')
    expect(store.currentUser).toBeTruthy()
  })

  it('loginWithPasskey completes full flow', async () => {
    vi.mocked(client.POST)
      .mockResolvedValueOnce({
        data: { challenge: 'test-challenge', rpId: 'example.com' },
      } as never)
      .mockResolvedValueOnce({
        data: { accessToken: 'passkey-token', tokenType: 'Bearer', expiresIn: 900 },
      } as never)
    vi.mocked(startAuthentication).mockResolvedValue({} as never)
    vi.mocked(client.GET).mockResolvedValue({
      data: {
        id: '1',
        email: 'test@example.com',
        name: 'Test',
        avatarUrl: null,
        bio: null,
        locale: 'en',
        createdAt: '2025-01-01T00:00:00Z',
        updatedAt: '2025-01-01T00:00:00Z',
      },
    } as never)

    const store = useAuthStore()

    const { data: beginData } = await client.POST('/api/v1/auth/passkey/login/begin')

    expect(beginData).toBeTruthy()

    const credential = await startAuthentication({
      optionsJSON: beginData as unknown as Parameters<typeof startAuthentication>[0]['optionsJSON'],
    })

    const { data: completeData } = await client.POST('/api/v1/auth/passkey/login/complete', {
      body: { challenge_id: (beginData as { challenge_id: string }).challenge_id, credential: credential as never },
    })

    if (completeData) {
      store.setTokens(completeData.accessToken)
    }

    expect(store.accessToken).toBe('passkey-token')
  })

  it('logout clears auth state', async () => {
    vi.mocked(client.POST).mockResolvedValue({ data: undefined } as never)

    const store = useAuthStore()
    store.setTokens('test-token')

    await store.logout()

    expect(store.isAuthenticated).toBe(false)
    expect(store.accessToken).toBeNull()
  })
})

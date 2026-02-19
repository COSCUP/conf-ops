import { describe, it, expect, beforeEach, vi } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { useAuthStore } from '../auth'

vi.mock('@/api/client', () => ({
  default: {
    GET: vi.fn(),
    POST: vi.fn(),
  },
}))

import client from '@/api/client'

describe('useAuthStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('starts unauthenticated', () => {
    const store = useAuthStore()
    expect(store.isAuthenticated).toBe(false)
    expect(store.accessToken).toBeNull()
    expect(store.currentUser).toBeNull()
  })

  it('setTokens sets access token and marks authenticated', () => {
    const store = useAuthStore()
    store.setTokens('test-token')
    expect(store.accessToken).toBe('test-token')
    expect(store.isAuthenticated).toBe(true)
  })

  it('clearAuth resets all state', () => {
    const store = useAuthStore()
    store.setTokens('test-token')
    store.clearAuth()
    expect(store.accessToken).toBeNull()
    expect(store.currentUser).toBeNull()
    expect(store.isAuthenticated).toBe(false)
  })

  it('fetchCurrentUser calls API and sets user', async () => {
    const mockUser = {
      id: '123',
      email: 'test@example.com',
      display_name: 'Test',
      avatar_url: null,
      locale: 'en',
    }
    vi.mocked(client.GET).mockResolvedValue({ data: mockUser } as never)

    const store = useAuthStore()
    store.setTokens('test-token')
    await store.fetchCurrentUser()

    expect(store.currentUser).toEqual(mockUser)
  })

  it('fetchCurrentUser does nothing without token', async () => {
    const store = useAuthStore()
    await store.fetchCurrentUser()
    expect(client.GET).not.toHaveBeenCalled()
  })

  it('refreshToken updates token on success', async () => {
    vi.mocked(client.POST).mockResolvedValue({
      data: { access_token: 'new-token' },
    } as never)
    vi.mocked(client.GET).mockResolvedValue({
      data: { id: '1', email: 'a@b.c', display_name: 'A', avatar_url: null, locale: 'en' },
    } as never)

    const store = useAuthStore()
    const result = await store.refreshToken()

    expect(result).toBe(true)
    expect(store.accessToken).toBe('new-token')
  })

  it('refreshToken clears auth on failure', async () => {
    vi.mocked(client.POST).mockRejectedValue(new Error('fail'))

    const store = useAuthStore()
    store.setTokens('old-token')
    const result = await store.refreshToken()

    expect(result).toBe(false)
    expect(store.isAuthenticated).toBe(false)
  })

  it('logout calls API and clears auth', async () => {
    vi.mocked(client.POST).mockResolvedValue({ data: null } as never)

    const store = useAuthStore()
    store.setTokens('test-token')
    await store.logout()

    expect(client.POST).toHaveBeenCalled()
    expect(store.isAuthenticated).toBe(false)
  })
})

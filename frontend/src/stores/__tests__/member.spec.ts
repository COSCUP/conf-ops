import { describe, it, expect, beforeEach, vi } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { useMemberStore } from '../member'

vi.mock('@/api/client', () => ({
  default: {
    GET: vi.fn(),
    POST: vi.fn(),
    PUT: vi.fn(),
    DELETE: vi.fn(),
  },
}))

import client from '@/api/client'

describe('useMemberStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('starts with empty state', () => {
    const store = useMemberStore()
    expect(store.members).toEqual([])
    expect(store.currentMember).toBeNull()
    expect(store.loading).toBe(false)
    expect(store.error).toBe('')
  })

  it('fetchMembers loads members', async () => {
    const mockMembers = [
      { id: '1', projectId: 'p1', accountId: 'a1', role: 'owner', name: 'Alice', email: 'alice@test.com', avatarUrl: null, createdAt: '', updatedAt: '' },
    ]
    vi.mocked(client.GET).mockResolvedValue({ data: { members: mockMembers } } as never)

    const store = useMemberStore()
    await store.fetchMembers('p1')

    expect(store.members).toEqual(mockMembers)
  })

  it('inviteMember calls POST and refreshes list', async () => {
    const mockMember = { id: '1', projectId: 'p1', accountId: 'a1', role: 'member', createdAt: '', updatedAt: '' }
    vi.mocked(client.POST).mockResolvedValue({ data: mockMember } as never)
    vi.mocked(client.GET).mockResolvedValue({ data: { members: [] } } as never)

    const store = useMemberStore()
    const result = await store.inviteMember('p1', 'a1', 'member')

    expect(result).toEqual(mockMember)
    expect(client.POST).toHaveBeenCalled()
    expect(client.GET).toHaveBeenCalled()
  })

  it('updateRole calls PUT', async () => {
    vi.mocked(client.PUT).mockResolvedValue({} as never)
    vi.mocked(client.GET).mockResolvedValue({ data: { members: [] } } as never)

    const store = useMemberStore()
    await store.updateRole('p1', 'm1', 'tag_admin')

    expect(client.PUT).toHaveBeenCalled()
  })

  it('removeMember calls DELETE', async () => {
    vi.mocked(client.DELETE).mockResolvedValue({} as never)
    vi.mocked(client.GET).mockResolvedValue({ data: { members: [] } } as never)

    const store = useMemberStore()
    await store.removeMember('p1', 'm1')

    expect(client.DELETE).toHaveBeenCalled()
  })

  it('fetchMembers sets error on failure', async () => {
    vi.mocked(client.GET).mockRejectedValue(new Error('fail'))

    const store = useMemberStore()
    await store.fetchMembers('p1')

    expect(store.error).toBe('Failed to load members.')
  })
})

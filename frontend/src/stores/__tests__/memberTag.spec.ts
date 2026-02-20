import { describe, it, expect, beforeEach, vi } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { useMemberTagStore } from '../memberTag'

vi.mock('@/api/client', () => ({
  default: {
    GET: vi.fn(),
    POST: vi.fn(),
    PUT: vi.fn(),
    DELETE: vi.fn(),
  },
}))

import client from '@/api/client'

describe('useMemberTagStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('starts with empty state', () => {
    const store = useMemberTagStore()
    expect(store.tags).toEqual([])
    expect(store.currentTagDetail).toBeNull()
    expect(store.loading).toBe(false)
    expect(store.error).toBe('')
  })

  it('fetchTags loads tags', async () => {
    const mockTags = [
      { id: '1', projectId: 'p1', name: 'dev', description: null, memberCount: 3, contactCount: 1, createdAt: '', updatedAt: '' },
    ]
    vi.mocked(client.GET).mockResolvedValue({ data: { tags: mockTags } } as never)

    const store = useMemberTagStore()
    await store.fetchTags('p1')

    expect(store.tags).toEqual(mockTags)
  })

  it('createTag calls POST and refreshes list', async () => {
    const mockTag = { id: '1', projectId: 'p1', name: 'dev', description: null, externalTaskCreation: {}, createdAt: '', updatedAt: '' }
    vi.mocked(client.POST).mockResolvedValue({ data: mockTag } as never)
    vi.mocked(client.GET).mockResolvedValue({ data: { tags: [] } } as never)

    const store = useMemberTagStore()
    const result = await store.createTag('p1', 'dev', 'Dev team')

    expect(result).toEqual(mockTag)
    expect(client.POST).toHaveBeenCalled()
  })

  it('getTagDetail loads tag detail', async () => {
    const mockDetail = {
      tag: { id: '1', projectId: 'p1', name: 'dev', description: null, externalTaskCreation: {}, createdAt: '', updatedAt: '' },
      members: [],
      contacts: [],
    }
    vi.mocked(client.GET).mockResolvedValue({ data: mockDetail } as never)

    const store = useMemberTagStore()
    await store.getTagDetail('p1', '1')

    expect(store.currentTagDetail).toEqual(mockDetail)
  })

  it('deleteTag calls DELETE', async () => {
    vi.mocked(client.DELETE).mockResolvedValue({} as never)
    vi.mocked(client.GET).mockResolvedValue({ data: { tags: [] } } as never)

    const store = useMemberTagStore()
    await store.deleteTag('p1', '1')

    expect(client.DELETE).toHaveBeenCalled()
  })

  it('assignTag calls POST assign endpoint', async () => {
    const mockAssignment = { id: 'a1', tagId: 't1', memberId: 'm1', contactId: null, projectId: 'p1', createdAt: '' }
    vi.mocked(client.POST).mockResolvedValue({ data: mockAssignment } as never)
    vi.mocked(client.GET).mockResolvedValue({ data: { tag: {}, members: [], contacts: [] } } as never)

    const store = useMemberTagStore()
    const result = await store.assignTag('p1', 't1', 'm1')

    expect(result).toEqual(mockAssignment)
    expect(client.POST).toHaveBeenCalled()
  })

  it('removeAssignment calls DELETE', async () => {
    vi.mocked(client.DELETE).mockResolvedValue({} as never)
    vi.mocked(client.GET).mockResolvedValue({ data: { tag: {}, members: [], contacts: [] } } as never)

    const store = useMemberTagStore()
    await store.removeAssignment('p1', 't1', 'a1')

    expect(client.DELETE).toHaveBeenCalled()
  })

  it('fetchTags sets error on failure', async () => {
    vi.mocked(client.GET).mockRejectedValue(new Error('fail'))

    const store = useMemberTagStore()
    await store.fetchTags('p1')

    expect(store.error).toBe('Failed to load tags.')
  })
})

import { describe, it, expect, beforeEach, vi } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { useUnassignedInboxStore } from '../unassignedInbox'

vi.mock('@/api/client', () => ({
  default: {
    GET: vi.fn(),
    POST: vi.fn(),
    PUT: vi.fn(),
    DELETE: vi.fn(),
  },
}))

import client from '@/api/client'

describe('useUnassignedInboxStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('starts with empty state', () => {
    const store = useUnassignedInboxStore()
    expect(store.emails).toEqual([])
    expect(store.loading).toBe(false)
    expect(store.error).toBe('')
    expect(store.pagination).toEqual({ hasMore: false, nextCursor: null })
  })

  it('fetchEmails loads emails', async () => {
    const mockEmails = [
      {
        id: 'e1', projectId: 'p1', subject: 'Hello', fromAddress: 'sender@test.com',
        fromName: 'Sender', snippet: 'Preview', hasAttachments: false, receivedAt: '2025-01-01T00:00:00Z',
      },
    ]
    vi.mocked(client.GET).mockResolvedValue({
      data: { emails: mockEmails, pagination: { hasMore: false, nextCursor: null } },
    } as never)

    const store = useUnassignedInboxStore()
    await store.fetchEmails('p1')

    expect(store.emails).toEqual(mockEmails)
    expect(store.pagination).toEqual({ hasMore: false, nextCursor: null })
    expect(client.GET).toHaveBeenCalledWith(
      '/api/v1/projects/{projectId}/unassigned-inbox',
      expect.objectContaining({
        params: expect.objectContaining({
          path: { projectId: 'p1' },
        }),
      }),
    )
  })

  it('fetchEmails appends emails when cursor provided', async () => {
    const firstBatch = [
      { id: 'e1', projectId: 'p1', subject: 'First', fromAddress: 'a@test.com', fromName: null, snippet: null, hasAttachments: false, receivedAt: '' },
    ]
    const secondBatch = [
      { id: 'e2', projectId: 'p1', subject: 'Second', fromAddress: 'b@test.com', fromName: null, snippet: null, hasAttachments: false, receivedAt: '' },
    ]

    vi.mocked(client.GET).mockResolvedValueOnce({
      data: { emails: firstBatch, pagination: { hasMore: true, nextCursor: 'cursor1' } },
    } as never)
    vi.mocked(client.GET).mockResolvedValueOnce({
      data: { emails: secondBatch, pagination: { hasMore: false, nextCursor: null } },
    } as never)

    const store = useUnassignedInboxStore()
    await store.fetchEmails('p1')
    await store.fetchEmails('p1', 'cursor1')

    expect(store.emails).toHaveLength(2)
    expect(store.emails[0]!.id).toBe('e1')
    expect(store.emails[1]!.id).toBe('e2')
  })

  it('fetchEmails sets error on failure', async () => {
    vi.mocked(client.GET).mockRejectedValue(new Error('fail'))

    const store = useUnassignedInboxStore()
    await store.fetchEmails('p1')

    expect(store.error).toBe('Failed to load unassigned emails.')
    expect(store.loading).toBe(false)
  })

  it('assignEmail removes the email from list and returns data', async () => {
    const mockEmails = [
      { id: 'e1', projectId: 'p1', subject: 'Assign Me', fromAddress: 'a@test.com', fromName: null, snippet: null, hasAttachments: false, receivedAt: '' },
      { id: 'e2', projectId: 'p1', subject: 'Keep Me', fromAddress: 'b@test.com', fromName: null, snippet: null, hasAttachments: false, receivedAt: '' },
    ]
    vi.mocked(client.GET).mockResolvedValue({
      data: { emails: mockEmails, pagination: { hasMore: false, nextCursor: null } },
    } as never)
    const mockResult = { id: 'e1', taskId: 't1' }
    vi.mocked(client.POST).mockResolvedValue({ data: mockResult } as never)

    const store = useUnassignedInboxStore()
    await store.fetchEmails('p1')
    const result = await store.assignEmail('p1', 'e1', 't1')

    expect(result).toEqual(mockResult)
    expect(store.emails).toHaveLength(1)
    expect(store.emails[0]!.id).toBe('e2')
    expect(client.POST).toHaveBeenCalledWith(
      '/api/v1/projects/{projectId}/unassigned-inbox/{emailId}/assign',
      expect.objectContaining({
        params: expect.objectContaining({ path: { projectId: 'p1', emailId: 'e1' } }),
        body: { taskId: 't1' },
      }),
    )
  })

  it('assignEmail returns null and sets error on failure', async () => {
    vi.mocked(client.POST).mockRejectedValue(new Error('fail'))

    const store = useUnassignedInboxStore()
    const result = await store.assignEmail('p1', 'e1', 't1')

    expect(result).toBeNull()
    expect(store.error).toBe('Failed to assign email.')
  })

  it('$reset restores initial state', async () => {
    const mockEmails = [
      { id: 'e1', projectId: 'p1', subject: 'Test', fromAddress: 'a@test.com', fromName: null, snippet: null, hasAttachments: false, receivedAt: '' },
    ]
    vi.mocked(client.GET).mockResolvedValue({
      data: { emails: mockEmails, pagination: { hasMore: true, nextCursor: 'c1' } },
    } as never)

    const store = useUnassignedInboxStore()
    await store.fetchEmails('p1')
    expect(store.emails).toHaveLength(1)

    store.$reset()

    expect(store.emails).toEqual([])
    expect(store.loading).toBe(false)
    expect(store.error).toBe('')
    expect(store.pagination).toEqual({ hasMore: false, nextCursor: null })
  })
})

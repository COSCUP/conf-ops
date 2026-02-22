import { describe, it, expect, beforeEach, vi } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { useEmailThreadStore } from '../emailThread'

vi.mock('@/api/client', () => ({
  default: {
    GET: vi.fn(),
    POST: vi.fn(),
    PUT: vi.fn(),
    DELETE: vi.fn(),
    PATCH: vi.fn(),
  },
}))

import client from '@/api/client'

describe('useEmailThreadStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('starts with empty state', () => {
    const store = useEmailThreadStore()
    expect(store.threads).toEqual([])
    expect(store.currentMessages).toEqual([])
    expect(store.loading).toBe(false)
    expect(store.error).toBe('')
    expect(store.pagination).toEqual({ hasMore: false, nextCursor: null })
  })

  it('fetchThreads loads threads', async () => {
    const mockThreads = [
      { id: 'th1', taskId: 't1', subject: 'Hello', participants: [], lastMessageAt: null, createdAt: '', updatedAt: '' },
    ]
    vi.mocked(client.GET).mockResolvedValue({
      data: { threads: mockThreads, pagination: { hasMore: false, nextCursor: null } },
    } as never)

    const store = useEmailThreadStore()
    await store.fetchThreads('p1', 't1')

    expect(store.threads).toEqual(mockThreads)
    expect(store.pagination).toEqual({ hasMore: false, nextCursor: null })
    expect(client.GET).toHaveBeenCalledWith(
      '/api/v1/projects/{projectId}/tasks/{taskId}/email-threads',
      expect.objectContaining({
        params: expect.objectContaining({
          path: { projectId: 'p1', taskId: 't1' },
        }),
      }),
    )
  })

  it('fetchThreads appends threads when cursor provided', async () => {
    const firstBatch = [
      { id: 'th1', taskId: 't1', subject: 'First', participants: [], lastMessageAt: null, createdAt: '', updatedAt: '' },
    ]
    const secondBatch = [
      { id: 'th2', taskId: 't1', subject: 'Second', participants: [], lastMessageAt: null, createdAt: '', updatedAt: '' },
    ]

    vi.mocked(client.GET).mockResolvedValueOnce({
      data: { threads: firstBatch, pagination: { hasMore: true, nextCursor: 'cursor1' } },
    } as never)
    vi.mocked(client.GET).mockResolvedValueOnce({
      data: { threads: secondBatch, pagination: { hasMore: false, nextCursor: null } },
    } as never)

    const store = useEmailThreadStore()
    await store.fetchThreads('p1', 't1')
    await store.fetchThreads('p1', 't1', 'cursor1')

    expect(store.threads).toHaveLength(2)
    expect(store.threads[0]!.id).toBe('th1')
    expect(store.threads[1]!.id).toBe('th2')
  })

  it('fetchThreads sets error on failure', async () => {
    vi.mocked(client.GET).mockRejectedValue(new Error('fail'))

    const store = useEmailThreadStore()
    await store.fetchThreads('p1', 't1')

    expect(store.error).toBe('Failed to load email threads.')
    expect(store.loading).toBe(false)
  })

  it('createThread calls POST and prepends to threads list', async () => {
    const mockThread = {
      id: 'th1', taskId: 't1', subject: 'New Thread', participants: ['a@b.com'], lastMessageAt: null, createdAt: '', updatedAt: '',
    }
    vi.mocked(client.POST).mockResolvedValue({ data: mockThread } as never)

    const store = useEmailThreadStore()
    const result = await store.createThread('p1', 't1', 'New Thread', ['a@b.com'])

    expect(result).toEqual(mockThread)
    expect(store.threads).toEqual([mockThread])
    expect(client.POST).toHaveBeenCalledWith(
      '/api/v1/projects/{projectId}/tasks/{taskId}/email-threads',
      expect.objectContaining({
        params: expect.objectContaining({ path: { projectId: 'p1', taskId: 't1' } }),
        body: { subject: 'New Thread', participants: ['a@b.com'] },
      }),
    )
  })

  it('createThread returns null and sets error on failure', async () => {
    vi.mocked(client.POST).mockRejectedValue(new Error('fail'))

    const store = useEmailThreadStore()
    const result = await store.createThread('p1', 't1', 'Subject', ['a@b.com'])

    expect(result).toBeNull()
    expect(store.error).toBe('Failed to create email thread.')
  })

  it('deleteThread removes thread from list', async () => {
    const mockThreads = [
      { id: 'th1', taskId: 't1', subject: 'Keep', participants: [], lastMessageAt: null, createdAt: '', updatedAt: '' },
      { id: 'th2', taskId: 't1', subject: 'Delete', participants: [], lastMessageAt: null, createdAt: '', updatedAt: '' },
    ]
    vi.mocked(client.GET).mockResolvedValue({
      data: { threads: mockThreads, pagination: { hasMore: false, nextCursor: null } },
    } as never)
    vi.mocked(client.DELETE).mockResolvedValue({} as never)

    const store = useEmailThreadStore()
    await store.fetchThreads('p1', 't1')
    await store.deleteThread('p1', 't1', 'th2')

    expect(store.threads).toHaveLength(1)
    expect(store.threads[0]!.id).toBe('th1')
    expect(client.DELETE).toHaveBeenCalledWith(
      '/api/v1/projects/{projectId}/tasks/{taskId}/email-threads/{threadId}',
      expect.objectContaining({
        params: expect.objectContaining({ path: { projectId: 'p1', taskId: 't1', threadId: 'th2' } }),
      }),
    )
  })

  it('deleteThread sets error on failure', async () => {
    vi.mocked(client.DELETE).mockRejectedValue(new Error('fail'))

    const store = useEmailThreadStore()
    await store.deleteThread('p1', 't1', 'th1')

    expect(store.error).toBe('Failed to delete email thread.')
  })

  it('fetchMessages loads messages into currentMessages', async () => {
    const mockMessages = [
      { id: 'm1', threadId: 'th1', fromAddress: 'a@b.com', subject: 'Hi', htmlBody: '<p>Hello</p>', sentAt: '', createdAt: '' },
    ]
    vi.mocked(client.GET).mockResolvedValue({ data: { messages: mockMessages } } as never)

    const store = useEmailThreadStore()
    await store.fetchMessages('p1', 't1', 'th1')

    expect(store.currentMessages).toEqual(mockMessages)
    expect(client.GET).toHaveBeenCalledWith(
      '/api/v1/projects/{projectId}/tasks/{taskId}/email-threads/{threadId}/messages',
      expect.objectContaining({
        params: expect.objectContaining({ path: { projectId: 'p1', taskId: 't1', threadId: 'th1' } }),
      }),
    )
  })

  it('fetchMessages sets error on failure', async () => {
    vi.mocked(client.GET).mockRejectedValue(new Error('fail'))

    const store = useEmailThreadStore()
    await store.fetchMessages('p1', 't1', 'th1')

    expect(store.error).toBe('Failed to load email messages.')
  })

  it('sendEmail appends message to currentMessages', async () => {
    const mockMessage = {
      id: 'm1', threadId: 'th1', fromAddress: 'me@example.com', subject: 'Hi', htmlBody: '<p>Body</p>', sentAt: '', createdAt: '',
    }
    vi.mocked(client.POST).mockResolvedValue({ data: mockMessage } as never)

    const store = useEmailThreadStore()
    const result = await store.sendEmail('p1', 't1', 'th1', {
      toAddresses: ['them@example.com'],
      htmlBody: '<p>Body</p>',
    })

    expect(result).toEqual(mockMessage)
    expect(store.currentMessages).toEqual([mockMessage])
    expect(client.POST).toHaveBeenCalledWith(
      '/api/v1/projects/{projectId}/tasks/{taskId}/email-threads/{threadId}/messages',
      expect.objectContaining({
        params: expect.objectContaining({ path: { projectId: 'p1', taskId: 't1', threadId: 'th1' } }),
        body: { toAddresses: ['them@example.com'], htmlBody: '<p>Body</p>' },
      }),
    )
  })

  it('sendEmail returns null and sets error on failure', async () => {
    vi.mocked(client.POST).mockRejectedValue(new Error('fail'))

    const store = useEmailThreadStore()
    const result = await store.sendEmail('p1', 't1', 'th1', {
      toAddresses: ['them@example.com'],
      htmlBody: '<p>Body</p>',
    })

    expect(result).toBeNull()
    expect(store.error).toBe('Failed to send email.')
  })

  it('$reset restores initial state', async () => {
    const mockThread = {
      id: 'th1', taskId: 't1', subject: 'Test', participants: [], lastMessageAt: null, createdAt: '', updatedAt: '',
    }
    vi.mocked(client.GET).mockResolvedValue({
      data: { threads: [mockThread], pagination: { hasMore: true, nextCursor: 'c1' } },
    } as never)

    const store = useEmailThreadStore()
    await store.fetchThreads('p1', 't1')
    expect(store.threads).toHaveLength(1)

    store.$reset()

    expect(store.threads).toEqual([])
    expect(store.currentMessages).toEqual([])
    expect(store.loading).toBe(false)
    expect(store.error).toBe('')
    expect(store.pagination).toEqual({ hasMore: false, nextCursor: null })
  })
})

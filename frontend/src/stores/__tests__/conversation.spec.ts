import { describe, it, expect, beforeEach, vi } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { useConversationStore } from '../conversation'

vi.mock('@/api/client', () => ({
  default: {
    GET: vi.fn(),
    POST: vi.fn(),
    PUT: vi.fn(),
    DELETE: vi.fn(),
  },
}))

import client from '@/api/client'

const mockMessage = {
  id: 'msg-1',
  taskId: 'task-1',
  sourceType: 'member',
  sourceId: 'member-1',
  content: { text: 'Hello', mentions: [] },
  attachments: null,
  actionResult: null,
  lastSeenMessageId: null,
  createdAt: '2025-01-01T00:00:00Z',
}

describe('useConversationStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('starts with empty state', () => {
    const store = useConversationStore()
    expect(store.messages).toEqual([])
    expect(store.loading).toBe(false)
    expect(store.error).toBe('')
    expect(store.hasMore).toBe(false)
    expect(store.nextCursor).toBeNull()
    expect(store.lastReadMessageId).toBeNull()
  })

  it('fetchMessages loads messages', async () => {
    vi.mocked(client.GET).mockResolvedValue({
      data: {
        messages: [mockMessage],
        pagination: { hasMore: false, nextCursor: null },
      },
    } as never)

    const store = useConversationStore()
    await store.fetchMessages('proj-1', 'task-1')

    expect(store.messages).toHaveLength(1)
    expect(store.messages[0]!.id).toBe('msg-1')
    expect(store.hasMore).toBe(false)
  })

  it('fetchMessages appends when cursor is provided', async () => {
    const store = useConversationStore()

    vi.mocked(client.GET).mockResolvedValue({
      data: {
        messages: [mockMessage],
        pagination: { hasMore: true, nextCursor: 'cursor-1' },
      },
    } as never)
    await store.fetchMessages('proj-1', 'task-1')

    const secondMessage = { ...mockMessage, id: 'msg-2' }
    vi.mocked(client.GET).mockResolvedValue({
      data: {
        messages: [secondMessage],
        pagination: { hasMore: false, nextCursor: null },
      },
    } as never)
    await store.fetchMessages('proj-1', 'task-1', 'cursor-1')

    expect(store.messages).toHaveLength(2)
    expect(store.hasMore).toBe(false)
  })

  it('fetchMessages sets error on failure', async () => {
    vi.mocked(client.GET).mockRejectedValue(new Error('fail'))
    const store = useConversationStore()
    await store.fetchMessages('proj-1', 'task-1')

    expect(store.error).toBe('Failed to load messages.')
    expect(store.messages).toEqual([])
  })

  it('sendMessage adds message to list', async () => {
    vi.mocked(client.POST).mockResolvedValue({ data: mockMessage } as never)

    const store = useConversationStore()
    const result = await store.sendMessage('proj-1', 'task-1', {
      text: 'Hello',
      mentions: [],
    })

    expect(result).toBeTruthy()
    expect(store.messages).toHaveLength(1)
    expect(store.messages[0]!.content).toEqual({ text: 'Hello', mentions: [] })
  })

  it('sendMessage sets error on failure', async () => {
    vi.mocked(client.POST).mockRejectedValue(new Error('fail'))

    const store = useConversationStore()
    const result = await store.sendMessage('proj-1', 'task-1', { text: 'Hello', mentions: [] })

    expect(result).toBeNull()
    expect(store.error).toBe('Failed to send message.')
  })

  it('updateLastSeen updates state', async () => {
    vi.mocked(client.PUT).mockResolvedValue({ data: undefined } as never)

    const store = useConversationStore()
    await store.updateLastSeen('proj-1', 'task-1', 'msg-1')

    expect(store.lastReadMessageId).toBe('msg-1')
  })

  it('getLastSeen loads last read message', async () => {
    vi.mocked(client.GET).mockResolvedValue({
      data: { lastReadMessageId: 'msg-5' },
    } as never)

    const store = useConversationStore()
    await store.getLastSeen('proj-1', 'task-1')

    expect(store.lastReadMessageId).toBe('msg-5')
  })

  it('$reset clears all state', async () => {
    vi.mocked(client.GET).mockResolvedValue({
      data: {
        messages: [mockMessage],
        pagination: { hasMore: true, nextCursor: 'c-1' },
      },
    } as never)

    const store = useConversationStore()
    await store.fetchMessages('proj-1', 'task-1')
    store.$reset()

    expect(store.messages).toEqual([])
    expect(store.hasMore).toBe(false)
    expect(store.nextCursor).toBeNull()
  })
})

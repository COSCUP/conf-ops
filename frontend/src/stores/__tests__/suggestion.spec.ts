import { describe, it, expect, beforeEach, vi } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { useSuggestionStore } from '../suggestion'

vi.mock('@/api/client', () => ({
  default: {
    GET: vi.fn(),
    POST: vi.fn(),
    PUT: vi.fn(),
    DELETE: vi.fn(),
  },
}))

import client from '@/api/client'

const mockSuggestion = {
  id: 'sugg-1',
  tool: 'send_email',
  summary: 'Send a welcome email',
  reasoning: 'The task was just created and requires an introductory email.',
  parameters: { to: '{{contact.email}}', subject: 'Welcome' },
  decision: 'pending' as const,
  decidedAt: null,
  decidedBy: null,
  modifiedParameters: null,
  executionResult: null,
  contextUsed: [],
}

const mockSuggestionGroup = {
  id: 'group-1',
  createdAt: '2025-01-01T00:00:00Z',
  trigger: 'task_created' as const,
  suggestions: [mockSuggestion],
}

const mockGroupItem = {
  messageId: 'msg-1',
  suggestionGroup: mockSuggestionGroup,
}

describe('useSuggestionStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('starts with empty state', () => {
    const store = useSuggestionStore()
    expect(store.suggestionGroups).toEqual([])
    expect(store.loading).toBe(false)
    expect(store.error).toBeNull()
  })

  it('listSuggestions loads suggestion groups', async () => {
    vi.mocked(client.GET).mockResolvedValue({
      data: { data: [mockGroupItem] },
    } as never)

    const store = useSuggestionStore()
    await store.listSuggestions('proj-1', 'task-1')

    expect(store.suggestionGroups).toHaveLength(1)
    expect(store.suggestionGroups[0]!.messageId).toBe('msg-1')
    expect(store.suggestionGroups[0]!.suggestionGroup.id).toBe('group-1')
  })

  it('listSuggestions appends when cursor is provided', async () => {
    vi.mocked(client.GET).mockResolvedValue({
      data: { data: [mockGroupItem] },
    } as never)
    const store = useSuggestionStore()
    await store.listSuggestions('proj-1', 'task-1')

    const secondItem = { ...mockGroupItem, messageId: 'msg-2' }
    vi.mocked(client.GET).mockResolvedValue({
      data: { data: [secondItem] },
    } as never)
    await store.listSuggestions('proj-1', 'task-1', 'cursor-1')

    expect(store.suggestionGroups).toHaveLength(2)
  })

  it('listSuggestions sets error on failure', async () => {
    vi.mocked(client.GET).mockRejectedValue(new Error('network error'))
    const store = useSuggestionStore()
    await store.listSuggestions('proj-1', 'task-1')

    expect(store.error).toBe('Failed to load suggestions.')
    expect(store.suggestionGroups).toEqual([])
  })

  it('getSuggestionGroup returns a group item', async () => {
    vi.mocked(client.GET).mockResolvedValue({
      data: {
        messageId: 'msg-1',
        suggestionGroup: mockSuggestionGroup,
      },
    } as never)

    const store = useSuggestionStore()
    const result = await store.getSuggestionGroup('proj-1', 'task-1', 'group-1')

    expect(result).not.toBeNull()
    expect(result?.messageId).toBe('msg-1')
    expect(result?.suggestionGroup.id).toBe('group-1')
  })

  it('getSuggestionGroup returns null on failure', async () => {
    vi.mocked(client.GET).mockRejectedValue(new Error('not found'))
    const store = useSuggestionStore()
    const result = await store.getSuggestionGroup('proj-1', 'task-1', 'group-1')

    expect(result).toBeNull()
    expect(store.error).toBe('Failed to load suggestion group.')
  })

  it('decideSuggestion posts decision and updates local state', async () => {
    vi.mocked(client.GET).mockResolvedValue({
      data: { data: [mockGroupItem] },
    } as never)
    vi.mocked(client.POST).mockResolvedValue({ data: { status: 'ok' } } as never)

    const store = useSuggestionStore()
    await store.listSuggestions('proj-1', 'task-1')

    const success = await store.decideSuggestion('proj-1', 'task-1', 'group-1', 'sugg-1', {
      decision: 'accept',
      lastSeenMessageId: 'msg-1',
    })

    expect(success).toBe(true)
    const group = store.suggestionGroups[0]
    expect(group?.suggestionGroup.suggestions[0]?.decision).toBe('accept')
  })

  it('decideSuggestion sets error on failure', async () => {
    vi.mocked(client.POST).mockRejectedValue(new Error('conflict'))
    const store = useSuggestionStore()
    const success = await store.decideSuggestion('proj-1', 'task-1', 'group-1', 'sugg-1', {
      decision: 'reject',
      lastSeenMessageId: 'msg-1',
    })

    expect(success).toBe(false)
    expect(store.error).toBe('Failed to record decision.')
  })

  it('requestSuggestion posts and returns true', async () => {
    vi.mocked(client.POST).mockResolvedValue({
      data: { eventId: 'event-1' },
    } as never)

    const store = useSuggestionStore()
    const result = await store.requestSuggestion('proj-1', 'task-1')

    expect(result).toBe(true)
    expect(vi.mocked(client.POST)).toHaveBeenCalledOnce()
  })

  it('requestSuggestion returns false on failure', async () => {
    vi.mocked(client.POST).mockRejectedValue(new Error('error'))
    const store = useSuggestionStore()
    const result = await store.requestSuggestion('proj-1', 'task-1')

    expect(result).toBe(false)
    expect(store.error).toBe('Failed to request suggestion.')
  })

  it('resolvePlaceholders returns resolved text', async () => {
    vi.mocked(client.POST).mockResolvedValue({
      data: { text: 'Hello John', unresolved: [] },
    } as never)

    const store = useSuggestionStore()
    const result = await store.resolvePlaceholders('proj-1', 'task-1', 'Hello {{name}}')

    expect(result).not.toBeNull()
    expect(result?.text).toBe('Hello John')
    expect(result?.unresolved).toEqual([])
  })

  it('$reset clears all state', async () => {
    vi.mocked(client.GET).mockResolvedValue({
      data: { data: [mockGroupItem] },
    } as never)

    const store = useSuggestionStore()
    await store.listSuggestions('proj-1', 'task-1')
    store.$reset()

    expect(store.suggestionGroups).toEqual([])
    expect(store.loading).toBe(false)
    expect(store.error).toBeNull()
  })
})

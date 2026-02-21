import { describe, it, expect, beforeEach, vi } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { useMyTodosStore } from '../myTodos'

vi.mock('@/api/client', () => ({
  default: {
    GET: vi.fn(),
    POST: vi.fn(),
    PUT: vi.fn(),
    DELETE: vi.fn(),
  },
}))

import client from '@/api/client'

describe('useMyTodosStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('starts with empty state', () => {
    const store = useMyTodosStore()
    expect(store.items).toEqual([])
    expect(store.loading).toBe(false)
    expect(store.error).toBe('')
  })

  it('fetchMyTodos loads items', async () => {
    const mockItems = [
      { id: 'mt1', title: 'My Todo 1', status: 'open', projectId: 'p1', taskId: 't1', createdAt: '', updatedAt: '' },
    ]
    vi.mocked(client.GET).mockResolvedValue({ data: { items: mockItems } } as never)

    const store = useMyTodosStore()
    await store.fetchMyTodos()

    expect(store.items).toEqual(mockItems)
    expect(client.GET).toHaveBeenCalled()
  })

  it('fetchMyTodos with status filter', async () => {
    vi.mocked(client.GET).mockResolvedValue({ data: { items: [] } } as never)

    const store = useMyTodosStore()
    await store.fetchMyTodos('completed' as never)

    expect(client.GET).toHaveBeenCalledWith(
      '/api/v1/accounts/me/todos',
      expect.objectContaining({
        params: expect.objectContaining({
          query: { status: 'completed' },
        }),
      }),
    )
  })

  it('fetchMyTodos with projectId filter', async () => {
    vi.mocked(client.GET).mockResolvedValue({ data: { items: [] } } as never)

    const store = useMyTodosStore()
    await store.fetchMyTodos(undefined, 'p1')

    expect(client.GET).toHaveBeenCalledWith(
      '/api/v1/accounts/me/todos',
      expect.objectContaining({
        params: expect.objectContaining({
          query: { projectId: 'p1' },
        }),
      }),
    )
  })

  it('fetchMyTodos sets error on failure', async () => {
    vi.mocked(client.GET).mockRejectedValue(new Error('fail'))

    const store = useMyTodosStore()
    await store.fetchMyTodos()

    expect(store.error).toBe('Failed to load my todos.')
  })
})

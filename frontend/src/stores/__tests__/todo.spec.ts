import { describe, it, expect, beforeEach, vi } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { useTodoStore } from '../todo'

vi.mock('@/api/client', () => ({
  default: {
    GET: vi.fn(),
    POST: vi.fn(),
    PUT: vi.fn(),
    DELETE: vi.fn(),
  },
}))

import client from '@/api/client'

describe('useTodoStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('starts with empty state', () => {
    const store = useTodoStore()
    expect(store.todos).toEqual([])
    expect(store.currentTodo).toBeNull()
    expect(store.loading).toBe(false)
    expect(store.error).toBe('')
  })

  it('fetchTodos loads todos', async () => {
    const mockTodos = [
      { id: 'todo1', title: 'Todo 1', status: 'open', createdAt: '', updatedAt: '' },
    ]
    vi.mocked(client.GET).mockResolvedValue({ data: { todos: mockTodos } } as never)

    const store = useTodoStore()
    await store.fetchTodos('p1', 't1')

    expect(store.todos).toEqual(mockTodos)
    expect(client.GET).toHaveBeenCalled()
  })

  it('fetchTodos sets error on failure', async () => {
    vi.mocked(client.GET).mockRejectedValue(new Error('fail'))

    const store = useTodoStore()
    await store.fetchTodos('p1', 't1')

    expect(store.error).toBe('Failed to load todos.')
  })

  it('createTodo calls POST and refetches', async () => {
    const mockTodo = { id: 'todo1', title: 'New Todo', status: 'open', createdAt: '', updatedAt: '' }
    vi.mocked(client.POST).mockResolvedValue({ data: mockTodo } as never)
    vi.mocked(client.GET).mockResolvedValue({ data: { todos: [] } } as never)

    const store = useTodoStore()
    const result = await store.createTodo('p1', 't1', 'New Todo', 'desc', 'parent1')

    expect(result).toEqual(mockTodo)
    expect(client.POST).toHaveBeenCalled()
    expect(client.GET).toHaveBeenCalled()
  })

  it('createTodo sets error on failure', async () => {
    vi.mocked(client.POST).mockRejectedValue(new Error('fail'))

    const store = useTodoStore()
    const result = await store.createTodo('p1', 't1', 'New Todo')

    expect(result).toBeNull()
    expect(store.error).toBe('Failed to create todo.')
  })

  it('updateTodo calls PUT and refetches', async () => {
    vi.mocked(client.PUT).mockResolvedValue({} as never)
    vi.mocked(client.GET).mockResolvedValue({ data: { todos: [] } } as never)

    const store = useTodoStore()
    await store.updateTodo('p1', 't1', 'todo1', { title: 'Updated' })

    expect(client.PUT).toHaveBeenCalled()
    expect(client.GET).toHaveBeenCalled()
  })

  it('updateTodo sets error on failure', async () => {
    vi.mocked(client.PUT).mockRejectedValue(new Error('fail'))

    const store = useTodoStore()
    await store.updateTodo('p1', 't1', 'todo1', { title: 'Updated' })

    expect(store.error).toBe('Failed to update todo.')
  })

  it('updateTodoStatus calls PUT and refetches', async () => {
    vi.mocked(client.PUT).mockResolvedValue({} as never)
    vi.mocked(client.GET).mockResolvedValue({ data: { todos: [] } } as never)

    const store = useTodoStore()
    await store.updateTodoStatus('p1', 't1', 'todo1', 'completed' as never)

    expect(client.PUT).toHaveBeenCalled()
    expect(client.GET).toHaveBeenCalled()
  })

  it('updateTodoStatus sets error on failure', async () => {
    vi.mocked(client.PUT).mockRejectedValue(new Error('fail'))

    const store = useTodoStore()
    await store.updateTodoStatus('p1', 't1', 'todo1', 'completed' as never)

    expect(store.error).toBe('Failed to update todo status.')
  })

  it('deleteTodo calls DELETE and refetches', async () => {
    vi.mocked(client.DELETE).mockResolvedValue({} as never)
    vi.mocked(client.GET).mockResolvedValue({ data: { todos: [] } } as never)

    const store = useTodoStore()
    await store.deleteTodo('p1', 't1', 'todo1')

    expect(client.DELETE).toHaveBeenCalled()
    expect(client.GET).toHaveBeenCalled()
  })

  it('deleteTodo sets error on failure', async () => {
    vi.mocked(client.DELETE).mockRejectedValue(new Error('fail'))

    const store = useTodoStore()
    await store.deleteTodo('p1', 't1', 'todo1')

    expect(store.error).toBe('Failed to delete todo.')
  })

  it('addAssignee calls POST and returns data without refetching', async () => {
    const mockAssignee = { id: 'a1', memberId: 'm1', todoId: 'todo1', createdAt: '' }
    vi.mocked(client.POST).mockResolvedValue({ data: mockAssignee } as never)

    const store = useTodoStore()
    const result = await store.addAssignee('p1', 't1', 'todo1', 'm1')

    expect(result).toEqual(mockAssignee)
    expect(client.POST).toHaveBeenCalled()
    expect(client.GET).not.toHaveBeenCalled()
  })

  it('addAssignee sets error on failure', async () => {
    vi.mocked(client.POST).mockRejectedValue(new Error('fail'))

    const store = useTodoStore()
    const result = await store.addAssignee('p1', 't1', 'todo1', 'm1')

    expect(result).toBeNull()
    expect(store.error).toBe('Failed to add assignee.')
  })

  it('removeAssignee calls DELETE without refetching', async () => {
    vi.mocked(client.DELETE).mockResolvedValue({} as never)

    const store = useTodoStore()
    await store.removeAssignee('p1', 't1', 'todo1', 'm1')

    expect(client.DELETE).toHaveBeenCalled()
    expect(client.GET).not.toHaveBeenCalled()
  })

  it('removeAssignee sets error on failure', async () => {
    vi.mocked(client.DELETE).mockRejectedValue(new Error('fail'))

    const store = useTodoStore()
    await store.removeAssignee('p1', 't1', 'todo1', 'm1')

    expect(store.error).toBe('Failed to remove assignee.')
  })
})

import { describe, it, expect, beforeEach, vi } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { useTaskStore } from '../task'

vi.mock('@/api/client', () => ({
  default: {
    GET: vi.fn(),
    POST: vi.fn(),
    PUT: vi.fn(),
    DELETE: vi.fn(),
  },
}))

import client from '@/api/client'

describe('useTaskStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('starts with empty state', () => {
    const store = useTaskStore()
    expect(store.tasks).toEqual([])
    expect(store.currentTask).toBeNull()
    expect(store.loading).toBe(false)
    expect(store.error).toBe('')
  })

  it('fetchTasks loads tasks', async () => {
    const mockTasks = [
      { id: 't1', projectId: 'p1', name: 'Task 1', status: 'open', createdAt: '', updatedAt: '' },
    ]
    vi.mocked(client.GET).mockResolvedValue({ data: { tasks: mockTasks } } as never)

    const store = useTaskStore()
    await store.fetchTasks('p1')

    expect(store.tasks).toEqual(mockTasks)
    expect(client.GET).toHaveBeenCalled()
  })

  it('fetchTasks with filters passes query params', async () => {
    vi.mocked(client.GET).mockResolvedValue({ data: { tasks: [] } } as never)

    const store = useTaskStore()
    await store.fetchTasks('p1', 'open' as never, 'tag1')

    expect(client.GET).toHaveBeenCalledWith(
      '/api/v1/projects/{projectId}/tasks',
      expect.objectContaining({
        params: expect.objectContaining({
          path: { projectId: 'p1' },
          query: { status: 'open', tagId: 'tag1' },
        }),
      }),
    )
  })

  it('fetchTasks sets error on failure', async () => {
    vi.mocked(client.GET).mockRejectedValue(new Error('fail'))

    const store = useTaskStore()
    await store.fetchTasks('p1')

    expect(store.error).toBe('Failed to load tasks.')
  })

  it('getTask loads a single task', async () => {
    const mockTask = { id: 't1', projectId: 'p1', name: 'Task 1', status: 'open', createdAt: '', updatedAt: '' }
    vi.mocked(client.GET).mockResolvedValue({ data: mockTask } as never)

    const store = useTaskStore()
    await store.getTask('p1', 't1')

    expect(store.currentTask).toEqual(mockTask)
  })

  it('getTask sets error on failure', async () => {
    vi.mocked(client.GET).mockRejectedValue(new Error('fail'))

    const store = useTaskStore()
    await store.getTask('p1', 't1')

    expect(store.error).toBe('Failed to load task.')
  })

  it('createTask calls POST and refetches', async () => {
    const mockTask = { id: 't1', projectId: 'p1', name: 'New Task', status: 'open', createdAt: '', updatedAt: '' }
    vi.mocked(client.POST).mockResolvedValue({ data: mockTask } as never)
    vi.mocked(client.GET).mockResolvedValue({ data: { tasks: [] } } as never)

    const store = useTaskStore()
    const result = await store.createTask('p1', 'tmpl1', 'tag1', 'New Task', 'desc')

    expect(result).toEqual(mockTask)
    expect(client.POST).toHaveBeenCalled()
    expect(client.GET).toHaveBeenCalled()
  })

  it('createTask sets error on failure', async () => {
    vi.mocked(client.POST).mockRejectedValue(new Error('fail'))

    const store = useTaskStore()
    const result = await store.createTask('p1', 'tmpl1', 'tag1', 'New Task')

    expect(result).toBeNull()
    expect(store.error).toBe('Failed to create task.')
  })

  it('updateTask sets currentTask from response', async () => {
    const mockTask = { id: 't1', projectId: 'p1', name: 'Updated', status: 'open', createdAt: '', updatedAt: '' }
    vi.mocked(client.PUT).mockResolvedValue({ data: mockTask } as never)

    const store = useTaskStore()
    await store.updateTask('p1', 't1', { name: 'Updated' })

    expect(store.currentTask).toEqual(mockTask)
    expect(client.PUT).toHaveBeenCalled()
  })

  it('updateTaskStatus sets currentTask and refetches', async () => {
    const mockTask = { id: 't1', projectId: 'p1', name: 'Task 1', status: 'in_progress', createdAt: '', updatedAt: '' }
    vi.mocked(client.PUT).mockResolvedValue({ data: mockTask } as never)
    vi.mocked(client.GET).mockResolvedValue({ data: { tasks: [] } } as never)

    const store = useTaskStore()
    await store.updateTaskStatus('p1', 't1', 'in_progress' as never)

    expect(store.currentTask).toEqual(mockTask)
    expect(client.PUT).toHaveBeenCalled()
    expect(client.GET).toHaveBeenCalled()
  })

  it('deleteTask calls DELETE and refetches', async () => {
    vi.mocked(client.DELETE).mockResolvedValue({} as never)
    vi.mocked(client.GET).mockResolvedValue({ data: { tasks: [] } } as never)

    const store = useTaskStore()
    await store.deleteTask('p1', 't1')

    expect(client.DELETE).toHaveBeenCalled()
    expect(client.GET).toHaveBeenCalled()
  })
})

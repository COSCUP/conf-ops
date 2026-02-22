import { describe, it, expect, beforeEach, vi } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { useToolConfigStore } from '../toolConfig'

vi.mock('@/api/client', () => ({
  default: {
    GET: vi.fn(),
    POST: vi.fn(),
    PUT: vi.fn(),
    DELETE: vi.fn(),
  },
}))

import client from '@/api/client'

describe('useToolConfigStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('starts with empty state', () => {
    const store = useToolConfigStore()
    expect(store.tools).toEqual([])
    expect(store.toolConfigs).toEqual([])
    expect(store.loading).toBe(false)
    expect(store.error).toBeNull()
  })

  it('listTools loads tools', async () => {
    const mockTools = [
      {
        name: 'createTask',
        displayName: 'Create Task',
        category: 'core' as const,
        description: 'Create a task',
        requiresConfirmation: true,
      },
    ]
    vi.mocked(client.GET).mockResolvedValue({ data: { tools: mockTools } } as never)

    const store = useToolConfigStore()
    await store.listTools('project-1')

    expect(store.tools).toEqual(mockTools)
    expect(client.GET).toHaveBeenCalled()
  })

  it('listToolConfigs loads configs', async () => {
    const mockConfigs = [
      {
        id: 'config-1',
        scopeType: 'project',
        scopeId: 'project-1',
        toolType: 'builtin',
        toolName: 'smtp/sendEmail',
        enabled: true,
        config: { host: 'smtp.example.com' },
        createdAt: '2025-01-01T00:00:00Z',
        updatedAt: '2025-01-01T00:00:00Z',
      },
    ]
    vi.mocked(client.GET).mockResolvedValue({ data: { toolConfigs: mockConfigs } } as never)

    const store = useToolConfigStore()
    await store.listToolConfigs('project-1')

    expect(store.toolConfigs).toEqual(mockConfigs)
  })

  it('createToolConfig adds to list', async () => {
    const newConfig = {
      id: 'config-2',
      scopeType: 'project',
      scopeId: 'project-1',
      toolType: 'builtin',
      toolName: 'hackmd/createDocument',
      enabled: false,
      config: {},
      createdAt: '2025-01-01T00:00:00Z',
      updatedAt: '2025-01-01T00:00:00Z',
    }
    vi.mocked(client.POST).mockResolvedValue({ data: newConfig } as never)

    const store = useToolConfigStore()
    const result = await store.createToolConfig('project-1', {
      toolName: 'hackmd/createDocument',
      enabled: false,
      config: {},
    })

    expect(result).toEqual(newConfig)
    expect(store.toolConfigs).toHaveLength(1)
  })

  it('deleteToolConfig removes from list', async () => {
    vi.mocked(client.DELETE).mockResolvedValue({} as never)

    const store = useToolConfigStore()
    store.toolConfigs = [
      {
        id: 'config-1',
        scopeType: 'project',
        scopeId: 'project-1',
        toolType: 'builtin',
        toolName: 'test',
        enabled: true,
        config: {},
        createdAt: '',
        updatedAt: '',
      },
    ]

    const result = await store.deleteToolConfig('project-1', 'config-1')
    expect(result).toBe(true)
    expect(store.toolConfigs).toHaveLength(0)
  })

  it('executeTool calls API and returns result', async () => {
    const mockResult = { success: true, result: { taskId: 'new-task' }, durationMs: 42 }
    vi.mocked(client.POST).mockResolvedValue({ data: mockResult } as never)

    const store = useToolConfigStore()
    const result = await store.executeTool('project-1', 'createTask', {
      taskId: 'task-1',
      parameters: { templateId: 'template-1' },
    })

    expect(result).toEqual(mockResult)
  })

  it('handles listTools error', async () => {
    vi.mocked(client.GET).mockRejectedValue(new Error('Network error'))

    const store = useToolConfigStore()
    await store.listTools('project-1')

    expect(store.error).toBe('Failed to load tools.')
    expect(store.tools).toEqual([])
  })
})

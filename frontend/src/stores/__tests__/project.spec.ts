import { describe, it, expect, beforeEach, vi } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { useProjectStore } from '../project'

vi.mock('@/api/client', () => ({
  default: {
    GET: vi.fn(),
    POST: vi.fn(),
    PUT: vi.fn(),
    DELETE: vi.fn(),
  },
}))

import client from '@/api/client'

describe('useProjectStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('starts with empty state', () => {
    const store = useProjectStore()
    expect(store.projects).toEqual([])
    expect(store.currentProject).toBeNull()
    expect(store.loading).toBe(false)
    expect(store.error).toBe('')
  })

  it('fetchProjects loads projects', async () => {
    const mockProjects = [
      { id: '1', name: 'P1', description: null, status: 'preparing' },
    ]
    vi.mocked(client.GET).mockResolvedValue({ data: { projects: mockProjects } } as never)

    const store = useProjectStore()
    await store.fetchProjects('org-1')

    expect(store.projects).toEqual(mockProjects)
  })

  it('fetchProject loads single project', async () => {
    const mockProject = { id: '1', name: 'P1', status: 'preparing', organizationId: 'org-1' }
    vi.mocked(client.GET).mockResolvedValue({ data: mockProject } as never)

    const store = useProjectStore()
    await store.fetchProject('1')

    expect(store.currentProject).toEqual(mockProject)
  })

  it('createProject calls POST', async () => {
    const mockProject = { id: '1', name: 'New Project', status: 'preparing' }
    vi.mocked(client.POST).mockResolvedValue({ data: mockProject } as never)

    const store = useProjectStore()
    const result = await store.createProject('org-1', 'New Project', 'Desc')

    expect(result).toEqual(mockProject)
    expect(client.POST).toHaveBeenCalled()
  })

  it('copyProject calls POST with copy endpoint', async () => {
    const mockProject = { id: '2', name: 'Copied', status: 'preparing' }
    vi.mocked(client.POST).mockResolvedValue({ data: mockProject } as never)

    const store = useProjectStore()
    const result = await store.copyProject('org-1', 'source-1', 'Copied', 'Desc')

    expect(result).toEqual(mockProject)
  })

  it('deleteProject calls DELETE and clears current', async () => {
    vi.mocked(client.DELETE).mockResolvedValue({} as never)

    const store = useProjectStore()
    await store.deleteProject('1')

    expect(client.DELETE).toHaveBeenCalled()
    expect(store.currentProject).toBeNull()
  })

  it('updateProjectStatus calls PUT', async () => {
    const mockProject = { id: '1', name: 'P1', status: 'active' }
    vi.mocked(client.PUT).mockResolvedValue({ data: mockProject } as never)

    const store = useProjectStore()
    await store.updateProjectStatus('1', 'active')

    expect(store.currentProject).toEqual(mockProject)
  })

  it('fetchProjects sets error on failure', async () => {
    vi.mocked(client.GET).mockRejectedValue(new Error('fail'))

    const store = useProjectStore()
    await store.fetchProjects('org-1')

    expect(store.error).toBe('Failed to load projects.')
  })
})

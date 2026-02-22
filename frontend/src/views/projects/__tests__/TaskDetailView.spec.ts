import { describe, it, expect, beforeEach, vi } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { createRouter, createMemoryHistory } from 'vue-router'
import TaskDetailView from '../TaskDetailView.vue'

vi.mock('@/api/client', () => ({
  default: {
    GET: vi.fn(),
    POST: vi.fn(),
    PUT: vi.fn(),
    DELETE: vi.fn(),
    use: vi.fn(),
  },
  setupAuthInterceptor: vi.fn(),
}))

import client from '@/api/client'

function createTestRouter() {
  return createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: '/projects/:projectId/tasks', name: 'project-tasks', component: { template: '<div />' } },
      { path: '/projects/:projectId/tasks/:taskId', name: 'task-detail', component: { template: '<div />' } },
    ],
  })
}

const mockTask = {
  id: 'task-1',
  projectId: 'proj-1',
  taskTemplateId: 'tmpl-1',
  ownerTagId: 'tag-1',
  name: 'Test Task',
  description: 'A test task',
  status: 'pending',
  createdAt: '2025-01-01T00:00:00Z',
  updatedAt: '2025-01-01T00:00:00Z',
}

function mockGetByPath() {
  vi.mocked(client.GET).mockImplementation(((path: string) => {
    if (path.includes('/todos')) {
      return Promise.resolve({ data: { todos: [] } })
    }
    if (path.includes('/data-entries')) {
      return Promise.resolve({ data: { entries: [] } })
    }
    if (path.includes('/data-schemas')) {
      return Promise.resolve({ data: { dataSchemas: [] } })
    }
    if (path.includes('/members')) {
      return Promise.resolve({ data: { members: [] } })
    }
    if (path.includes('/email-threads')) {
      return Promise.resolve({ data: { threads: [], pagination: { hasMore: false, nextCursor: null } } })
    }
    // task detail
    return Promise.resolve({ data: mockTask })
  }) as typeof client.GET)
}

describe('TaskDetailView', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('renders Task Detail heading', async () => {
    mockGetByPath()

    const router = createTestRouter()
    await router.push('/projects/proj-1/tasks/task-1')
    await router.isReady()

    const wrapper = mount(TaskDetailView, {
      global: { plugins: [router] },
    })

    expect(wrapper.find('h1').text()).toBe('Task Detail')
  })

  it('renders Task Info, Todos, and Data Entries sections', async () => {
    mockGetByPath()

    const router = createTestRouter()
    await router.push('/projects/proj-1/tasks/task-1')
    await router.isReady()

    const wrapper = mount(TaskDetailView, {
      global: { plugins: [router] },
    })

    await flushPromises()

    expect(wrapper.text()).toContain('Task Info')
    expect(wrapper.text()).toContain('Todos')
    expect(wrapper.text()).toContain('Data Entries')
  })
})

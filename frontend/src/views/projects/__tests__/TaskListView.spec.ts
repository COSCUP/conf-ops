import { describe, it, expect, beforeEach, vi } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { createRouter, createMemoryHistory } from 'vue-router'
import TaskListView from '../TaskListView.vue'

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

function mockGetByPath() {
  vi.mocked(client.GET).mockImplementation(((path: string) => {
    if (path.includes('/task-templates')) {
      return Promise.resolve({ data: { templates: [] } })
    }
    if (path.includes('/member-tags')) {
      return Promise.resolve({ data: { tags: [] } })
    }
    // tasks
    return Promise.resolve({ data: { tasks: [] } })
  }) as typeof client.GET)
}

describe('TaskListView', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('renders Tasks heading', async () => {
    mockGetByPath()

    const router = createTestRouter()
    await router.push('/projects/proj-1/tasks')
    await router.isReady()

    const wrapper = mount(TaskListView, {
      global: { plugins: [router] },
    })

    expect(wrapper.find('h1').text()).toBe('Tasks')
  })

  it('shows empty message when no tasks', async () => {
    mockGetByPath()

    const router = createTestRouter()
    await router.push('/projects/proj-1/tasks')
    await router.isReady()

    const wrapper = mount(TaskListView, {
      global: { plugins: [router] },
    })

    await flushPromises()

    expect(wrapper.text()).toContain('No tasks yet.')
  })

  it('renders Create Task and Filters sections', async () => {
    mockGetByPath()

    const router = createTestRouter()
    await router.push('/projects/proj-1/tasks')
    await router.isReady()

    const wrapper = mount(TaskListView, {
      global: { plugins: [router] },
    })

    expect(wrapper.text()).toContain('Create Task')
    expect(wrapper.text()).toContain('Filters')
  })
})

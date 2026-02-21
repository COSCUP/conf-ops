import { describe, it, expect, beforeEach, vi } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { createRouter, createMemoryHistory } from 'vue-router'
import MyTodosView from '../MyTodosView.vue'

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
      { path: '/my-todos', name: 'my-todos', component: { template: '<div />' } },
      { path: '/projects/:projectId/tasks/:taskId', name: 'task-detail', component: { template: '<div />' } },
    ],
  })
}

const mockItems = [
  {
    id: 'todo-1',
    taskId: 'task-1',
    title: 'Review PR',
    description: 'Review the pull request',
    status: 'open',
    todoType: 'template',
    dueDate: '2025-02-01T00:00:00Z',
    projectId: 'proj-1',
    projectName: 'Project Alpha',
    taskName: 'Sprint Task',
    createdAt: '2025-01-01T00:00:00Z',
    updatedAt: '2025-01-01T00:00:00Z',
  },
  {
    id: 'todo-2',
    taskId: 'task-2',
    title: 'Write docs',
    description: null,
    status: 'completed',
    todoType: 'ad_hoc',
    dueDate: null,
    projectId: 'proj-2',
    projectName: 'Project Beta',
    taskName: 'Documentation Task',
    createdAt: '2025-01-02T00:00:00Z',
    updatedAt: '2025-01-02T00:00:00Z',
  },
]

describe('MyTodosView', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('renders heading', async () => {
    vi.mocked(client.GET).mockResolvedValue({ data: { items: [] } } as never)

    const router = createTestRouter()
    await router.push('/my-todos')
    await router.isReady()

    const wrapper = mount(MyTodosView, {
      global: { plugins: [router] },
    })

    expect(wrapper.find('h1').text()).toBe('My Todos')
  })

  it('renders empty state when no todos', async () => {
    vi.mocked(client.GET).mockResolvedValue({ data: { items: [] } } as never)

    const router = createTestRouter()
    await router.push('/my-todos')
    await router.isReady()

    const wrapper = mount(MyTodosView, {
      global: { plugins: [router] },
    })

    await flushPromises()

    expect(wrapper.text()).toContain('You have no assigned todos.')
  })

  it('renders todos grouped by project', async () => {
    vi.mocked(client.GET).mockResolvedValue({ data: { items: mockItems } } as never)

    const router = createTestRouter()
    await router.push('/my-todos')
    await router.isReady()

    const wrapper = mount(MyTodosView, {
      global: { plugins: [router] },
    })

    await flushPromises()

    expect(wrapper.text()).toContain('Project Alpha')
    expect(wrapper.text()).toContain('Project Beta')
    expect(wrapper.text()).toContain('Review PR')
    expect(wrapper.text()).toContain('Write docs')
  })

  it('renders status filter', async () => {
    vi.mocked(client.GET).mockResolvedValue({ data: { items: [] } } as never)

    const router = createTestRouter()
    await router.push('/my-todos')
    await router.isReady()

    const wrapper = mount(MyTodosView, {
      global: { plugins: [router] },
    })

    const select = wrapper.find('select')
    expect(select.exists()).toBe(true)
    // Should have All + open + completed options
    const options = select.findAll('option')
    expect(options.length).toBe(3)
  })

  it('displays error message on failure', async () => {
    vi.mocked(client.GET).mockRejectedValue(new Error('fail'))

    const router = createTestRouter()
    await router.push('/my-todos')
    await router.isReady()

    const wrapper = mount(MyTodosView, {
      global: { plugins: [router] },
    })

    await flushPromises()

    expect(wrapper.text()).toContain('Failed to load my todos.')
  })
})

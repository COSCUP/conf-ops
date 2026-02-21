import { describe, it, expect, beforeEach, vi } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { createRouter, createMemoryHistory } from 'vue-router'
import TaskTemplateListView from '../TaskTemplateListView.vue'

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
      { path: '/projects/:projectId/task-templates', name: 'project-task-templates', component: { template: '<div />' } },
      { path: '/projects/:projectId/task-templates/:templateId', name: 'task-template-editor', component: { template: '<div />' } },
    ],
  })
}

describe('TaskTemplateListView', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('renders Task Templates heading', async () => {
    vi.mocked(client.GET).mockResolvedValue({ data: { templates: [] } } as never)

    const router = createTestRouter()
    await router.push('/projects/proj-1/task-templates')
    await router.isReady()

    const wrapper = mount(TaskTemplateListView, {
      global: { plugins: [router] },
    })

    expect(wrapper.find('h1').text()).toBe('Task Templates')
  })

  it('shows empty message when no templates', async () => {
    vi.mocked(client.GET).mockResolvedValue({ data: { templates: [] } } as never)

    const router = createTestRouter()
    await router.push('/projects/proj-1/task-templates')
    await router.isReady()

    const wrapper = mount(TaskTemplateListView, {
      global: { plugins: [router] },
    })

    await flushPromises()

    expect(wrapper.text()).toContain('No task templates yet.')
  })

  it('renders template names after loading', async () => {
    const mockTemplates = [
      { id: 't1', projectId: 'proj-1', name: 'Sprint Template', description: 'A sprint', createdAt: '2025-01-01T00:00:00Z', updatedAt: '2025-01-01T00:00:00Z' },
    ]
    vi.mocked(client.GET).mockResolvedValue({ data: { templates: mockTemplates } } as never)

    const router = createTestRouter()
    await router.push('/projects/proj-1/task-templates')
    await router.isReady()

    const wrapper = mount(TaskTemplateListView, {
      global: { plugins: [router] },
    })

    await flushPromises()

    expect(wrapper.text()).toContain('Sprint Template')
    expect(wrapper.text()).toContain('A sprint')
  })

  it('renders Create Template form', async () => {
    vi.mocked(client.GET).mockResolvedValue({ data: { templates: [] } } as never)

    const router = createTestRouter()
    await router.push('/projects/proj-1/task-templates')
    await router.isReady()

    const wrapper = mount(TaskTemplateListView, {
      global: { plugins: [router] },
    })

    expect(wrapper.text()).toContain('Create Template')
    const createButton = wrapper.findAll('button').find((b) => b.text() === 'Create')
    expect(createButton).toBeTruthy()
  })
})

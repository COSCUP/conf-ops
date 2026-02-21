import { describe, it, expect, beforeEach, vi } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { createRouter, createMemoryHistory } from 'vue-router'
import TaskTemplateEditorView from '../TaskTemplateEditorView.vue'

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

const mockTemplate = {
  id: 'tmpl-1',
  projectId: 'proj-1',
  name: 'My Template',
  description: 'Test description',
  createdAt: '2025-01-01T00:00:00Z',
  updatedAt: '2025-01-01T00:00:00Z',
}

function mockGetByPath() {
  vi.mocked(client.GET).mockImplementation(((path: string) => {
    if (path.includes('/todo-templates')) {
      return Promise.resolve({ data: { todoTemplates: [] } })
    }
    if (path.includes('/data-schemas')) {
      return Promise.resolve({ data: { dataSchemas: [] } })
    }
    if (path.includes('/tags') || path.includes('/member-tags')) {
      return Promise.resolve({ data: { tags: [] } })
    }
    // single template
    return Promise.resolve({ data: mockTemplate })
  }) as typeof client.GET)
}

describe('TaskTemplateEditorView', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('renders Edit Template heading', async () => {
    mockGetByPath()

    const router = createTestRouter()
    await router.push('/projects/proj-1/task-templates/tmpl-1')
    await router.isReady()

    const wrapper = mount(TaskTemplateEditorView, {
      global: { plugins: [router] },
    })

    expect(wrapper.find('h1').text()).toBe('Edit Template')
  })

  it('renders Template Info, Todo Templates, Data Schemas, and Linked Tags sections', async () => {
    mockGetByPath()

    const router = createTestRouter()
    await router.push('/projects/proj-1/task-templates/tmpl-1')
    await router.isReady()

    const wrapper = mount(TaskTemplateEditorView, {
      global: { plugins: [router] },
    })

    await flushPromises()

    expect(wrapper.text()).toContain('Template Info')
    expect(wrapper.text()).toContain('Todo Templates')
    expect(wrapper.text()).toContain('Data Schemas')
    expect(wrapper.text()).toContain('Linked Tags')
  })
})

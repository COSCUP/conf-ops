import { describe, it, expect, beforeEach, vi } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { createRouter, createMemoryHistory } from 'vue-router'
import AuditLogView from '../AuditLogView.vue'

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
      { path: '/projects/:projectId/audit-logs', name: 'project-audit-logs', component: { template: '<div />' } },
    ],
  })
}

describe('AuditLogView', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('renders Audit Logs heading', async () => {
    vi.mocked(client.GET).mockResolvedValue({ data: { data: [] } } as never)

    const router = createTestRouter()
    await router.push('/projects/proj-1/audit-logs')
    await router.isReady()

    const wrapper = mount(AuditLogView, {
      global: { plugins: [router] },
    })

    await flushPromises()
    expect(wrapper.find('h1').text()).toBe('Audit Logs')
  })

  it('displays audit log entries', async () => {
    vi.mocked(client.GET).mockResolvedValue({
      data: {
        data: [
          {
            id: 'log-1',
            actorType: 'account',
            actorId: 'user-1',
            action: 'task.create',
            resourceType: 'task',
            resourceId: 'task-1',
            contextType: 'project',
            contextId: 'proj-1',
            details: {},
            createdAt: '2024-01-01T00:00:00Z',
          },
        ],
      },
    } as never)

    const router = createTestRouter()
    await router.push('/projects/proj-1/audit-logs')
    await router.isReady()

    const wrapper = mount(AuditLogView, {
      global: { plugins: [router] },
    })

    await flushPromises()
    expect(wrapper.text()).toContain('task.create')
    expect(wrapper.text()).toContain('account')
  })

  it('shows filter controls', async () => {
    vi.mocked(client.GET).mockResolvedValue({ data: { data: [] } } as never)

    const router = createTestRouter()
    await router.push('/projects/proj-1/audit-logs')
    await router.isReady()

    const wrapper = mount(AuditLogView, {
      global: { plugins: [router] },
    })

    await flushPromises()
    expect(wrapper.find('select').exists()).toBe(true)
    const filterButton = wrapper.findAll('button').find(b => b.text() === 'Filter')
    expect(filterButton).toBeDefined()
  })
})

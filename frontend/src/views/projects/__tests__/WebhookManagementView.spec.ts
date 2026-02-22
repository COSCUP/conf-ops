import { describe, it, expect, beforeEach, vi } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { createRouter, createMemoryHistory } from 'vue-router'
import WebhookManagementView from '../WebhookManagementView.vue'

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
      { path: '/projects/:projectId/webhooks', name: 'project-webhooks', component: { template: '<div />' } },
    ],
  })
}

describe('WebhookManagementView', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('renders Webhook Management heading', async () => {
    vi.mocked(client.GET).mockResolvedValue({ data: { data: [] } } as never)

    const router = createTestRouter()
    await router.push('/projects/proj-1/webhooks')
    await router.isReady()

    const wrapper = mount(WebhookManagementView, {
      global: { plugins: [router] },
    })

    await flushPromises()
    expect(wrapper.find('h1').text()).toBe('Webhook Management')
  })

  it('displays webhooks from API', async () => {
    vi.mocked(client.GET).mockResolvedValue({
      data: {
        data: [
          {
            id: 'wh-1',
            name: 'Test Webhook',
            url: 'https://example.com/hook',
            hasSecret: true,
            eventTypes: ['task.created'],
            enabled: true,
            createdAt: '2024-01-01T00:00:00Z',
          },
        ],
      },
    } as never)

    const router = createTestRouter()
    await router.push('/projects/proj-1/webhooks')
    await router.isReady()

    const wrapper = mount(WebhookManagementView, {
      global: { plugins: [router] },
    })

    await flushPromises()
    expect(wrapper.text()).toContain('Test Webhook')
    expect(wrapper.text()).toContain('https://example.com/hook')
  })

  it('shows Add Webhook button', async () => {
    vi.mocked(client.GET).mockResolvedValue({ data: { data: [] } } as never)

    const router = createTestRouter()
    await router.push('/projects/proj-1/webhooks')
    await router.isReady()

    const wrapper = mount(WebhookManagementView, {
      global: { plugins: [router] },
    })

    await flushPromises()
    const button = wrapper.findAll('button').find(b => b.text() === 'Add Webhook')
    expect(button).toBeDefined()
  })
})

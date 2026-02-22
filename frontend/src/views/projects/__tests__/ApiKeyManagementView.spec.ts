import { describe, it, expect, beforeEach, vi } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { createRouter, createMemoryHistory } from 'vue-router'
import ApiKeyManagementView from '../ApiKeyManagementView.vue'

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
      { path: '/projects/:projectId/api-keys', name: 'project-api-keys', component: { template: '<div />' } },
    ],
  })
}

describe('ApiKeyManagementView', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('renders API Key Management heading', async () => {
    vi.mocked(client.GET).mockResolvedValue({ data: { data: [] } } as never)

    const router = createTestRouter()
    await router.push('/projects/proj-1/api-keys')
    await router.isReady()

    const wrapper = mount(ApiKeyManagementView, {
      global: { plugins: [router] },
    })

    await flushPromises()
    expect(wrapper.find('h1').text()).toBe('API Key Management')
  })

  it('displays API keys from API', async () => {
    vi.mocked(client.GET).mockResolvedValue({
      data: {
        data: [
          {
            id: 'key-1',
            name: 'CI/CD Key',
            permissions: { scopes: ['read:data'] },
            lastUsedAt: '2024-01-01T00:00:00Z',
            createdAt: '2024-01-01T00:00:00Z',
          },
        ],
      },
    } as never)

    const router = createTestRouter()
    await router.push('/projects/proj-1/api-keys')
    await router.isReady()

    const wrapper = mount(ApiKeyManagementView, {
      global: { plugins: [router] },
    })

    await flushPromises()
    expect(wrapper.text()).toContain('CI/CD Key')
  })

  it('shows created key value only once', async () => {
    vi.mocked(client.GET).mockResolvedValue({ data: { data: [] } } as never)
    vi.mocked(client.POST).mockResolvedValue({
      data: {
        id: 'key-1',
        name: 'New Key',
        apiKey: 'test-raw-key-value-12345',
        permissions: { scopes: ['read:data'] },
        createdAt: '2024-01-01T00:00:00Z',
      },
    } as never)

    const router = createTestRouter()
    await router.push('/projects/proj-1/api-keys')
    await router.isReady()

    const wrapper = mount(ApiKeyManagementView, {
      global: { plugins: [router] },
    })

    await flushPromises()

    // Click "Create API Key" to show form
    const createBtn = wrapper.findAll('button').find(b => b.text() === 'Create API Key')
    await createBtn?.trigger('click')

    // Fill name and create
    const inputs = wrapper.findAll('input')
    const nameInput = inputs.find(i => i.attributes('placeholder') === 'API key name')
    if (nameInput) {
      await nameInput.setValue('New Key')
    }

    const submitBtn = wrapper.findAll('button').find(b => b.text() === 'Create')
    await submitBtn?.trigger('click')
    await flushPromises()

    // Should display the raw key
    expect(wrapper.text()).toContain('test-raw-key-value-12345')
    expect(wrapper.text()).toContain('Copy this key now')
  })
})

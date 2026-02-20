import { describe, it, expect, beforeEach, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { createRouter, createMemoryHistory } from 'vue-router'
import OrganizationListView from '../OrganizationListView.vue'

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
      { path: '/organizations', name: 'organizations', component: { template: '<div />' } },
      { path: '/organizations/:orgId/settings', name: 'org-settings', component: { template: '<div />' } },
      { path: '/organizations/:orgId/projects', name: 'org-projects', component: { template: '<div />' } },
    ],
  })
}

describe('OrganizationListView', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('renders the organizations heading', async () => {
    vi.mocked(client.GET).mockResolvedValue({ data: { organizations: [] } } as never)

    const router = createTestRouter()
    await router.push('/organizations')
    await router.isReady()

    const wrapper = mount(OrganizationListView, {
      global: { plugins: [router] },
    })

    expect(wrapper.find('h1').text()).toBe('Organizations')
  })

  it('shows empty state when no organizations', async () => {
    vi.mocked(client.GET).mockResolvedValue({ data: { organizations: [] } } as never)

    const router = createTestRouter()
    await router.push('/organizations')
    await router.isReady()

    const wrapper = mount(OrganizationListView, {
      global: { plugins: [router] },
    })

    await vi.dynamicImportSettled()

    expect(wrapper.text()).toContain("don't belong to any organizations")
  })

  it('renders organization cards', async () => {
    const mockOrgs = [
      { id: '1', name: 'Test Org', description: 'A test', logoUrl: null, createdAt: '', role: 'org_owner' },
    ]
    vi.mocked(client.GET).mockResolvedValue({ data: { organizations: mockOrgs } } as never)

    const router = createTestRouter()
    await router.push('/organizations')
    await router.isReady()

    const wrapper = mount(OrganizationListView, {
      global: { plugins: [router] },
    })

    await vi.dynamicImportSettled()

    expect(wrapper.text()).toContain('Test Org')
    expect(wrapper.text()).toContain('A test')
  })

  it('shows create form when button clicked', async () => {
    vi.mocked(client.GET).mockResolvedValue({ data: { organizations: [] } } as never)

    const router = createTestRouter()
    await router.push('/organizations')
    await router.isReady()

    const wrapper = mount(OrganizationListView, {
      global: { plugins: [router] },
    })

    await wrapper.find('button').trigger('click')

    expect(wrapper.text()).toContain('Create Organization')
  })
})

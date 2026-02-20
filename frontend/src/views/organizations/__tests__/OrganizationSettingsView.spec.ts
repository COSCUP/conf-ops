import { describe, it, expect, beforeEach, vi } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { createRouter, createMemoryHistory } from 'vue-router'
import OrganizationSettingsView from '../OrganizationSettingsView.vue'

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

const mockOrg = {
  id: 'org-1',
  name: 'Test Organization',
  description: 'A test org',
  logoUrl: null,
  createdAt: '2024-01-01T00:00:00Z',
  updatedAt: '2024-01-01T00:00:00Z',
}

const mockMembers = [
  { id: 'mem-1', name: 'Alice', email: 'alice@example.com', role: 'org_owner', createdAt: '' },
  { id: 'mem-2', name: 'Bob', email: 'bob@example.com', role: 'org_member', createdAt: '' },
]

function createTestRouter() {
  return createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: '/organizations/:orgId/settings', name: 'org-settings', component: { template: '<div />' } },
      { path: '/organizations', name: 'organizations', component: { template: '<div />' } },
    ],
  })
}

function mockClientGET() {
  vi.mocked(client.GET).mockImplementation(((url: string) => {
    if (url.includes('/members')) {
      return Promise.resolve({ data: { members: mockMembers } })
    }
    return Promise.resolve({ data: mockOrg })
  }) as never)
}

describe('OrganizationSettingsView', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('renders Organization Settings heading', async () => {
    mockClientGET()

    const router = createTestRouter()
    await router.push('/organizations/org-1/settings')
    await router.isReady()

    const wrapper = mount(OrganizationSettingsView, {
      global: { plugins: [router] },
    })

    expect(wrapper.find('h1').text()).toBe('Organization Settings')
  })

  it('shows organization data after loading', async () => {
    mockClientGET()

    const router = createTestRouter()
    await router.push('/organizations/org-1/settings')
    await router.isReady()

    const wrapper = mount(OrganizationSettingsView, {
      global: { plugins: [router] },
    })

    await flushPromises()

    const inputs = wrapper.findAll('input')
    const nameInput = inputs.find((i) => i.element.value === 'Test Organization')
    const descInput = inputs.find((i) => i.element.value === 'A test org')
    expect(nameInput).toBeTruthy()
    expect(descInput).toBeTruthy()
  })

  it('renders member list', async () => {
    mockClientGET()

    const router = createTestRouter()
    await router.push('/organizations/org-1/settings')
    await router.isReady()

    const wrapper = mount(OrganizationSettingsView, {
      global: { plugins: [router] },
    })

    await flushPromises()

    expect(wrapper.text()).toContain('Alice')
    expect(wrapper.text()).toContain('alice@example.com')
    expect(wrapper.text()).toContain('Bob')
    expect(wrapper.text()).toContain('bob@example.com')
  })

  it('renders invite form with email input and role select', async () => {
    mockClientGET()

    const router = createTestRouter()
    await router.push('/organizations/org-1/settings')
    await router.isReady()

    const wrapper = mount(OrganizationSettingsView, {
      global: { plugins: [router] },
    })

    await flushPromises()

    const emailInput = wrapper.find('input[type="email"]')
    expect(emailInput.exists()).toBe(true)

    const selects = wrapper.findAll('select')
    expect(selects.length).toBeGreaterThanOrEqual(1)

    const inviteButton = wrapper.findAll('button').find((b) => b.text() === 'Invite')
    expect(inviteButton).toBeTruthy()
  })

  it('renders danger zone delete button', async () => {
    mockClientGET()

    const router = createTestRouter()
    await router.push('/organizations/org-1/settings')
    await router.isReady()

    const wrapper = mount(OrganizationSettingsView, {
      global: { plugins: [router] },
    })

    await flushPromises()

    expect(wrapper.text()).toContain('Danger Zone')
    const deleteButton = wrapper.findAll('button').find((b) => b.text() === 'Delete Organization')
    expect(deleteButton).toBeTruthy()
  })
})

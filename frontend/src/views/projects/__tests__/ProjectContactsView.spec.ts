import { describe, it, expect, beforeEach, vi } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { createRouter, createMemoryHistory } from 'vue-router'
import ProjectContactsView from '../ProjectContactsView.vue'

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
      { path: '/projects/:projectId/contacts', name: 'project-contacts', component: { template: '<div />' } },
    ],
  })
}

describe('ProjectContactsView', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('renders Project Contacts heading', async () => {
    vi.mocked(client.GET).mockResolvedValue({ data: { contacts: [] } } as never)

    const router = createTestRouter()
    await router.push('/projects/proj-1/contacts')
    await router.isReady()

    const wrapper = mount(ProjectContactsView, {
      global: { plugins: [router] },
    })

    expect(wrapper.find('h1').text()).toBe('Project Contacts')
  })

  it('shows empty message when no contacts', async () => {
    vi.mocked(client.GET).mockResolvedValue({ data: { contacts: [] } } as never)

    const router = createTestRouter()
    await router.push('/projects/proj-1/contacts')
    await router.isReady()

    const wrapper = mount(ProjectContactsView, {
      global: { plugins: [router] },
    })

    await flushPromises()

    expect(wrapper.text()).toContain('No contacts associated with this project.')
  })

  it('renders contact list after loading', async () => {
    const mockContacts = [
      { id: 'c1', organizationId: 'org-1', name: 'Alice', email: 'alice@test.com', mergedIntoId: null, createdAt: '', updatedAt: '' },
    ]
    vi.mocked(client.GET).mockResolvedValue({ data: { contacts: mockContacts } } as never)

    const router = createTestRouter()
    await router.push('/projects/proj-1/contacts')
    await router.isReady()

    const wrapper = mount(ProjectContactsView, {
      global: { plugins: [router] },
    })

    await flushPromises()

    expect(wrapper.text()).toContain('Alice')
    expect(wrapper.text()).toContain('alice@test.com')
  })
})

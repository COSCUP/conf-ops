import { describe, it, expect, beforeEach, vi } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { createRouter, createMemoryHistory } from 'vue-router'
import ContactsView from '../ContactsView.vue'

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
      { path: '/organizations/:orgId/contacts', name: 'org-contacts', component: { template: '<div />' } },
    ],
  })
}

describe('ContactsView', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('renders Contacts heading', async () => {
    vi.mocked(client.GET).mockResolvedValue({ data: { contacts: [] } } as never)

    const router = createTestRouter()
    await router.push('/organizations/org-1/contacts')
    await router.isReady()

    const wrapper = mount(ContactsView, {
      global: { plugins: [router] },
    })

    expect(wrapper.find('h1').text()).toBe('Contacts')
  })

  it('renders contact list after loading', async () => {
    const mockContacts = [
      { id: 'c1', organizationId: 'org-1', name: 'Alice', email: 'alice@test.com', mergedIntoId: null, createdAt: '', updatedAt: '' },
    ]
    vi.mocked(client.GET).mockResolvedValue({ data: { contacts: mockContacts } } as never)

    const router = createTestRouter()
    await router.push('/organizations/org-1/contacts')
    await router.isReady()

    const wrapper = mount(ContactsView, {
      global: { plugins: [router] },
    })

    await flushPromises()

    expect(wrapper.text()).toContain('Alice')
    expect(wrapper.text()).toContain('alice@test.com')
  })

  it('shows empty message when no contacts', async () => {
    vi.mocked(client.GET).mockResolvedValue({ data: { contacts: [] } } as never)

    const router = createTestRouter()
    await router.push('/organizations/org-1/contacts')
    await router.isReady()

    const wrapper = mount(ContactsView, {
      global: { plugins: [router] },
    })

    await flushPromises()

    expect(wrapper.text()).toContain('No contacts yet.')
  })

  it('renders create contact form', async () => {
    vi.mocked(client.GET).mockResolvedValue({ data: { contacts: [] } } as never)

    const router = createTestRouter()
    await router.push('/organizations/org-1/contacts')
    await router.isReady()

    const wrapper = mount(ContactsView, {
      global: { plugins: [router] },
    })

    expect(wrapper.text()).toContain('Create Contact')
    const createButton = wrapper.findAll('button').find((b) => b.text() === 'Create')
    expect(createButton).toBeTruthy()
  })

  it('renders merge contacts section', async () => {
    vi.mocked(client.GET).mockResolvedValue({ data: { contacts: [] } } as never)

    const router = createTestRouter()
    await router.push('/organizations/org-1/contacts')
    await router.isReady()

    const wrapper = mount(ContactsView, {
      global: { plugins: [router] },
    })

    expect(wrapper.text()).toContain('Merge Contacts')
    const mergeButton = wrapper.findAll('button').find((b) => b.text() === 'Merge')
    expect(mergeButton).toBeTruthy()
  })

  it('shows merged badge for merged contacts', async () => {
    const mockContacts = [
      { id: 'c1', organizationId: 'org-1', name: 'Old', email: 'old@test.com', mergedIntoId: 'c2', createdAt: '', updatedAt: '' },
    ]
    vi.mocked(client.GET).mockResolvedValue({ data: { contacts: mockContacts } } as never)

    const router = createTestRouter()
    await router.push('/organizations/org-1/contacts')
    await router.isReady()

    const wrapper = mount(ContactsView, {
      global: { plugins: [router] },
    })

    await flushPromises()

    const badge = wrapper.find('.merged-badge')
    expect(badge.exists()).toBe(true)
    expect(badge.text()).toBe('Merged')
  })
})

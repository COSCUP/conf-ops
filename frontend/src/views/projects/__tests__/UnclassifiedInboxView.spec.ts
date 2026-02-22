import { describe, it, expect, beforeEach, vi } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { createRouter, createMemoryHistory } from 'vue-router'
import UnclassifiedInboxView from '../UnclassifiedInboxView.vue'

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
      {
        path: '/projects/:projectId/unclassified-inbox',
        name: 'unclassified-inbox',
        component: { template: '<div />' },
      },
    ],
  })
}

const mockEmails = [
  {
    id: 'e1',
    projectId: 'p1',
    subject: 'Test Email Subject',
    fromAddress: 'sender@example.com',
    fromName: 'Test Sender',
    snippet: 'Preview of the email...',
    hasAttachments: false,
    receivedAt: '2025-01-01T00:00:00Z',
  },
  {
    id: 'e2',
    projectId: 'p1',
    subject: 'Another Email',
    fromAddress: 'other@example.com',
    fromName: null,
    snippet: null,
    hasAttachments: true,
    receivedAt: '2025-01-02T00:00:00Z',
  },
]

const mockTasks = [
  { id: 't1', projectId: 'p1', name: 'Task One', status: 'open', createdAt: '', updatedAt: '' },
]

const mockProject = {
  id: 'p1',
  name: 'Project One',
  organizationId: 'org1',
  status: 'active',
  description: null,
  createdAt: '',
  updatedAt: '',
}

function mockGetByPath(emails = mockEmails) {
  vi.mocked(client.GET).mockImplementation(((path: string) => {
    if (path.includes('/unassigned-inbox')) {
      return Promise.resolve({
        data: { emails, pagination: { hasMore: false, nextCursor: null } },
      })
    }
    if (path.includes('/tasks') && !path.includes('/task-templates')) {
      return Promise.resolve({ data: { tasks: mockTasks } })
    }
    // project fetch
    return Promise.resolve({ data: mockProject })
  }) as typeof client.GET)
}

describe('UnclassifiedInboxView', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('renders Unclassified Inbox heading', async () => {
    mockGetByPath()

    const router = createTestRouter()
    await router.push('/projects/p1/unclassified-inbox')
    await router.isReady()

    const wrapper = mount(UnclassifiedInboxView, {
      global: { plugins: [createPinia(), router] },
    })

    expect(wrapper.find('h1').text()).toBe('Unclassified Inbox')
  })

  it('renders email list after loading', async () => {
    mockGetByPath()

    const router = createTestRouter()
    await router.push('/projects/p1/unclassified-inbox')
    await router.isReady()

    const wrapper = mount(UnclassifiedInboxView, {
      global: { plugins: [createPinia(), router] },
    })
    await flushPromises()

    expect(wrapper.text()).toContain('Test Email Subject')
    expect(wrapper.text()).toContain('Another Email')
    expect(wrapper.text()).toContain('sender@example.com')
  })

  it('shows empty state message when no emails', async () => {
    vi.mocked(client.GET).mockImplementation(((path: string) => {
      if (path.includes('/unassigned-inbox')) {
        return Promise.resolve({
          data: { emails: [], pagination: { hasMore: false, nextCursor: null } },
        })
      }
      if (path.includes('/tasks')) {
        return Promise.resolve({ data: { tasks: [] } })
      }
      return Promise.resolve({ data: mockProject })
    }) as typeof client.GET)

    const router = createTestRouter()
    await router.push('/projects/p1/unclassified-inbox')
    await router.isReady()

    const wrapper = mount(UnclassifiedInboxView, {
      global: { plugins: [createPinia(), router] },
    })
    await flushPromises()

    expect(wrapper.text()).toContain('No unclassified emails.')
  })

  it('shows Assign to Task button for each email', async () => {
    mockGetByPath()

    const router = createTestRouter()
    await router.push('/projects/p1/unclassified-inbox')
    await router.isReady()

    const wrapper = mount(UnclassifiedInboxView, {
      global: { plugins: [createPinia(), router] },
    })
    await flushPromises()

    const assignButtons = wrapper.findAll('button').filter((b) => b.text() === 'Assign to Task')
    expect(assignButtons.length).toBe(mockEmails.length)
  })

  it('shows assign form when Assign to Task is clicked', async () => {
    mockGetByPath()

    const router = createTestRouter()
    await router.push('/projects/p1/unclassified-inbox')
    await router.isReady()

    const wrapper = mount(UnclassifiedInboxView, {
      global: { plugins: [createPinia(), router] },
    })
    await flushPromises()

    const assignBtn = wrapper.findAll('button').find((b) => b.text() === 'Assign to Task')
    await assignBtn!.trigger('click')

    expect(wrapper.find('select').exists()).toBe(true)
    expect(wrapper.findAll('button').some((b) => b.text() === 'Assign')).toBe(true)
    expect(wrapper.findAll('button').some((b) => b.text() === 'Cancel')).toBe(true)
  })

  it('shows Create Contact button for each email', async () => {
    mockGetByPath()

    const router = createTestRouter()
    await router.push('/projects/p1/unclassified-inbox')
    await router.isReady()

    const wrapper = mount(UnclassifiedInboxView, {
      global: { plugins: [createPinia(), router] },
    })
    await flushPromises()

    const createContactButtons = wrapper.findAll('button').filter((b) => b.text() === 'Create Contact')
    expect(createContactButtons.length).toBe(mockEmails.length)
  })

  it('shows create contact form when Create Contact is clicked', async () => {
    mockGetByPath()

    const router = createTestRouter()
    await router.push('/projects/p1/unclassified-inbox')
    await router.isReady()

    const wrapper = mount(UnclassifiedInboxView, {
      global: { plugins: [createPinia(), router] },
    })
    await flushPromises()

    const createContactBtn = wrapper.findAll('button').find((b) => b.text() === 'Create Contact')
    await createContactBtn!.trigger('click')

    // Form shows Name input pre-filled and email input
    const inputs = wrapper.findAll('input')
    const nameInput = inputs.find((i) => (i.element as HTMLInputElement).value === 'Test Sender')
    expect(nameInput).toBeTruthy()
  })

  it('shows Load More button when pagination has more', async () => {
    vi.mocked(client.GET).mockImplementation(((path: string) => {
      if (path.includes('/unassigned-inbox')) {
        return Promise.resolve({
          data: { emails: mockEmails, pagination: { hasMore: true, nextCursor: 'cursor1' } },
        })
      }
      if (path.includes('/tasks')) {
        return Promise.resolve({ data: { tasks: mockTasks } })
      }
      return Promise.resolve({ data: mockProject })
    }) as typeof client.GET)

    const router = createTestRouter()
    await router.push('/projects/p1/unclassified-inbox')
    await router.isReady()

    const wrapper = mount(UnclassifiedInboxView, {
      global: { plugins: [createPinia(), router] },
    })
    await flushPromises()

    const loadMoreBtn = wrapper.findAll('button').find((b) => b.text() === 'Load More')
    expect(loadMoreBtn).toBeTruthy()
  })

  it('does not show Load More button when no more pages', async () => {
    mockGetByPath()

    const router = createTestRouter()
    await router.push('/projects/p1/unclassified-inbox')
    await router.isReady()

    const wrapper = mount(UnclassifiedInboxView, {
      global: { plugins: [createPinia(), router] },
    })
    await flushPromises()

    const loadMoreBtn = wrapper.findAll('button').find((b) => b.text() === 'Load More')
    expect(loadMoreBtn).toBeFalsy()
  })

  it('shows error message from inbox store', async () => {
    vi.mocked(client.GET).mockImplementation(((path: string) => {
      if (path.includes('/unassigned-inbox')) {
        return Promise.reject(new Error('network error'))
      }
      if (path.includes('/tasks')) {
        return Promise.resolve({ data: { tasks: [] } })
      }
      return Promise.resolve({ data: mockProject })
    }) as typeof client.GET)

    const router = createTestRouter()
    await router.push('/projects/p1/unclassified-inbox')
    await router.isReady()

    const wrapper = mount(UnclassifiedInboxView, {
      global: { plugins: [createPinia(), router] },
    })
    await flushPromises()

    expect(wrapper.text()).toContain('Failed to load unassigned emails.')
  })

  it('shows fromName and fromAddress in email card', async () => {
    mockGetByPath()

    const router = createTestRouter()
    await router.push('/projects/p1/unclassified-inbox')
    await router.isReady()

    const wrapper = mount(UnclassifiedInboxView, {
      global: { plugins: [createPinia(), router] },
    })
    await flushPromises()

    expect(wrapper.text()).toContain('Test Sender <sender@example.com>')
  })

  it('shows Has attachments label when email has attachments', async () => {
    mockGetByPath()

    const router = createTestRouter()
    await router.push('/projects/p1/unclassified-inbox')
    await router.isReady()

    const wrapper = mount(UnclassifiedInboxView, {
      global: { plugins: [createPinia(), router] },
    })
    await flushPromises()

    expect(wrapper.text()).toContain('Has attachments')
  })
})

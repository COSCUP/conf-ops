import { describe, it, expect, beforeEach, vi } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import EmailThreadsPanel from '../EmailThreadsPanel.vue'

vi.mock('@/api/client', () => ({
  default: {
    GET: vi.fn(),
    POST: vi.fn(),
    PUT: vi.fn(),
    DELETE: vi.fn(),
    PATCH: vi.fn(),
  },
}))

import client from '@/api/client'

const mockThreads = [
  {
    id: 'th1',
    taskId: 't1',
    subject: 'Thread One',
    participants: ['a@test.com'],
    lastMessageAt: '2025-01-01T00:00:00Z',
    createdAt: '2025-01-01T00:00:00Z',
    updatedAt: '2025-01-01T00:00:00Z',
  },
  {
    id: 'th2',
    taskId: 't1',
    subject: 'Thread Two',
    participants: ['b@test.com'],
    lastMessageAt: null,
    createdAt: '2025-01-01T00:00:00Z',
    updatedAt: '2025-01-01T00:00:00Z',
  },
]

function createWrapper() {
  return mount(EmailThreadsPanel, {
    props: { projectId: 'p1', taskId: 't1' },
    global: { plugins: [createPinia()] },
  })
}

describe('EmailThreadsPanel', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('renders Email Threads title', async () => {
    vi.mocked(client.GET).mockResolvedValue({
      data: { threads: [], pagination: { hasMore: false, nextCursor: null } },
    } as never)

    const wrapper = createWrapper()
    await flushPromises()

    expect(wrapper.find('h3').text()).toBe('Email Threads')
  })

  it('renders New Thread button', async () => {
    vi.mocked(client.GET).mockResolvedValue({
      data: { threads: [], pagination: { hasMore: false, nextCursor: null } },
    } as never)

    const wrapper = createWrapper()
    await flushPromises()

    const buttons = wrapper.findAll('button')
    const newThreadBtn = buttons.find((b) => b.text() === 'New Thread')
    expect(newThreadBtn).toBeTruthy()
  })

  it('shows empty message when no threads', async () => {
    vi.mocked(client.GET).mockResolvedValue({
      data: { threads: [], pagination: { hasMore: false, nextCursor: null } },
    } as never)

    const wrapper = createWrapper()
    await flushPromises()

    expect(wrapper.text()).toContain('No email threads yet.')
  })

  it('renders thread list after loading', async () => {
    vi.mocked(client.GET).mockResolvedValue({
      data: { threads: mockThreads, pagination: { hasMore: false, nextCursor: null } },
    } as never)

    const wrapper = createWrapper()
    await flushPromises()

    expect(wrapper.text()).toContain('Thread One')
    expect(wrapper.text()).toContain('Thread Two')
  })

  it('toggles new thread form on button click', async () => {
    vi.mocked(client.GET).mockResolvedValue({
      data: { threads: [], pagination: { hasMore: false, nextCursor: null } },
    } as never)

    const wrapper = createWrapper()
    await flushPromises()

    // Form should not be visible initially
    expect(wrapper.text()).not.toContain('Create Thread')

    const newThreadBtn = wrapper.findAll('button').find((b) => b.text() === 'New Thread')
    await newThreadBtn!.trigger('click')

    expect(wrapper.text()).toContain('Create Thread')
    expect(newThreadBtn!.text()).toBe('Cancel')
  })

  it('hides new thread form when Cancel is clicked', async () => {
    vi.mocked(client.GET).mockResolvedValue({
      data: { threads: [], pagination: { hasMore: false, nextCursor: null } },
    } as never)

    const wrapper = createWrapper()
    await flushPromises()

    const newThreadBtn = wrapper.findAll('button').find((b) => b.text() === 'New Thread')
    await newThreadBtn!.trigger('click')
    expect(wrapper.text()).toContain('Create Thread')

    // Now click Cancel
    const cancelBtn = wrapper.findAll('button').find((b) => b.text() === 'Cancel')
    await cancelBtn!.trigger('click')
    expect(wrapper.text()).not.toContain('Create Thread')
  })

  it('expands thread when header is clicked', async () => {
    vi.mocked(client.GET).mockImplementation(((path: string) => {
      if (path.includes('/messages')) {
        return Promise.resolve({ data: { messages: [] } })
      }
      return Promise.resolve({
        data: { threads: mockThreads, pagination: { hasMore: false, nextCursor: null } },
      })
    }) as typeof client.GET)

    const wrapper = createWrapper()
    await flushPromises()

    // Thread messages not shown initially
    expect(wrapper.text()).not.toContain('No messages in this thread yet.')

    const threadHeader = wrapper.find('.thread-header')
    await threadHeader.trigger('click')
    await flushPromises()

    expect(wrapper.text()).toContain('No messages in this thread yet.')
  })

  it('collapses expanded thread when header is clicked again', async () => {
    vi.mocked(client.GET).mockImplementation(((path: string) => {
      if (path.includes('/messages')) {
        return Promise.resolve({ data: { messages: [] } })
      }
      return Promise.resolve({
        data: { threads: mockThreads, pagination: { hasMore: false, nextCursor: null } },
      })
    }) as typeof client.GET)

    const wrapper = createWrapper()
    await flushPromises()

    const threadHeader = wrapper.find('.thread-header')
    await threadHeader.trigger('click')
    await flushPromises()
    expect(wrapper.text()).toContain('No messages in this thread yet.')

    await threadHeader.trigger('click')
    await flushPromises()
    expect(wrapper.text()).not.toContain('No messages in this thread yet.')
  })

  it('renders Delete button for each thread', async () => {
    vi.mocked(client.GET).mockResolvedValue({
      data: { threads: mockThreads, pagination: { hasMore: false, nextCursor: null } },
    } as never)

    const wrapper = createWrapper()
    await flushPromises()

    const deleteButtons = wrapper.findAll('button').filter((b) => b.text() === 'Delete')
    expect(deleteButtons).toHaveLength(mockThreads.length)
  })

  it('calls deleteThread when Delete button is clicked', async () => {
    vi.mocked(client.GET).mockResolvedValue({
      data: { threads: mockThreads, pagination: { hasMore: false, nextCursor: null } },
    } as never)
    vi.mocked(client.DELETE).mockResolvedValue({} as never)

    const wrapper = createWrapper()
    await flushPromises()

    const deleteBtn = wrapper.findAll('button').find((b) => b.text() === 'Delete')
    await deleteBtn!.trigger('click')
    await flushPromises()

    expect(client.DELETE).toHaveBeenCalled()
  })

  it('shows error message when store has an error', async () => {
    vi.mocked(client.GET).mockRejectedValue(new Error('network error'))

    const wrapper = createWrapper()
    await flushPromises()

    expect(wrapper.text()).toContain('Failed to load email threads.')
  })
})

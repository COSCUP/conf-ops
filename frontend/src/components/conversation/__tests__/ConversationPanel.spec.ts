import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import ConversationPanel from '../ConversationPanel.vue'

vi.mock('@/api/client', () => ({
  default: {
    GET: vi.fn(),
    POST: vi.fn(),
    PUT: vi.fn(),
    DELETE: vi.fn(),
  },
}))

vi.mock('@/composables/useWebSocket', () => ({
  useWebSocket: () => ({
    connected: { value: true },
    reconnecting: { value: false },
    connect: vi.fn().mockResolvedValue(undefined),
    disconnect: vi.fn(),
    sendSyncStep2: vi.fn(),
    sendAwarenessUpdate: vi.fn(),
  }),
}))

import client from '@/api/client'

describe('ConversationPanel', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
    vi.mocked(client.GET).mockResolvedValue({
      data: {
        messages: [],
        pagination: { hasMore: false, nextCursor: null },
      },
    } as never)
  })

  function createWrapper() {
    return mount(ConversationPanel, {
      props: { projectId: 'proj-1', taskId: 'task-1' },
    })
  }

  it('renders conversation panel', () => {
    const wrapper = createWrapper()
    expect(wrapper.find('.conversation-panel').exists()).toBe(true)
  })

  it('shows empty message when no messages', async () => {
    const wrapper = createWrapper()
    await vi.dynamicImportSettled()
    expect(wrapper.text()).toContain('No messages yet.')
  })

  it('renders message input', () => {
    const wrapper = createWrapper()
    expect(wrapper.find('textarea').exists()).toBe(true)
  })

  it('shows connection status', () => {
    const wrapper = createWrapper()
    expect(wrapper.find('.connection-status').exists()).toBe(true)
  })
})

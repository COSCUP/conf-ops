import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import NotificationPanel from '../NotificationPanel.vue'
import { useNotificationStore } from '@/stores/notification'
import type { NotificationItem } from '@/stores/notification'

vi.mock('@/api/client', () => ({
  default: {
    GET: vi.fn().mockResolvedValue({ data: { data: [], nextCursor: null } }),
    PUT: vi.fn().mockResolvedValue({}),
    POST: vi.fn(),
    DELETE: vi.fn(),
  },
}))

const sampleNotification: NotificationItem = {
  id: 'n1',
  accountId: 'acc-1',
  type: 'mention',
  title: 'You were mentioned in a task',
  body: 'Check it out',
  referenceType: 'task',
  referenceId: 'task-1',
  projectId: 'proj-1',
  isRead: false,
  readAt: null,
  deliveredChannels: ['in_app'],
  createdAt: '2025-06-01T10:00:00Z',
}

const readNotification: NotificationItem = {
  id: 'n2',
  accountId: 'acc-1',
  type: 'system',
  title: 'System maintenance completed',
  body: null,
  referenceType: null,
  referenceId: null,
  projectId: null,
  isRead: true,
  readAt: '2025-06-01T11:00:00Z',
  deliveredChannels: ['in_app'],
  createdAt: '2025-06-01T09:00:00Z',
}

describe('NotificationPanel', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('renders panel title', () => {
    const wrapper = mount(NotificationPanel)
    expect(wrapper.find('.panel-title').text()).toBe('Notifications')
  })

  it('shows empty state when no notifications', async () => {
    const wrapper = mount(NotificationPanel)
    await wrapper.vm.$nextTick()
    // After onMounted fetch completes with empty data
    const store = useNotificationStore()
    store.notifications = []
    store.loading = false
    await wrapper.vm.$nextTick()
    expect(wrapper.find('.empty').exists()).toBe(true)
    expect(wrapper.text()).toContain('No notifications yet.')
  })

  it('renders notification items', async () => {
    const store = useNotificationStore()
    store.notifications = [sampleNotification, readNotification]
    store.loading = false

    const wrapper = mount(NotificationPanel)
    await wrapper.vm.$nextTick()

    const items = wrapper.findAll('.notification-item')
    expect(items).toHaveLength(2)
    expect(wrapper.text()).toContain('You were mentioned in a task')
    expect(wrapper.text()).toContain('System maintenance completed')
  })

  it('marks unread notifications with unread class', async () => {
    const store = useNotificationStore()
    store.notifications = [sampleNotification, readNotification]
    store.loading = false

    const wrapper = mount(NotificationPanel)
    await wrapper.vm.$nextTick()

    const items = wrapper.findAll('.notification-item')
    expect(items[0]?.classes()).toContain('unread')
    expect(items[1]?.classes()).not.toContain('unread')
  })

  it('shows unread dot for unread notification', async () => {
    const store = useNotificationStore()
    store.notifications = [sampleNotification]
    store.loading = false

    const wrapper = mount(NotificationPanel)
    await wrapper.vm.$nextTick()

    expect(wrapper.find('.unread-dot').exists()).toBe(true)
  })

  it('does not show unread dot for read notification', async () => {
    const store = useNotificationStore()
    store.notifications = [readNotification]
    store.loading = false

    const wrapper = mount(NotificationPanel)
    await wrapper.vm.$nextTick()

    expect(wrapper.find('.read-btn').exists()).toBe(false)
  })

  it('shows type badge', async () => {
    const store = useNotificationStore()
    store.notifications = [sampleNotification]
    store.loading = false

    const wrapper = mount(NotificationPanel)
    await wrapper.vm.$nextTick()

    const badge = wrapper.find('.type-badge')
    expect(badge.exists()).toBe(true)
    expect(badge.text()).toBe('mention')
    expect(badge.classes()).toContain('type-mention')
  })

  it('shows notification body when present', async () => {
    const store = useNotificationStore()
    store.notifications = [sampleNotification]
    store.loading = false

    const wrapper = mount(NotificationPanel)
    await wrapper.vm.$nextTick()

    expect(wrapper.find('.notification-body').exists()).toBe(true)
    expect(wrapper.text()).toContain('Check it out')
  })

  it('shows mark all read button when unread count > 0', async () => {
    const store = useNotificationStore()
    store.unreadCount = 3
    store.notifications = [sampleNotification]
    store.loading = false

    const wrapper = mount(NotificationPanel)
    await wrapper.vm.$nextTick()

    expect(wrapper.find('.mark-all-btn').exists()).toBe(true)
  })

  it('hides mark all read button when unread count is 0', async () => {
    const store = useNotificationStore()
    store.unreadCount = 0
    store.notifications = [readNotification]
    store.loading = false

    const wrapper = mount(NotificationPanel)
    await wrapper.vm.$nextTick()

    expect(wrapper.find('.mark-all-btn').exists()).toBe(false)
  })

  it('shows load more button when hasMore is true', async () => {
    const store = useNotificationStore()
    store.notifications = [sampleNotification]
    store.nextCursor = 'cursor-abc'
    store.loading = false

    const wrapper = mount(NotificationPanel)
    await wrapper.vm.$nextTick()

    expect(wrapper.find('.load-more-btn').exists()).toBe(true)
  })

  it('shows error banner when error exists', async () => {
    const store = useNotificationStore()

    const wrapper = mount(NotificationPanel)
    await wrapper.vm.$nextTick()

    // Set error after onMounted fetch clears it
    store.error = 'Something went wrong'
    await wrapper.vm.$nextTick()

    expect(wrapper.find('.error-banner').exists()).toBe(true)
    expect(wrapper.text()).toContain('Something went wrong')
  })
})

import { describe, it, expect, beforeEach, vi } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { useNotificationStore } from '../notification'

vi.mock('@/api/client', () => ({
  default: {
    GET: vi.fn(),
    PUT: vi.fn(),
    POST: vi.fn(),
    DELETE: vi.fn(),
  },
}))

import client from '@/api/client'

describe('useNotificationStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('starts with empty state', () => {
    const store = useNotificationStore()
    expect(store.notifications).toEqual([])
    expect(store.unreadCount).toBe(0)
    expect(store.loading).toBe(false)
    expect(store.error).toBeNull()
    expect(store.nextCursor).toBeNull()
    expect(store.hasMore).toBe(false)
  })

  it('fetchNotifications loads notifications', async () => {
    const mockData = [
      {
        id: 'n1',
        accountId: 'acc-1',
        type: 'mention',
        title: 'You were mentioned',
        body: null,
        referenceType: null,
        referenceId: null,
        projectId: null,
        isRead: false,
        readAt: null,
        deliveredChannels: ['in_app'],
        createdAt: '2025-01-01T00:00:00Z',
      },
    ]
    vi.mocked(client.GET).mockResolvedValue({
      data: { data: mockData, nextCursor: 'cursor-abc' },
    } as never)

    const store = useNotificationStore()
    await store.fetchNotifications()

    expect(store.notifications).toEqual(mockData)
    expect(store.nextCursor).toBe('cursor-abc')
    expect(store.hasMore).toBe(true)
    expect(store.loading).toBe(false)
  })

  it('fetchNotifications appends when cursor provided', async () => {
    const store = useNotificationStore()
    store.notifications = [
      {
        id: 'n0',
        accountId: 'acc-1',
        type: 'system',
        title: 'Existing',
        isRead: true,
        deliveredChannels: [],
        createdAt: '2025-01-01T00:00:00Z',
      },
    ]

    const newItems = [
      {
        id: 'n1',
        accountId: 'acc-1',
        type: 'mention',
        title: 'New',
        isRead: false,
        deliveredChannels: [],
        createdAt: '2025-01-02T00:00:00Z',
      },
    ]
    vi.mocked(client.GET).mockResolvedValue({
      data: { data: newItems, nextCursor: null },
    } as never)

    await store.fetchNotifications(false, 'cursor-1')

    expect(store.notifications).toHaveLength(2)
    expect(store.nextCursor).toBeNull()
    expect(store.hasMore).toBe(false)
  })

  it('fetchNotifications handles error', async () => {
    vi.mocked(client.GET).mockRejectedValue(new Error('Network error'))

    const store = useNotificationStore()
    await store.fetchNotifications()

    expect(store.error).toBe('Failed to load notifications.')
    expect(store.notifications).toEqual([])
  })

  it('fetchUnreadCount updates count', async () => {
    vi.mocked(client.GET).mockResolvedValue({ data: { count: 5 } } as never)

    const store = useNotificationStore()
    await store.fetchUnreadCount()

    expect(store.unreadCount).toBe(5)
  })

  it('fetchUnreadCount silently fails', async () => {
    vi.mocked(client.GET).mockRejectedValue(new Error('fail'))

    const store = useNotificationStore()
    store.unreadCount = 3
    await store.fetchUnreadCount()

    // count remains unchanged on silent fail
    expect(store.unreadCount).toBe(3)
  })

  it('markAsRead updates notification and decrements count', async () => {
    vi.mocked(client.PUT).mockResolvedValue({} as never)

    const store = useNotificationStore()
    store.unreadCount = 2
    store.notifications = [
      {
        id: 'n1',
        accountId: 'acc-1',
        type: 'mention',
        title: 'Test',
        isRead: false,
        readAt: null,
        deliveredChannels: [],
        createdAt: '2025-01-01T00:00:00Z',
      },
    ]

    const result = await store.markAsRead('n1')

    expect(result).toBe(true)
    expect(store.notifications[0]?.isRead).toBe(true)
    expect(store.notifications[0]?.readAt).toBeTruthy()
    expect(store.unreadCount).toBe(1)
  })

  it('markAsRead handles error', async () => {
    vi.mocked(client.PUT).mockRejectedValue(new Error('fail'))

    const store = useNotificationStore()
    const result = await store.markAsRead('n1')

    expect(result).toBe(false)
    expect(store.error).toBe('Failed to mark notification as read.')
  })

  it('markAllAsRead updates all notifications', async () => {
    vi.mocked(client.PUT).mockResolvedValue({} as never)

    const store = useNotificationStore()
    store.unreadCount = 3
    store.notifications = [
      {
        id: 'n1',
        accountId: 'acc-1',
        type: 'mention',
        title: 'A',
        isRead: false,
        readAt: null,
        deliveredChannels: [],
        createdAt: '2025-01-01T00:00:00Z',
      },
      {
        id: 'n2',
        accountId: 'acc-1',
        type: 'system',
        title: 'B',
        isRead: true,
        readAt: '2025-01-01T00:00:00Z',
        deliveredChannels: [],
        createdAt: '2025-01-01T00:00:00Z',
      },
    ]

    const result = await store.markAllAsRead()

    expect(result).toBe(true)
    expect(store.notifications[0]?.isRead).toBe(true)
    expect(store.notifications[1]?.isRead).toBe(true)
    expect(store.unreadCount).toBe(0)
  })

  it('subscribeWebPush calls API', async () => {
    vi.mocked(client.POST).mockResolvedValue({} as never)

    const store = useNotificationStore()
    const result = await store.subscribeWebPush(
      'https://push.example.com/sub',
      'p256dh-key',
      'auth-key',
      'My Device',
    )

    expect(result).toBe(true)
    expect(client.POST).toHaveBeenCalled()
  })

  it('unsubscribeWebPush calls API', async () => {
    vi.mocked(client.DELETE).mockResolvedValue({} as never)

    const store = useNotificationStore()
    const result = await store.unsubscribeWebPush('https://push.example.com/sub')

    expect(result).toBe(true)
    expect(client.DELETE).toHaveBeenCalled()
  })

  it('addNotification prepends and increments unread count', () => {
    const store = useNotificationStore()
    store.notifications = [
      {
        id: 'n0',
        accountId: 'acc-1',
        type: 'system',
        title: 'Old',
        isRead: true,
        deliveredChannels: [],
        createdAt: '2025-01-01T00:00:00Z',
      },
    ]
    store.unreadCount = 0

    store.addNotification({
      id: 'n1',
      accountId: 'acc-1',
      type: 'mention',
      title: 'New notification',
      isRead: false,
      deliveredChannels: ['in_app'],
      createdAt: '2025-01-02T00:00:00Z',
    })

    expect(store.notifications).toHaveLength(2)
    expect(store.notifications[0]?.id).toBe('n1')
    expect(store.unreadCount).toBe(1)
  })

  it('addNotification does not increment count for read notification', () => {
    const store = useNotificationStore()
    store.unreadCount = 0

    store.addNotification({
      id: 'n1',
      accountId: 'acc-1',
      type: 'system',
      title: 'Already read',
      isRead: true,
      deliveredChannels: [],
      createdAt: '2025-01-01T00:00:00Z',
    })

    expect(store.unreadCount).toBe(0)
  })

  it('$reset clears all state', () => {
    const store = useNotificationStore()
    store.notifications = [
      {
        id: 'n1',
        accountId: 'acc-1',
        type: 'system',
        title: 'Test',
        isRead: false,
        deliveredChannels: [],
        createdAt: '2025-01-01T00:00:00Z',
      },
    ]
    store.unreadCount = 5
    store.loading = true
    store.error = 'Some error'
    store.nextCursor = 'cursor-1'

    store.$reset()

    expect(store.notifications).toEqual([])
    expect(store.unreadCount).toBe(0)
    expect(store.loading).toBe(false)
    expect(store.error).toBeNull()
    expect(store.nextCursor).toBeNull()
  })
})

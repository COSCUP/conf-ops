import { ref, computed } from 'vue'
import { defineStore } from 'pinia'
import client from '@/api/client'

export interface NotificationItem {
  id: string
  accountId: string
  type: string
  title: string
  body?: string | null
  referenceType?: string | null
  referenceId?: string | null
  projectId?: string | null
  isRead: boolean
  readAt?: string | null
  deliveredChannels: string[]
  createdAt: string
}

export interface NotificationPreferences {
  channels: {
    email: ChannelPreference
    webPush: ChannelPreference
    inApp: ChannelPreference
  }
}

export interface ChannelPreference {
  enabled: boolean
  categories: Record<string, boolean>
}

export const useNotificationStore = defineStore('notification', () => {
  const notifications = ref<NotificationItem[]>([])
  const unreadCount = ref(0)
  const loading = ref(false)
  const error = ref<string | null>(null)
  const nextCursor = ref<string | null>(null)

  const hasMore = computed(() => nextCursor.value !== null)

  async function fetchNotifications(unreadOnly = false, cursor?: string) {
    loading.value = true
    error.value = null
    try {
      const query: Record<string, string> = { limit: '50' }
      if (cursor) query.cursor = cursor
      if (unreadOnly) query.unreadOnly = 'true'

      const { data } = await client.GET('/api/v1/notifications' as never, {
        params: { query },
      } as never)
      const result = data as
        | { data: NotificationItem[]; nextCursor?: string | null }
        | undefined
      if (result) {
        if (cursor) {
          notifications.value = [...notifications.value, ...result.data]
        } else {
          notifications.value = result.data
        }
        nextCursor.value = result.nextCursor ?? null
      }
    } catch {
      error.value = 'Failed to load notifications.'
    } finally {
      loading.value = false
    }
  }

  async function fetchUnreadCount() {
    try {
      const { data } = await client.GET(
        '/api/v1/notifications/unread-count' as never,
      )
      const result = data as { count: number } | undefined
      if (result) {
        unreadCount.value = result.count
      }
    } catch {
      // Silently fail for badge count
    }
  }

  async function markAsRead(notificationId: string): Promise<boolean> {
    try {
      await client.PUT(
        '/api/v1/notifications/{notificationId}/read' as never,
        { params: { path: { notificationId } } } as never,
      )
      const notification = notifications.value.find(
        (n) => n.id === notificationId,
      )
      if (notification) {
        notification.isRead = true
        notification.readAt = new Date().toISOString()
      }
      if (unreadCount.value > 0) {
        unreadCount.value -= 1
      }
      return true
    } catch {
      error.value = 'Failed to mark notification as read.'
      return false
    }
  }

  async function markAllAsRead(): Promise<boolean> {
    try {
      await client.PUT('/api/v1/notifications/read-all' as never)
      for (const n of notifications.value) {
        if (!n.isRead) {
          n.isRead = true
          n.readAt = new Date().toISOString()
        }
      }
      unreadCount.value = 0
      return true
    } catch {
      error.value = 'Failed to mark all as read.'
      return false
    }
  }

  async function subscribeWebPush(
    endpoint: string,
    p256dh: string,
    auth: string,
    deviceName?: string,
  ): Promise<boolean> {
    try {
      await client.POST(
        '/api/v1/notifications/web-push/subscribe' as never,
        {
          body: {
            subscription: { endpoint, keys: { p256dh, auth } },
            deviceName,
          },
        } as never,
      )
      return true
    } catch {
      error.value = 'Failed to subscribe to Web Push.'
      return false
    }
  }

  async function unsubscribeWebPush(endpoint: string): Promise<boolean> {
    try {
      await client.DELETE(
        '/api/v1/notifications/web-push/subscriptions/{endpoint}' as never,
        { params: { path: { endpoint } } } as never,
      )
      return true
    } catch {
      error.value = 'Failed to unsubscribe from Web Push.'
      return false
    }
  }

  function addNotification(notification: NotificationItem) {
    notifications.value = [notification, ...notifications.value]
    if (!notification.isRead) {
      unreadCount.value += 1
    }
  }

  function $reset() {
    notifications.value = []
    unreadCount.value = 0
    loading.value = false
    error.value = null
    nextCursor.value = null
  }

  return {
    notifications,
    unreadCount,
    loading,
    error,
    nextCursor,
    hasMore,
    fetchNotifications,
    fetchUnreadCount,
    markAsRead,
    markAllAsRead,
    subscribeWebPush,
    unsubscribeWebPush,
    addNotification,
    $reset,
  }
})

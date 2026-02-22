<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useNotificationStore } from '@/stores/notification'

const store = useNotificationStore()
const unreadOnly = ref(false)

onMounted(() => {
  void store.fetchNotifications(unreadOnly.value)
})

async function toggleFilter() {
  unreadOnly.value = !unreadOnly.value
  await store.fetchNotifications(unreadOnly.value)
}

async function loadMore() {
  if (store.nextCursor) {
    await store.fetchNotifications(unreadOnly.value, store.nextCursor)
  }
}

async function handleMarkAsRead(notificationId: string) {
  await store.markAsRead(notificationId)
}

async function handleMarkAllAsRead() {
  await store.markAllAsRead()
}

function formatTime(iso: string): string {
  const d = new Date(iso)
  return d.toLocaleString(undefined, {
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  })
}

function getTypeClass(type: string): string {
  switch (type) {
    case 'mention':
      return 'type-mention'
    case 'task_update':
      return 'type-task'
    case 'reminder':
      return 'type-reminder'
    case 'system':
      return 'type-system'
    default:
      return 'type-default'
  }
}
</script>

<template>
  <div class="notification-panel">
    <div class="panel-header">
      <h3 class="panel-title">Notifications</h3>
      <div class="panel-actions">
        <button
          type="button"
          class="filter-btn"
          :class="{ active: unreadOnly }"
          @click="toggleFilter"
        >
          {{ unreadOnly ? 'Show all' : 'Unread only' }}
        </button>
        <button
          v-if="store.unreadCount > 0"
          type="button"
          class="mark-all-btn"
          @click="handleMarkAllAsRead"
        >
          Mark all read
        </button>
      </div>
    </div>

    <div v-if="store.error" class="error-banner">{{ store.error }}</div>

    <div v-if="store.loading && store.notifications.length === 0" class="loading">
      Loading notifications...
    </div>

    <div v-else-if="store.notifications.length === 0" class="empty">
      No notifications yet.
    </div>

    <div v-else class="notification-list">
      <div
        v-for="notification in store.notifications"
        :key="notification.id"
        :class="['notification-item', { unread: !notification.isRead }]"
      >
        <div class="notification-content">
          <div class="notification-meta">
            <span :class="['type-badge', getTypeClass(notification.type)]">
              {{ notification.type }}
            </span>
            <span class="notification-time">{{ formatTime(notification.createdAt) }}</span>
          </div>
          <p class="notification-title">{{ notification.title }}</p>
          <p v-if="notification.body" class="notification-body">{{ notification.body }}</p>
        </div>
        <button
          v-if="!notification.isRead"
          type="button"
          class="read-btn"
          title="Mark as read"
          @click="handleMarkAsRead(notification.id)"
        >
          <span class="unread-dot" />
        </button>
      </div>

      <button
        v-if="store.hasMore"
        type="button"
        class="load-more-btn"
        :disabled="store.loading"
        @click="loadMore"
      >
        {{ store.loading ? 'Loading...' : 'Load more' }}
      </button>
    </div>
  </div>
</template>

<style scoped>
.notification-panel {
  width: 22rem;
  max-height: 28rem;
  display: flex;
  flex-direction: column;
  background: #ffffff;
  border: 1px solid #e5e7eb;
  border-radius: 0.5rem;
  box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.1);
  overflow: hidden;
}

.panel-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0.75rem 1rem;
  border-bottom: 1px solid #e5e7eb;
}

.panel-title {
  margin: 0;
  font-size: 0.9375rem;
  font-weight: 600;
  color: #111827;
}

.panel-actions {
  display: flex;
  gap: 0.5rem;
}

.filter-btn,
.mark-all-btn {
  font-size: 0.75rem;
  color: #2563eb;
  background: none;
  border: none;
  cursor: pointer;
  padding: 0;
  text-decoration: underline;
}

.filter-btn.active {
  font-weight: 600;
}

.error-banner {
  background: #fef2f2;
  color: #dc2626;
  padding: 0.5rem 1rem;
  font-size: 0.8125rem;
}

.loading,
.empty {
  padding: 2rem 1rem;
  text-align: center;
  color: #6b7280;
  font-size: 0.875rem;
}

.notification-list {
  overflow-y: auto;
  flex: 1;
}

.notification-item {
  display: flex;
  align-items: flex-start;
  gap: 0.5rem;
  padding: 0.75rem 1rem;
  border-bottom: 1px solid #f3f4f6;
  transition: background-color 0.15s;
}

.notification-item:hover {
  background: #f9fafb;
}

.notification-item.unread {
  background: #eff6ff;
}

.notification-content {
  flex: 1;
  min-width: 0;
}

.notification-meta {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 0.25rem;
}

.type-badge {
  font-size: 0.6875rem;
  font-weight: 600;
  padding: 0.0625rem 0.375rem;
  border-radius: 0.25rem;
}

.type-mention {
  background: #dbeafe;
  color: #1e40af;
}

.type-task {
  background: #d1fae5;
  color: #065f46;
}

.type-reminder {
  background: #fef3c7;
  color: #92400e;
}

.type-system {
  background: #f3f4f6;
  color: #374151;
}

.type-default {
  background: #f3f4f6;
  color: #6b7280;
}

.notification-time {
  font-size: 0.6875rem;
  color: #9ca3af;
}

.notification-title {
  margin: 0;
  font-size: 0.8125rem;
  font-weight: 500;
  color: #1f2937;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.notification-body {
  margin: 0.125rem 0 0;
  font-size: 0.75rem;
  color: #6b7280;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.read-btn {
  background: none;
  border: none;
  cursor: pointer;
  padding: 0.25rem;
  flex-shrink: 0;
}

.unread-dot {
  display: block;
  width: 0.5rem;
  height: 0.5rem;
  background: #2563eb;
  border-radius: 9999px;
}

.load-more-btn {
  width: 100%;
  padding: 0.5rem;
  font-size: 0.8125rem;
  color: #2563eb;
  background: none;
  border: none;
  border-top: 1px solid #e5e7eb;
  cursor: pointer;
}

.load-more-btn:hover {
  background: #f9fafb;
}

.load-more-btn:disabled {
  color: #9ca3af;
  cursor: not-allowed;
}
</style>

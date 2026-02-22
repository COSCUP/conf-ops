<script setup lang="ts">
import { onMounted, onUnmounted } from 'vue'
import { useNotificationStore } from '@/stores/notification'

const store = useNotificationStore()

const emit = defineEmits<{
  toggle: []
}>()

let pollTimer: ReturnType<typeof setInterval> | null = null

onMounted(() => {
  void store.fetchUnreadCount()
  pollTimer = setInterval(() => {
    void store.fetchUnreadCount()
  }, 60_000)
})

onUnmounted(() => {
  if (pollTimer) {
    clearInterval(pollTimer)
    pollTimer = null
  }
})

function handleClick() {
  emit('toggle')
}
</script>

<template>
  <button
    type="button"
    class="notification-bell"
    aria-label="Notifications"
    @click="handleClick"
  >
    <svg
      class="bell-icon"
      xmlns="http://www.w3.org/2000/svg"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width="2"
      stroke-linecap="round"
      stroke-linejoin="round"
    >
      <path d="M18 8A6 6 0 0 0 6 8c0 7-3 9-3 9h18s-3-2-3-9" />
      <path d="M13.73 21a2 2 0 0 1-3.46 0" />
    </svg>
    <span
      v-if="store.unreadCount > 0"
      class="badge"
    >
      {{ store.unreadCount > 99 ? '99+' : store.unreadCount }}
    </span>
  </button>
</template>

<style scoped>
.notification-bell {
  position: relative;
  background: none;
  border: none;
  cursor: pointer;
  padding: 0.5rem;
  color: #374151;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border-radius: 0.375rem;
  transition: background-color 0.15s;
}

.notification-bell:hover {
  background-color: #f3f4f6;
}

.bell-icon {
  width: 1.25rem;
  height: 1.25rem;
}

.badge {
  position: absolute;
  top: 0;
  right: 0;
  min-width: 1.125rem;
  height: 1.125rem;
  padding: 0 0.25rem;
  font-size: 0.625rem;
  font-weight: 700;
  color: #ffffff;
  background: #ef4444;
  border-radius: 9999px;
  display: flex;
  align-items: center;
  justify-content: center;
  line-height: 1;
}
</style>

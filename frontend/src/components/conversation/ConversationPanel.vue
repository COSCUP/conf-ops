<script setup lang="ts">
import { ref, watch, nextTick } from 'vue'
import { useConversation } from '@/composables/useConversation'
import MessageItem from './MessageItem.vue'
import MessageInput from './MessageInput.vue'
import AwarenessBar from './AwarenessBar.vue'
import BaseButton from '@/components/base/BaseButton.vue'

const props = defineProps<{
  projectId: string
  taskId: string
}>()

const {
  messages,
  loading,
  error,
  hasMore,
  lastReadMessageId,
  connected,
  reconnecting,
  awarenessEntries,
  sendMessage,
  loadMore,
  setTyping,
} = useConversation({ projectId: props.projectId, taskId: props.taskId })

const messageListRef = ref<HTMLElement | null>(null)

function isUnread(messageId: string): boolean {
  if (!lastReadMessageId) return false
  return messageId > (lastReadMessageId as string)
}

async function handleSend(text: string) {
  await sendMessage(text)
  await nextTick()
  scrollToBottom()
}

const members: { id: string; displayName: string }[] = []

function scrollToBottom() {
  if (messageListRef.value) {
    messageListRef.value.scrollTop = messageListRef.value.scrollHeight
  }
}

watch(
  () => messages.length,
  () => {
    void nextTick(() => scrollToBottom())
  },
)
</script>

<template>
  <div class="conversation-panel">
    <AwarenessBar :entries="awarenessEntries" />

    <div class="connection-status">
      <span v-if="connected" class="status-connected">Connected</span>
      <span v-else-if="reconnecting" class="status-reconnecting">Reconnecting...</span>
      <span v-else class="status-disconnected">Disconnected</span>
    </div>

    <p v-if="error" class="error-message">{{ error }}</p>

    <div ref="messageListRef" class="message-list">
      <BaseButton
        v-if="hasMore"
        variant="secondary"
        :disabled="loading"
        @click="loadMore"
      >
        Load more
      </BaseButton>

      <template v-if="messages.length > 0">
        <MessageItem
          v-for="msg in [...messages].reverse()"
          :key="msg.id"
          :message="msg"
          :is-unread="isUnread(msg.id)"
        />
      </template>
      <p v-else-if="!loading" class="empty-text">No messages yet.</p>
    </div>

    <MessageInput :disabled="!connected" :members="members" @send="handleSend" @typing="setTyping" />
  </div>
</template>

<style scoped>
.conversation-panel {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.connection-status {
  font-size: 0.625rem;
  text-align: right;
}

.status-connected {
  color: #059669;
}

.status-reconnecting {
  color: #d97706;
}

.status-disconnected {
  color: #dc2626;
}

.message-list {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  max-height: 400px;
  overflow-y: auto;
  padding: 0.5rem;
  border: 1px solid #e5e7eb;
  border-radius: 0.375rem;
  background: #ffffff;
}

.error-message {
  color: #ef4444;
  padding: 0.5rem 0.75rem;
  background: #fef2f2;
  border-radius: 0.375rem;
  font-size: 0.875rem;
}

.empty-text {
  color: #9ca3af;
  font-size: 0.875rem;
  text-align: center;
  padding: 2rem 0;
}
</style>

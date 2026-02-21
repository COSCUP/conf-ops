<script setup lang="ts">
import type { MessageResponse } from '@/stores/conversation'

defineProps<{
  message: MessageResponse
  isUnread: boolean
}>()

function formatTime(iso: string): string {
  const d = new Date(iso)
  return d.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' })
}

function formatDate(iso: string): string {
  const d = new Date(iso)
  return d.toLocaleDateString(undefined, { month: 'short', day: 'numeric', year: 'numeric' })
}

function getMessageText(
  content: Record<string, unknown>,
  sourceType: string,
): string {
  switch (sourceType) {
    case 'system': {
      const event = typeof content.event === 'string' ? content.event : 'system'
      const details =
        content.details != null ? JSON.stringify(content.details) : ''
      return `[${event}] ${details}`
    }
    case 'tool_execution': {
      const toolName =
        typeof content.toolName === 'string' ? content.toolName : 'unknown'
      const status =
        typeof content.status === 'string' ? content.status : ''
      return `[Tool: ${toolName}] ${status}`
    }
    case 'ai_suggestion':
    case 'member':
    case 'email_inbound':
    default:
      return typeof content.text === 'string' ? content.text : ''
  }
}
</script>

<template>
  <div :class="['message-item', `message-${message.sourceType}`, { 'message-unread': isUnread }]">
    <div class="message-header">
      <span class="message-source">
        {{ message.sourceType === 'system' ? 'System' : (message.sourceId ?? 'Unknown') }}
      </span>
      <span class="message-time" :title="formatDate(message.createdAt)">
        {{ formatTime(message.createdAt) }}
      </span>
    </div>
    <div class="message-body">
      {{ getMessageText(message.content, message.sourceType) }}
    </div>
  </div>
</template>

<style scoped>
.message-item {
  padding: 0.5rem 0.75rem;
  border-radius: 0.375rem;
  background: #f9fafb;
}

.message-item.message-unread {
  border-left: 3px solid #3b82f6;
}

.message-system {
  background: #fffbeb;
  font-style: italic;
}

.message-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 0.25rem;
}

.message-source {
  font-size: 0.75rem;
  font-weight: 600;
  color: #374151;
}

.message-time {
  font-size: 0.625rem;
  color: #9ca3af;
}

.message-body {
  font-size: 0.875rem;
  color: #1f2937;
  white-space: pre-wrap;
  word-break: break-word;
}
</style>

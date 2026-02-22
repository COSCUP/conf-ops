<script setup lang="ts">
import type { components } from '@/api/schema'

type EmailMessageResponse = components['schemas']['EmailMessageResponse']

defineProps<{
  message: EmailMessageResponse
}>()

function formatDate(iso: string): string {
  return new Date(iso).toLocaleString()
}
</script>

<template>
  <div class="email-message-item" :class="[`direction-${message.direction}`]">
    <div class="message-header">
      <span class="from-address">{{ message.fromAddress }}</span>
      <span class="direction-badge">{{ message.direction }}</span>
      <span class="timestamp">{{ formatDate(message.createdAt) }}</span>
    </div>
    <div class="message-subject">{{ message.subject }}</div>
    <div class="message-meta">
      <span v-if="message.sendStatus !== 'sent'" class="send-status" :class="message.sendStatus">
        {{ message.sendStatus }}
      </span>
    </div>
  </div>
</template>

<style scoped>
.email-message-item {
  padding: 0.75rem;
  border: 1px solid #e5e7eb;
  border-radius: 0.375rem;
  background: #ffffff;
}

.direction-inbound {
  border-left: 3px solid #3b82f6;
}

.direction-outbound {
  border-left: 3px solid #10b981;
}

.message-header {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  font-size: 0.75rem;
}

.from-address {
  font-weight: 600;
  color: #1f2937;
}

.direction-badge {
  font-size: 0.625rem;
  padding: 0.125rem 0.375rem;
  border-radius: 9999px;
  background: #f3f4f6;
  color: #6b7280;
  text-transform: uppercase;
}

.timestamp {
  color: #9ca3af;
  margin-left: auto;
}

.message-subject {
  font-size: 0.875rem;
  color: #374151;
  margin-top: 0.25rem;
}

.message-meta {
  margin-top: 0.25rem;
}

.send-status {
  font-size: 0.625rem;
  padding: 0.125rem 0.375rem;
  border-radius: 9999px;
  font-weight: 500;
}

.send-status.failed {
  background: #fef2f2;
  color: #dc2626;
}

.send-status.pending {
  background: #fef3c7;
  color: #92400e;
}
</style>

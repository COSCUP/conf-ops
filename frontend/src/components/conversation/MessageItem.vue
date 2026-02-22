<script setup lang="ts">
import type { MessageResponse } from '@/stores/conversation'
import type { SuggestionDecision } from '@/stores/suggestion'
import SuggestionCard from '@/components/ai/SuggestionCard.vue'
import type { SuggestionGroup, Suggestion } from '@/stores/suggestion'

const props = defineProps<{
  message: MessageResponse
  isUnread: boolean
  projectId: string
  taskId: string
}>()

const emit = defineEmits<{
  suggestionDecide: [
    payload: {
      groupId: string
      suggestionId: string
      decision: SuggestionDecision
      messageId: string
      modifiedParameters?: unknown
    },
  ]
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
    case 'member':
    case 'email_inbound':
    default:
      return typeof content.text === 'string' ? content.text : ''
  }
}

function getEmailFrom(content: Record<string, unknown>): string {
  return typeof content.from === 'string' ? content.from : ''
}

function getEmailSubject(content: Record<string, unknown>): string {
  return typeof content.subject === 'string' ? content.subject : ''
}

function parseSuggestionGroup(content: Record<string, unknown>): SuggestionGroup | null {
  try {
    const raw = content.suggestionGroup
    if (
      raw != null &&
      typeof raw === 'object' &&
      !Array.isArray(raw) &&
      'id' in raw &&
      'suggestions' in raw &&
      'trigger' in raw &&
      'createdAt' in raw
    ) {
      return raw as SuggestionGroup
    }
    return null
  } catch {
    return null
  }
}

function parseSuggestions(content: Record<string, unknown>): Suggestion[] {
  const group = parseSuggestionGroup(content)
  return group?.suggestions ?? []
}

function getGroupId(content: Record<string, unknown>): string {
  const group = parseSuggestionGroup(content)
  return group?.id ?? ''
}
</script>

<template>
  <div :class="['message-item', `message-${message.sourceType}`, { 'message-unread': isUnread }]">
    <div class="message-header">
      <span class="message-source">
        {{ message.sourceType === 'system' ? 'System' : (message.sourceType === 'ai_suggestion' ? 'AI' : (message.sourceId ?? 'Unknown')) }}
      </span>
      <span class="message-time" :title="formatDate(message.createdAt)">
        {{ formatTime(message.createdAt) }}
      </span>
    </div>

    <div v-if="message.sourceType === 'email_inbound'" class="email-inbound-display">
      <span class="email-icon">&#9993;</span>
      <div class="email-from">{{ getEmailFrom(message.content) }}</div>
      <div class="email-subject">{{ getEmailSubject(message.content) }}</div>
      <div class="message-body">
        {{ getMessageText(message.content, message.sourceType) }}
      </div>
    </div>

    <div v-else-if="message.sourceType === 'ai_suggestion'" class="ai-suggestion-display">
      <div v-if="parseSuggestions(message.content).length > 0" class="suggestion-list">
        <SuggestionCard
          v-for="suggestion in parseSuggestions(message.content)"
          :key="suggestion.id"
          :suggestion="suggestion"
          :group-id="getGroupId(message.content)"
          :message-id="message.id"
          :project-id="props.projectId"
          :task-id="props.taskId"
          @decide="emit('suggestionDecide', $event)"
        />
      </div>
      <div v-else class="message-body">
        {{ typeof message.content.text === 'string' ? message.content.text : '[AI Suggestion]' }}
      </div>
    </div>

    <div v-else class="message-body">
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

.message-ai_suggestion {
  background: #f0fdf4;
  border-left: 3px solid #059669;
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

.email-inbound-display {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}

.email-icon {
  font-size: 1rem;
  color: #6b7280;
}

.email-from {
  font-size: 0.75rem;
  font-weight: 600;
  color: #374151;
}

.email-subject {
  font-size: 0.8125rem;
  font-weight: 500;
  color: #4b5563;
}

.ai-suggestion-display {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.suggestion-list {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}
</style>

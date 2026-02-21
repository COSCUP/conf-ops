import { ref } from 'vue'
import { defineStore } from 'pinia'
import client from '@/api/client'

// Local types until OpenAPI schema is regenerated
export interface MessageResponse {
  id: string
  taskId: string
  sourceType: string
  sourceId: string | null
  content: Record<string, unknown>
  attachments: Record<string, unknown> | null
  actionResult: Record<string, unknown> | null
  lastSeenMessageId: string | null
  createdAt: string
}

interface PaginationInfo {
  hasMore: boolean
  nextCursor: string | null
}

interface ConversationResponse {
  messages: MessageResponse[]
  pagination: PaginationInfo
}

interface LastSeenResponse {
  lastReadMessageId: string | null
}

export const useConversationStore = defineStore('conversation', () => {
  const messages = ref<MessageResponse[]>([])
  const loading = ref(false)
  const error = ref('')
  const hasMore = ref(false)
  const nextCursor = ref<string | null>(null)
  const lastReadMessageId = ref<string | null>(null)

  async function fetchMessages(projectId: string, taskId: string, cursor?: string) {
    loading.value = true
    error.value = ''
    try {
      const query: Record<string, string> = { limit: '50' }
      if (cursor) query.cursor = cursor

      const { data } = await client.GET(
        '/api/v1/projects/{projectId}/tasks/{taskId}/conversation' as never,
        { params: { path: { projectId, taskId }, query } } as never,
      )
      const result = data as ConversationResponse | undefined
      if (result) {
        if (cursor) {
          messages.value = [...messages.value, ...result.messages]
        } else {
          messages.value = result.messages
        }
        hasMore.value = result.pagination.hasMore
        nextCursor.value = result.pagination.nextCursor
      }
    } catch {
      error.value = 'Failed to load messages.'
    } finally {
      loading.value = false
    }
  }

  async function sendMessage(
    projectId: string,
    taskId: string,
    content: Record<string, unknown>,
    lastSeenMessageId?: string,
  ) {
    loading.value = true
    error.value = ''
    try {
      const body: Record<string, unknown> = { content }
      if (lastSeenMessageId) body.lastSeenMessageId = lastSeenMessageId

      const { data } = await client.POST(
        '/api/v1/projects/{projectId}/tasks/{taskId}/conversation/messages' as never,
        { params: { path: { projectId, taskId } }, body } as never,
      )
      const result = data as MessageResponse | undefined
      if (result) {
        messages.value = [result, ...messages.value]
        return result
      }
      return null
    } catch {
      error.value = 'Failed to send message.'
      return null
    } finally {
      loading.value = false
    }
  }

  async function updateLastSeen(projectId: string, taskId: string, messageId: string) {
    try {
      await client.PUT(
        '/api/v1/projects/{projectId}/tasks/{taskId}/conversation/last-seen' as never,
        {
          params: { path: { projectId, taskId } },
          body: { messageId },
        } as never,
      )
      lastReadMessageId.value = messageId
    } catch {
      // Silently handle — not critical
    }
  }

  async function getLastSeen(projectId: string, taskId: string) {
    try {
      const { data } = await client.GET(
        '/api/v1/projects/{projectId}/tasks/{taskId}/conversation/last-seen' as never,
        { params: { path: { projectId, taskId } } } as never,
      )
      const result = data as LastSeenResponse | undefined
      if (result) {
        lastReadMessageId.value = result.lastReadMessageId
      }
    } catch {
      // Silently handle
    }
  }

  function $reset() {
    messages.value = []
    loading.value = false
    error.value = ''
    hasMore.value = false
    nextCursor.value = null
    lastReadMessageId.value = null
  }

  return {
    messages,
    loading,
    error,
    hasMore,
    nextCursor,
    lastReadMessageId,
    fetchMessages,
    sendMessage,
    updateLastSeen,
    getLastSeen,
    $reset,
  }
})

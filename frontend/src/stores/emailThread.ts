import { ref } from 'vue'
import { defineStore } from 'pinia'
import client from '@/api/client'
import type { components } from '@/api/schema'

type EmailThreadResponse = components['schemas']['EmailThreadResponse']
type EmailMessageResponse = components['schemas']['EmailMessageResponse']
type PaginationInfo = components['schemas']['PaginationInfo']

export const useEmailThreadStore = defineStore('emailThread', () => {
  const threads = ref<EmailThreadResponse[]>([])
  const currentMessages = ref<EmailMessageResponse[]>([])
  const loading = ref(false)
  const error = ref('')
  const pagination = ref<PaginationInfo>({ hasMore: false, nextCursor: null })

  async function fetchThreads(projectId: string, taskId: string, cursor?: string) {
    loading.value = true
    error.value = ''
    try {
      const query: Record<string, string> = { limit: '50' }
      if (cursor) query.cursor = cursor

      const { data } = await client.GET(
        '/api/v1/projects/{projectId}/tasks/{taskId}/email-threads',
        { params: { path: { projectId, taskId }, query } },
      )
      if (data) {
        if (cursor) {
          threads.value = [...threads.value, ...data.threads]
        } else {
          threads.value = data.threads
        }
        pagination.value = data.pagination
      }
    } catch {
      error.value = 'Failed to load email threads.'
    } finally {
      loading.value = false
    }
  }

  async function createThread(
    projectId: string,
    taskId: string,
    subject: string,
    participants: string[],
  ) {
    loading.value = true
    error.value = ''
    try {
      const { data } = await client.POST(
        '/api/v1/projects/{projectId}/tasks/{taskId}/email-threads',
        {
          params: { path: { projectId, taskId } },
          body: { subject, participants },
        },
      )
      if (data) {
        threads.value = [data, ...threads.value]
        return data
      }
      return null
    } catch {
      error.value = 'Failed to create email thread.'
      return null
    } finally {
      loading.value = false
    }
  }

  async function updateThread(
    projectId: string,
    taskId: string,
    threadId: string,
    body: { subject?: string; participants?: string[] },
  ) {
    loading.value = true
    error.value = ''
    try {
      const { data } = await client.PATCH(
        '/api/v1/projects/{projectId}/tasks/{taskId}/email-threads/{threadId}',
        {
          params: { path: { projectId, taskId, threadId } },
          body,
        },
      )
      if (data) {
        const idx = threads.value.findIndex((t) => t.id === threadId)
        if (idx >= 0) {
          threads.value[idx] = data
        }
        return data
      }
      return null
    } catch {
      error.value = 'Failed to update email thread.'
      return null
    } finally {
      loading.value = false
    }
  }

  async function deleteThread(projectId: string, taskId: string, threadId: string) {
    loading.value = true
    error.value = ''
    try {
      await client.DELETE(
        '/api/v1/projects/{projectId}/tasks/{taskId}/email-threads/{threadId}',
        { params: { path: { projectId, taskId, threadId } } },
      )
      threads.value = threads.value.filter((t) => t.id !== threadId)
    } catch {
      error.value = 'Failed to delete email thread.'
    } finally {
      loading.value = false
    }
  }

  async function fetchMessages(projectId: string, taskId: string, threadId: string) {
    loading.value = true
    error.value = ''
    try {
      const { data } = await client.GET(
        '/api/v1/projects/{projectId}/tasks/{taskId}/email-threads/{threadId}/messages',
        { params: { path: { projectId, taskId, threadId } } },
      )
      if (data) {
        currentMessages.value = data.messages
      }
    } catch {
      error.value = 'Failed to load email messages.'
    } finally {
      loading.value = false
    }
  }

  async function sendEmail(
    projectId: string,
    taskId: string,
    threadId: string,
    body: {
      toAddresses: string[]
      ccAddresses?: string[]
      subject?: string
      htmlBody: string
    },
  ) {
    loading.value = true
    error.value = ''
    try {
      const { data } = await client.POST(
        '/api/v1/projects/{projectId}/tasks/{taskId}/email-threads/{threadId}/messages',
        {
          params: { path: { projectId, taskId, threadId } },
          body,
        },
      )
      if (data) {
        currentMessages.value = [...currentMessages.value, data]
        return data
      }
      return null
    } catch {
      error.value = 'Failed to send email.'
      return null
    } finally {
      loading.value = false
    }
  }

  function $reset() {
    threads.value = []
    currentMessages.value = []
    loading.value = false
    error.value = ''
    pagination.value = { hasMore: false, nextCursor: null }
  }

  return {
    threads,
    currentMessages,
    loading,
    error,
    pagination,
    fetchThreads,
    createThread,
    updateThread,
    deleteThread,
    fetchMessages,
    sendEmail,
    $reset,
  }
})

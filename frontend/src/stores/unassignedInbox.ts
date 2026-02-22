import { ref } from 'vue'
import { defineStore } from 'pinia'
import client from '@/api/client'
import type { components } from '@/api/schema'

type UnassignedEmailResponse = components['schemas']['UnassignedEmailResponse']
type PaginationInfo = components['schemas']['PaginationInfo']

export const useUnassignedInboxStore = defineStore('unassignedInbox', () => {
  const emails = ref<UnassignedEmailResponse[]>([])
  const loading = ref(false)
  const error = ref('')
  const pagination = ref<PaginationInfo>({ hasMore: false, nextCursor: null })

  async function fetchEmails(projectId: string, cursor?: string) {
    loading.value = true
    error.value = ''
    try {
      const query: Record<string, string> = { limit: '50' }
      if (cursor) query.cursor = cursor

      const { data } = await client.GET(
        '/api/v1/projects/{projectId}/unassigned-inbox',
        { params: { path: { projectId }, query } },
      )
      if (data) {
        if (cursor) {
          emails.value = [...emails.value, ...data.emails]
        } else {
          emails.value = data.emails
        }
        pagination.value = data.pagination
      }
    } catch {
      error.value = 'Failed to load unassigned emails.'
    } finally {
      loading.value = false
    }
  }

  async function assignEmail(projectId: string, emailId: string, taskId: string) {
    loading.value = true
    error.value = ''
    try {
      const { data } = await client.POST(
        '/api/v1/projects/{projectId}/unassigned-inbox/{emailId}/assign',
        {
          params: { path: { projectId, emailId } },
          body: { taskId },
        },
      )
      if (data) {
        emails.value = emails.value.filter((e) => e.id !== emailId)
        return data
      }
      return null
    } catch {
      error.value = 'Failed to assign email.'
      return null
    } finally {
      loading.value = false
    }
  }

  function $reset() {
    emails.value = []
    loading.value = false
    error.value = ''
    pagination.value = { hasMore: false, nextCursor: null }
  }

  return {
    emails,
    loading,
    error,
    pagination,
    fetchEmails,
    assignEmail,
    $reset,
  }
})

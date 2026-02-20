import { ref } from 'vue'
import { defineStore } from 'pinia'
import client from '@/api/client'
import type { components } from '@/api/schema'

type MemberTagListItemResponse = components['schemas']['MemberTagListItemResponse']
type MemberTagDetailResponse = components['schemas']['MemberTagDetailResponse']

export const useMemberTagStore = defineStore('memberTag', () => {
  const tags = ref<MemberTagListItemResponse[]>([])
  const currentTagDetail = ref<MemberTagDetailResponse | null>(null)
  const loading = ref(false)
  const error = ref('')

  async function fetchTags(projectId: string) {
    loading.value = true
    error.value = ''
    try {
      const { data } = await client.GET('/api/v1/projects/{projectId}/member-tags', {
        params: { path: { projectId } },
      })
      if (data) {
        tags.value = data.tags
      }
    } catch {
      error.value = 'Failed to load tags.'
    } finally {
      loading.value = false
    }
  }

  async function createTag(projectId: string, name: string, description?: string) {
    loading.value = true
    error.value = ''
    try {
      const { data } = await client.POST('/api/v1/projects/{projectId}/member-tags', {
        params: { path: { projectId } },
        body: { name, description: description ?? null },
      })
      if (data) {
        await fetchTags(projectId)
        return data
      }
      return null
    } catch {
      error.value = 'Failed to create tag.'
      return null
    } finally {
      loading.value = false
    }
  }

  async function getTagDetail(projectId: string, tagId: string) {
    loading.value = true
    error.value = ''
    try {
      const { data } = await client.GET('/api/v1/projects/{projectId}/member-tags/{tagId}', {
        params: { path: { projectId, tagId } },
      })
      if (data) {
        currentTagDetail.value = data
      }
    } catch {
      error.value = 'Failed to load tag detail.'
    } finally {
      loading.value = false
    }
  }

  async function updateTag(
    projectId: string,
    tagId: string,
    updates: { name?: string; description?: string | null },
  ) {
    loading.value = true
    error.value = ''
    try {
      const { data } = await client.PUT('/api/v1/projects/{projectId}/member-tags/{tagId}', {
        params: { path: { projectId, tagId } },
        body: updates,
      })
      if (data) {
        await fetchTags(projectId)
        return data
      }
      return null
    } catch {
      error.value = 'Failed to update tag.'
      return null
    } finally {
      loading.value = false
    }
  }

  async function deleteTag(projectId: string, tagId: string) {
    loading.value = true
    error.value = ''
    try {
      await client.DELETE('/api/v1/projects/{projectId}/member-tags/{tagId}', {
        params: { path: { projectId, tagId } },
      })
      await fetchTags(projectId)
    } catch {
      error.value = 'Failed to delete tag.'
    } finally {
      loading.value = false
    }
  }

  async function assignTag(
    projectId: string,
    tagId: string,
    memberId?: string,
    contactId?: string,
  ) {
    loading.value = true
    error.value = ''
    try {
      const { data } = await client.POST(
        '/api/v1/projects/{projectId}/member-tags/{tagId}/assign',
        {
          params: { path: { projectId, tagId } },
          body: { memberId: memberId ?? null, contactId: contactId ?? null },
        },
      )
      if (data) {
        await getTagDetail(projectId, tagId)
        return data
      }
      return null
    } catch {
      error.value = 'Failed to assign tag.'
      return null
    } finally {
      loading.value = false
    }
  }

  async function removeAssignment(
    projectId: string,
    tagId: string,
    assignmentId: string,
  ) {
    loading.value = true
    error.value = ''
    try {
      await client.DELETE(
        '/api/v1/projects/{projectId}/member-tags/{tagId}/assignments/{assignmentId}',
        {
          params: { path: { projectId, tagId, assignmentId } },
        },
      )
      await getTagDetail(projectId, tagId)
    } catch {
      error.value = 'Failed to remove assignment.'
    } finally {
      loading.value = false
    }
  }

  async function updateExternalTaskCreation(
    projectId: string,
    tagId: string,
    settings: Record<string, never>,
  ) {
    loading.value = true
    error.value = ''
    try {
      const { data } = await client.PUT(
        '/api/v1/projects/{projectId}/member-tags/{tagId}/external-task-creation',
        {
          params: { path: { projectId, tagId } },
          body: { settings },
        },
      )
      if (data) {
        await fetchTags(projectId)
        return data
      }
      return null
    } catch {
      error.value = 'Failed to update external task creation settings.'
      return null
    } finally {
      loading.value = false
    }
  }

  return {
    tags,
    currentTagDetail,
    loading,
    error,
    fetchTags,
    createTag,
    getTagDetail,
    updateTag,
    deleteTag,
    assignTag,
    removeAssignment,
    updateExternalTaskCreation,
  }
})

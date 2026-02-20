import { ref } from 'vue'
import { defineStore } from 'pinia'
import client from '@/api/client'
import type { components } from '@/api/schema'

type MemberDetailResponse = components['schemas']['MemberDetailResponse']
type MemberResponse = components['schemas']['MemberResponse']
type MemberRole = components['schemas']['MemberRole']

export const useMemberStore = defineStore('member', () => {
  const members = ref<MemberDetailResponse[]>([])
  const currentMember = ref<MemberResponse | null>(null)
  const loading = ref(false)
  const error = ref('')

  async function fetchMembers(projectId: string, role?: MemberRole) {
    loading.value = true
    error.value = ''
    try {
      const { data } = await client.GET('/api/v1/projects/{projectId}/members', {
        params: {
          path: { projectId },
          ...(role ? { query: { role } } : {}),
        },
      })
      if (data) {
        members.value = data.members
      }
    } catch {
      error.value = 'Failed to load members.'
    } finally {
      loading.value = false
    }
  }

  async function inviteMember(
    projectId: string,
    accountId: string,
    role: MemberRole,
    tagIds?: string[],
  ) {
    loading.value = true
    error.value = ''
    try {
      const { data } = await client.POST('/api/v1/projects/{projectId}/members/invite', {
        params: { path: { projectId } },
        body: { accountId, role, tagIds: tagIds ?? null },
      })
      if (data) {
        await fetchMembers(projectId)
        return data
      }
      return null
    } catch {
      error.value = 'Failed to invite member.'
      return null
    } finally {
      loading.value = false
    }
  }

  async function getMember(projectId: string, memberId: string) {
    loading.value = true
    error.value = ''
    try {
      const { data } = await client.GET('/api/v1/projects/{projectId}/members/{memberId}', {
        params: { path: { projectId, memberId } },
      })
      if (data) {
        currentMember.value = data
      }
    } catch {
      error.value = 'Failed to load member.'
    } finally {
      loading.value = false
    }
  }

  async function updateRole(projectId: string, memberId: string, role: MemberRole) {
    loading.value = true
    error.value = ''
    try {
      await client.PUT('/api/v1/projects/{projectId}/members/{memberId}', {
        params: { path: { projectId, memberId } },
        body: { role },
      })
      await fetchMembers(projectId)
    } catch {
      error.value = 'Failed to update member role.'
    } finally {
      loading.value = false
    }
  }

  async function removeMember(projectId: string, memberId: string) {
    loading.value = true
    error.value = ''
    try {
      await client.DELETE('/api/v1/projects/{projectId}/members/{memberId}', {
        params: { path: { projectId, memberId } },
      })
      await fetchMembers(projectId)
    } catch {
      error.value = 'Failed to remove member.'
    } finally {
      loading.value = false
    }
  }

  return {
    members,
    currentMember,
    loading,
    error,
    fetchMembers,
    inviteMember,
    getMember,
    updateRole,
    removeMember,
  }
})

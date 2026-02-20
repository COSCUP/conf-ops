import { ref, computed } from 'vue'
import { defineStore } from 'pinia'
import client from '@/api/client'
import type { components } from '@/api/schema'

type OrganizationResponse = components['schemas']['OrganizationResponse']
type OrganizationListItem = components['schemas']['OrganizationListItem']
type OrgMemberResponse = components['schemas']['OrgMemberResponse']

export const useOrganizationStore = defineStore('organization', () => {
  const organizations = ref<OrganizationListItem[]>([])
  const currentOrganization = ref<OrganizationResponse | null>(null)
  const members = ref<OrgMemberResponse[]>([])
  const loading = ref(false)
  const error = ref('')

  const organizationCount = computed(() => organizations.value.length)

  async function fetchMyOrganizations() {
    loading.value = true
    error.value = ''
    try {
      const { data } = await client.GET('/api/v1/organizations')
      if (data) {
        organizations.value = data.organizations
      }
    } catch {
      error.value = 'Failed to load organizations.'
    } finally {
      loading.value = false
    }
  }

  async function fetchOrganization(orgId: string) {
    loading.value = true
    error.value = ''
    try {
      const { data } = await client.GET('/api/v1/organizations/{orgId}', {
        params: { path: { orgId } },
      })
      if (data) {
        currentOrganization.value = data
      }
    } catch {
      error.value = 'Failed to load organization.'
    } finally {
      loading.value = false
    }
  }

  async function createOrganization(name: string, description?: string, logoUrl?: string) {
    loading.value = true
    error.value = ''
    try {
      const { data } = await client.POST('/api/v1/organizations', {
        body: { name, description: description ?? null, logoUrl: logoUrl ?? null },
      })
      if (data) {
        await fetchMyOrganizations()
        return data
      }
      return null
    } catch {
      error.value = 'Failed to create organization.'
      return null
    } finally {
      loading.value = false
    }
  }

  async function updateOrganization(
    orgId: string,
    updates: { name?: string; description?: string | null; logoUrl?: string | null },
  ) {
    loading.value = true
    error.value = ''
    try {
      const { data } = await client.PUT('/api/v1/organizations/{orgId}', {
        params: { path: { orgId } },
        body: updates,
      })
      if (data) {
        currentOrganization.value = data
        await fetchMyOrganizations()
      }
    } catch {
      error.value = 'Failed to update organization.'
    } finally {
      loading.value = false
    }
  }

  async function deleteOrganization(orgId: string) {
    loading.value = true
    error.value = ''
    try {
      await client.DELETE('/api/v1/organizations/{orgId}', {
        params: { path: { orgId } },
      })
      currentOrganization.value = null
      await fetchMyOrganizations()
    } catch {
      error.value = 'Failed to delete organization. It may have active projects.'
    } finally {
      loading.value = false
    }
  }

  async function fetchMembers(orgId: string) {
    loading.value = true
    error.value = ''
    try {
      const { data } = await client.GET('/api/v1/organizations/{orgId}/members', {
        params: { path: { orgId } },
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

  async function inviteMember(orgId: string, email: string, role: string) {
    loading.value = true
    error.value = ''
    try {
      const { data } = await client.POST('/api/v1/organizations/{orgId}/members/invite', {
        params: { path: { orgId } },
        body: { email, role: role as components['schemas']['OrgRole'] },
      })
      if (data) {
        await fetchMembers(orgId)
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

  async function updateMemberRole(orgId: string, memberId: string, role: string) {
    loading.value = true
    error.value = ''
    try {
      await client.PUT('/api/v1/organizations/{orgId}/members/{memberId}', {
        params: { path: { orgId, memberId } },
        body: { role: role as components['schemas']['OrgRole'] },
      })
      await fetchMembers(orgId)
    } catch {
      error.value = 'Failed to update member role.'
    } finally {
      loading.value = false
    }
  }

  async function removeMember(orgId: string, memberId: string) {
    loading.value = true
    error.value = ''
    try {
      await client.DELETE('/api/v1/organizations/{orgId}/members/{memberId}', {
        params: { path: { orgId, memberId } },
      })
      await fetchMembers(orgId)
    } catch {
      error.value = 'Failed to remove member.'
    } finally {
      loading.value = false
    }
  }

  return {
    organizations,
    currentOrganization,
    members,
    loading,
    error,
    organizationCount,
    fetchMyOrganizations,
    fetchOrganization,
    createOrganization,
    updateOrganization,
    deleteOrganization,
    fetchMembers,
    inviteMember,
    updateMemberRole,
    removeMember,
  }
})

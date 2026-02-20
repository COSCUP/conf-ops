import { useOrganizationStore } from '@/stores/organization'

export function useOrganization() {
  const store = useOrganizationStore()

  return {
    organizations: store.organizations,
    currentOrganization: store.currentOrganization,
    members: store.members,
    loading: store.loading,
    error: store.error,
    organizationCount: store.organizationCount,
    fetchMyOrganizations: store.fetchMyOrganizations,
    fetchOrganization: store.fetchOrganization,
    createOrganization: store.createOrganization,
    updateOrganization: store.updateOrganization,
    deleteOrganization: store.deleteOrganization,
    fetchMembers: store.fetchMembers,
    inviteMember: store.inviteMember,
    updateMemberRole: store.updateMemberRole,
    removeMember: store.removeMember,
  }
}

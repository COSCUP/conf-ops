import { describe, it, expect, beforeEach, vi } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { useOrganizationStore } from '../organization'

vi.mock('@/api/client', () => ({
  default: {
    GET: vi.fn(),
    POST: vi.fn(),
    PUT: vi.fn(),
    DELETE: vi.fn(),
  },
}))

import client from '@/api/client'

describe('useOrganizationStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('starts with empty state', () => {
    const store = useOrganizationStore()
    expect(store.organizations).toEqual([])
    expect(store.currentOrganization).toBeNull()
    expect(store.members).toEqual([])
    expect(store.loading).toBe(false)
    expect(store.error).toBe('')
  })

  it('fetchMyOrganizations loads organizations', async () => {
    const mockOrgs = [
      { id: '1', name: 'Org 1', description: null, logoUrl: null, createdAt: '', role: 'org_owner' },
    ]
    vi.mocked(client.GET).mockResolvedValue({ data: { organizations: mockOrgs } } as never)

    const store = useOrganizationStore()
    await store.fetchMyOrganizations()

    expect(store.organizations).toEqual(mockOrgs)
    expect(store.loading).toBe(false)
  })

  it('fetchMyOrganizations sets error on failure', async () => {
    vi.mocked(client.GET).mockRejectedValue(new Error('fail'))

    const store = useOrganizationStore()
    await store.fetchMyOrganizations()

    expect(store.error).toBe('Failed to load organizations.')
  })

  it('fetchOrganization loads single org', async () => {
    const mockOrg = { id: '1', name: 'Test Org', description: 'Desc', createdBy: '2' }
    vi.mocked(client.GET).mockResolvedValue({ data: mockOrg } as never)

    const store = useOrganizationStore()
    await store.fetchOrganization('1')

    expect(store.currentOrganization).toEqual(mockOrg)
  })

  it('createOrganization calls POST and refreshes list', async () => {
    const mockOrg = { id: '1', name: 'New Org' }
    vi.mocked(client.POST).mockResolvedValue({ data: mockOrg } as never)
    vi.mocked(client.GET).mockResolvedValue({ data: { organizations: [mockOrg] } } as never)

    const store = useOrganizationStore()
    const result = await store.createOrganization('New Org', 'Description')

    expect(result).toEqual(mockOrg)
    expect(client.POST).toHaveBeenCalled()
  })

  it('deleteOrganization calls DELETE', async () => {
    vi.mocked(client.DELETE).mockResolvedValue({} as never)
    vi.mocked(client.GET).mockResolvedValue({ data: { organizations: [] } } as never)

    const store = useOrganizationStore()
    await store.deleteOrganization('1')

    expect(client.DELETE).toHaveBeenCalled()
    expect(store.currentOrganization).toBeNull()
  })

  it('fetchMembers loads members', async () => {
    const mockMembers = [
      { id: '1', accountId: '2', role: 'org_owner', name: 'User', email: 'u@e.com' },
    ]
    vi.mocked(client.GET).mockResolvedValue({ data: { members: mockMembers } } as never)

    const store = useOrganizationStore()
    await store.fetchMembers('org-1')

    expect(store.members).toEqual(mockMembers)
  })

  it('inviteMember calls POST', async () => {
    vi.mocked(client.POST).mockResolvedValue({ data: { memberId: '123' } } as never)
    vi.mocked(client.GET).mockResolvedValue({ data: { members: [] } } as never)

    const store = useOrganizationStore()
    const result = await store.inviteMember('org-1', 'new@user.com', 'org_member')

    expect(result).toEqual({ memberId: '123' })
  })

  it('removeMember calls DELETE', async () => {
    vi.mocked(client.DELETE).mockResolvedValue({} as never)
    vi.mocked(client.GET).mockResolvedValue({ data: { members: [] } } as never)

    const store = useOrganizationStore()
    await store.removeMember('org-1', 'member-1')

    expect(client.DELETE).toHaveBeenCalled()
  })
})

import { describe, it, expect, beforeEach, vi } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { useContactStore } from '../contact'

vi.mock('@/api/client', () => ({
  default: {
    GET: vi.fn(),
    POST: vi.fn(),
    PUT: vi.fn(),
    DELETE: vi.fn(),
  },
}))

import client from '@/api/client'

describe('useContactStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('starts with empty state', () => {
    const store = useContactStore()
    expect(store.contacts).toEqual([])
    expect(store.currentContact).toBeNull()
    expect(store.loading).toBe(false)
    expect(store.error).toBe('')
  })

  it('fetchContacts loads contacts', async () => {
    const mockContacts = [
      { id: '1', organizationId: 'org1', name: 'Alice', email: 'alice@test.com', mergedIntoId: null, createdAt: '', updatedAt: '' },
    ]
    vi.mocked(client.GET).mockResolvedValue({ data: { contacts: mockContacts } } as never)

    const store = useContactStore()
    await store.fetchContacts('org1')

    expect(store.contacts).toEqual(mockContacts)
  })

  it('createContact calls POST and refreshes list', async () => {
    const mockContact = { id: '1', organizationId: 'org1', name: 'Bob', email: 'bob@test.com', mergedIntoId: null, createdAt: '', updatedAt: '' }
    vi.mocked(client.POST).mockResolvedValue({ data: mockContact } as never)
    vi.mocked(client.GET).mockResolvedValue({ data: { contacts: [] } } as never)

    const store = useContactStore()
    const result = await store.createContact('org1', 'Bob', 'bob@test.com')

    expect(result).toEqual(mockContact)
    expect(client.POST).toHaveBeenCalled()
  })

  it('updateContact calls PUT', async () => {
    const mockContact = { id: '1', organizationId: 'org1', name: 'Updated', email: 'updated@test.com', mergedIntoId: null, createdAt: '', updatedAt: '' }
    vi.mocked(client.PUT).mockResolvedValue({ data: mockContact } as never)
    vi.mocked(client.GET).mockResolvedValue({ data: { contacts: [] } } as never)

    const store = useContactStore()
    await store.updateContact('org1', '1', { name: 'Updated' })

    expect(client.PUT).toHaveBeenCalled()
    expect(store.currentContact).toEqual(mockContact)
  })

  it('deleteContact calls DELETE', async () => {
    vi.mocked(client.DELETE).mockResolvedValue({} as never)
    vi.mocked(client.GET).mockResolvedValue({ data: { contacts: [] } } as never)

    const store = useContactStore()
    await store.deleteContact('org1', '1')

    expect(client.DELETE).toHaveBeenCalled()
  })

  it('mergeContacts calls POST merge endpoint', async () => {
    vi.mocked(client.POST).mockResolvedValue({ data: { mergedCount: 2 } } as never)
    vi.mocked(client.GET).mockResolvedValue({ data: { contacts: [] } } as never)

    const store = useContactStore()
    const result = await store.mergeContacts('org1', ['id1', 'id2'], 'target1')

    expect(result).toEqual({ mergedCount: 2 })
    expect(client.POST).toHaveBeenCalled()
  })

  it('fetchContacts sets error on failure', async () => {
    vi.mocked(client.GET).mockRejectedValue(new Error('fail'))

    const store = useContactStore()
    await store.fetchContacts('org1')

    expect(store.error).toBe('Failed to load contacts.')
  })
})

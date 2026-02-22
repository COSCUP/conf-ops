import { describe, it, expect, beforeEach, vi } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { useMemoryStore } from '../memory'

vi.mock('@/api/client', () => ({
  default: {
    GET: vi.fn(),
    POST: vi.fn(),
    PUT: vi.fn(),
    DELETE: vi.fn(),
  },
}))

import client from '@/api/client'

const makeMemory = (overrides = {}) => ({
  id: 'mem1',
  content: 'Test memory content',
  source: 'manual',
  scopeType: 'organization',
  scopeId: 'org1',
  createdBy: 'user1',
  libraryRef: null,
  createdAt: '2024-01-01T00:00:00Z',
  updatedAt: '2024-01-01T00:00:00Z',
  ...overrides,
})

const makeLibraryDoc = (overrides = {}) => ({
  id: 'doc1',
  title: 'Test Document',
  content: 'Document content',
  scopeType: 'organization',
  scopeId: 'org1',
  createdBy: 'user1',
  createdAt: '2024-01-01T00:00:00Z',
  updatedAt: '2024-01-01T00:00:00Z',
  ...overrides,
})

describe('useMemoryStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('starts with empty state', () => {
    const store = useMemoryStore()
    expect(store.memories).toEqual([])
    expect(store.memoryVersions).toEqual([])
    expect(store.libraryDocuments).toEqual([])
    expect(store.libraryDocumentVersions).toEqual([])
    expect(store.loading).toBe(false)
    expect(store.error).toBe('')
  })

  describe('listMemories', () => {
    it('loads memories from API', async () => {
      const mockMemories = [makeMemory()]
      vi.mocked(client.GET).mockResolvedValue({
        data: { items: mockMemories, nextCursor: null },
      } as never)

      const store = useMemoryStore()
      await store.listMemories('organization', 'org1')

      expect(store.memories).toEqual(mockMemories)
      expect(client.GET).toHaveBeenCalledWith('/api/v1/memories', expect.objectContaining({
        params: { query: { scopeType: 'organization', scopeId: 'org1' } },
      }))
    })

    it('appends memories when cursor is provided', async () => {
      const first = [makeMemory({ id: 'mem1' })]
      const second = [makeMemory({ id: 'mem2' })]

      vi.mocked(client.GET)
        .mockResolvedValueOnce({ data: { items: first, nextCursor: 'cursor1' } } as never)
        .mockResolvedValueOnce({ data: { items: second, nextCursor: null } } as never)

      const store = useMemoryStore()
      await store.listMemories('organization', 'org1')
      await store.listMemories('organization', 'org1', 'cursor1')

      expect(store.memories).toHaveLength(2)
      expect(store.memories[0]!.id).toBe('mem1')
      expect(store.memories[1]!.id).toBe('mem2')
    })

    it('sets error on failure', async () => {
      vi.mocked(client.GET).mockRejectedValue(new Error('fail'))

      const store = useMemoryStore()
      await store.listMemories('organization', 'org1')

      expect(store.error).toBe('Failed to load memories.')
      expect(store.loading).toBe(false)
    })
  })

  describe('createMemory', () => {
    it('calls POST and prepends to memories list', async () => {
      const mockMemory = makeMemory()
      vi.mocked(client.POST).mockResolvedValue({ data: mockMemory } as never)

      const store = useMemoryStore()
      const result = await store.createMemory({
        content: 'Test memory content',
        source: 'manual',
        scopeType: 'organization',
        scopeId: 'org1',
      })

      expect(result).toEqual(mockMemory)
      expect(store.memories).toEqual([mockMemory])
      expect(client.POST).toHaveBeenCalledWith('/api/v1/memories', expect.objectContaining({
        body: {
          content: 'Test memory content',
          source: 'manual',
          scopeType: 'organization',
          scopeId: 'org1',
        },
      }))
    })

    it('returns null and sets error on failure', async () => {
      vi.mocked(client.POST).mockRejectedValue(new Error('fail'))

      const store = useMemoryStore()
      const result = await store.createMemory({
        content: 'Test',
        source: 'manual',
        scopeType: 'organization',
        scopeId: 'org1',
      })

      expect(result).toBeNull()
      expect(store.error).toBe('Failed to create memory.')
    })
  })

  describe('updateMemory', () => {
    it('calls PUT and updates memory in list', async () => {
      const original = makeMemory({ content: 'Original' })
      const updated = makeMemory({ content: 'Updated' })
      vi.mocked(client.GET).mockResolvedValue({ data: { items: [original], nextCursor: null } } as never)
      vi.mocked(client.PUT).mockResolvedValue({ data: updated } as never)

      const store = useMemoryStore()
      await store.listMemories('organization', 'org1')
      const result = await store.updateMemory('mem1', { content: 'Updated' })

      expect(result).toEqual(updated)
      expect(store.memories[0]!.content).toBe('Updated')
    })

    it('sets error on failure', async () => {
      vi.mocked(client.PUT).mockRejectedValue(new Error('fail'))

      const store = useMemoryStore()
      await store.updateMemory('mem1', { content: 'Updated' })

      expect(store.error).toBe('Failed to update memory.')
    })
  })

  describe('deleteMemory', () => {
    it('calls DELETE and removes memory from list', async () => {
      const memories = [makeMemory({ id: 'mem1' }), makeMemory({ id: 'mem2' })]
      vi.mocked(client.GET).mockResolvedValue({ data: { items: memories, nextCursor: null } } as never)
      vi.mocked(client.DELETE).mockResolvedValue({} as never)

      const store = useMemoryStore()
      await store.listMemories('organization', 'org1')
      await store.deleteMemory('mem1')

      expect(store.memories).toHaveLength(1)
      expect(store.memories[0]!.id).toBe('mem2')
      expect(client.DELETE).toHaveBeenCalledWith('/api/v1/memories/{memoryId}', expect.objectContaining({
        params: { path: { memoryId: 'mem1' } },
      }))
    })

    it('sets error on failure', async () => {
      vi.mocked(client.DELETE).mockRejectedValue(new Error('fail'))

      const store = useMemoryStore()
      await store.deleteMemory('mem1')

      expect(store.error).toBe('Failed to delete memory.')
    })
  })

  describe('listMemoryVersions', () => {
    it('loads memory versions', async () => {
      const mockVersions = [
        { id: 'v1', memoryId: 'mem1', content: 'Version 1', changedBy: 'user1', createdAt: '2024-01-01T00:00:00Z' },
      ]
      vi.mocked(client.GET).mockResolvedValue({ data: mockVersions } as never)

      const store = useMemoryStore()
      await store.listMemoryVersions('mem1')

      expect(store.memoryVersions).toEqual(mockVersions)
      expect(client.GET).toHaveBeenCalledWith('/api/v1/memories/{memoryId}/versions', expect.objectContaining({
        params: { path: { memoryId: 'mem1' } },
      }))
    })

    it('sets error on failure', async () => {
      vi.mocked(client.GET).mockRejectedValue(new Error('fail'))

      const store = useMemoryStore()
      await store.listMemoryVersions('mem1')

      expect(store.error).toBe('Failed to load memory versions.')
    })
  })

  describe('listLibraryDocuments', () => {
    it('loads library documents from API', async () => {
      const mockDocs = [makeLibraryDoc()]
      vi.mocked(client.GET).mockResolvedValue({ data: { items: mockDocs, nextCursor: null } } as never)

      const store = useMemoryStore()
      await store.listLibraryDocuments('organization', 'org1')

      expect(store.libraryDocuments).toEqual(mockDocs)
      expect(client.GET).toHaveBeenCalledWith('/api/v1/library-documents', expect.objectContaining({
        params: { query: { scopeType: 'organization', scopeId: 'org1' } },
      }))
    })

    it('sets error on failure', async () => {
      vi.mocked(client.GET).mockRejectedValue(new Error('fail'))

      const store = useMemoryStore()
      await store.listLibraryDocuments('organization', 'org1')

      expect(store.error).toBe('Failed to load library documents.')
    })
  })

  describe('createLibraryDocument', () => {
    it('calls POST and prepends to libraryDocuments list', async () => {
      const mockDoc = makeLibraryDoc()
      vi.mocked(client.POST).mockResolvedValue({ data: mockDoc } as never)

      const store = useMemoryStore()
      const result = await store.createLibraryDocument({
        title: 'Test Document',
        content: 'Document content',
        scopeType: 'organization',
        scopeId: 'org1',
      })

      expect(result).toEqual(mockDoc)
      expect(store.libraryDocuments).toEqual([mockDoc])
    })

    it('returns null and sets error on failure', async () => {
      vi.mocked(client.POST).mockRejectedValue(new Error('fail'))

      const store = useMemoryStore()
      const result = await store.createLibraryDocument({
        title: 'Test',
        content: 'Content',
        scopeType: 'organization',
        scopeId: 'org1',
      })

      expect(result).toBeNull()
      expect(store.error).toBe('Failed to create library document.')
    })
  })

  describe('updateLibraryDocument', () => {
    it('calls PUT and updates document in list', async () => {
      const original = makeLibraryDoc({ content: 'Original' })
      const updated = makeLibraryDoc({ content: 'Updated' })
      vi.mocked(client.GET).mockResolvedValue({ data: { items: [original], nextCursor: null } } as never)
      vi.mocked(client.PUT).mockResolvedValue({ data: updated } as never)

      const store = useMemoryStore()
      await store.listLibraryDocuments('organization', 'org1')
      const result = await store.updateLibraryDocument('doc1', { content: 'Updated' })

      expect(result).toEqual(updated)
      expect(store.libraryDocuments[0]!.content).toBe('Updated')
    })

    it('sets error on failure', async () => {
      vi.mocked(client.PUT).mockRejectedValue(new Error('fail'))

      const store = useMemoryStore()
      await store.updateLibraryDocument('doc1', { content: 'Updated' })

      expect(store.error).toBe('Failed to update library document.')
    })
  })

  describe('deleteLibraryDocument', () => {
    it('calls DELETE and removes document from list', async () => {
      const docs = [makeLibraryDoc({ id: 'doc1' }), makeLibraryDoc({ id: 'doc2' })]
      vi.mocked(client.GET).mockResolvedValue({ data: { items: docs, nextCursor: null } } as never)
      vi.mocked(client.DELETE).mockResolvedValue({} as never)

      const store = useMemoryStore()
      await store.listLibraryDocuments('organization', 'org1')
      await store.deleteLibraryDocument('doc1')

      expect(store.libraryDocuments).toHaveLength(1)
      expect(store.libraryDocuments[0]!.id).toBe('doc2')
    })

    it('sets error on failure', async () => {
      vi.mocked(client.DELETE).mockRejectedValue(new Error('fail'))

      const store = useMemoryStore()
      await store.deleteLibraryDocument('doc1')

      expect(store.error).toBe('Failed to delete library document.')
    })
  })

  describe('listLibraryDocumentVersions', () => {
    it('loads library document versions', async () => {
      const mockVersions = [
        { id: 'v1', documentId: 'doc1', content: 'Version 1', title: null, changedBy: 'user1', createdAt: '2024-01-01T00:00:00Z' },
      ]
      vi.mocked(client.GET).mockResolvedValue({ data: mockVersions } as never)

      const store = useMemoryStore()
      await store.listLibraryDocumentVersions('doc1')

      expect(store.libraryDocumentVersions).toEqual(mockVersions)
      expect(client.GET).toHaveBeenCalledWith('/api/v1/library-documents/{documentId}/versions', expect.objectContaining({
        params: { path: { documentId: 'doc1' } },
      }))
    })

    it('sets error on failure', async () => {
      vi.mocked(client.GET).mockRejectedValue(new Error('fail'))

      const store = useMemoryStore()
      await store.listLibraryDocumentVersions('doc1')

      expect(store.error).toBe('Failed to load library document versions.')
    })
  })
})

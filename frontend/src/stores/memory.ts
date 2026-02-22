import { ref, reactive } from 'vue'
import { defineStore } from 'pinia'
import client from '@/api/client'
import type { components } from '@/api/schema'

type MemoryResponse = components['schemas']['MemoryResponse']
type MemoryVersionResponse = components['schemas']['MemoryVersionResponse']
type LibraryDocumentResponse = components['schemas']['LibraryDocumentResponse']
type LibraryDocumentVersionResponse = components['schemas']['LibraryDocumentVersionResponse']
type CreateMemoryRequest = components['schemas']['CreateMemoryRequest']
type UpdateMemoryRequest = components['schemas']['UpdateMemoryRequest']
type CreateLibraryDocumentRequest = components['schemas']['CreateLibraryDocumentRequest']
type UpdateLibraryDocumentRequest = components['schemas']['UpdateLibraryDocumentRequest']

export const SCOPE_LABELS: Record<string, string> = {
  account: 'Account',
  organization: 'Organization',
  project: 'Project',
  member_tag: 'Member Tag',
  task_template: 'Task Template',
  task: 'Task',
}

export const SCOPE_ORDER: string[] = [
  'account',
  'organization',
  'project',
  'member_tag',
  'task_template',
  'task',
]

export interface TaskMemoryContext {
  accountId: string
  organizationId: string
  projectId: string
  memberTagId?: string
  taskTemplateId?: string
  taskId: string
}

export const useMemoryStore = defineStore('memory', () => {
  const memories = ref<MemoryResponse[]>([])
  const groupedMemories = reactive<Record<string, MemoryResponse[]>>({})
  const memoryVersions = ref<MemoryVersionResponse[]>([])
  const libraryDocuments = ref<LibraryDocumentResponse[]>([])
  const libraryDocumentVersions = ref<LibraryDocumentVersionResponse[]>([])
  const loading = ref(false)
  const error = ref('')

  async function listMemories(scopeType: string, scopeId: string, cursor?: string) {
    loading.value = true
    error.value = ''
    try {
      const { data } = await client.GET('/api/v1/memories', {
        params: {
          query: {
            scopeType,
            scopeId,
            ...(cursor ? { cursor } : {}),
          },
        },
      })
      if (data) {
        if (cursor) {
          memories.value = [...memories.value, ...data.items]
        } else {
          memories.value = data.items
        }
      }
    } catch {
      error.value = 'Failed to load memories.'
    } finally {
      loading.value = false
    }
  }

  async function createMemory(body: CreateMemoryRequest) {
    loading.value = true
    error.value = ''
    try {
      const { data } = await client.POST('/api/v1/memories', {
        body,
      })
      if (data) {
        memories.value = [data, ...memories.value]
        return data
      }
      return null
    } catch {
      error.value = 'Failed to create memory.'
      return null
    } finally {
      loading.value = false
    }
  }

  async function updateMemory(memoryId: string, body: UpdateMemoryRequest) {
    loading.value = true
    error.value = ''
    try {
      const { data } = await client.PUT('/api/v1/memories/{memoryId}', {
        params: { path: { memoryId } },
        body,
      })
      if (data) {
        const index = memories.value.findIndex((m) => m.id === memoryId)
        if (index !== -1) {
          memories.value[index] = data
        }
        return data
      }
      return null
    } catch {
      error.value = 'Failed to update memory.'
      return null
    } finally {
      loading.value = false
    }
  }

  async function deleteMemory(memoryId: string) {
    loading.value = true
    error.value = ''
    try {
      await client.DELETE('/api/v1/memories/{memoryId}', {
        params: { path: { memoryId } },
      })
      memories.value = memories.value.filter((m) => m.id !== memoryId)
    } catch {
      error.value = 'Failed to delete memory.'
    } finally {
      loading.value = false
    }
  }

  async function listMemoryVersions(memoryId: string) {
    loading.value = true
    error.value = ''
    try {
      const { data } = await client.GET('/api/v1/memories/{memoryId}/versions', {
        params: { path: { memoryId } },
      })
      if (data) {
        memoryVersions.value = data
      }
    } catch {
      error.value = 'Failed to load memory versions.'
    } finally {
      loading.value = false
    }
  }

  async function listLibraryDocuments(scopeType: string, scopeId: string, cursor?: string) {
    loading.value = true
    error.value = ''
    try {
      const { data } = await client.GET('/api/v1/library-documents', {
        params: {
          query: {
            scopeType,
            scopeId,
            ...(cursor ? { cursor } : {}),
          },
        },
      })
      if (data) {
        if (cursor) {
          libraryDocuments.value = [...libraryDocuments.value, ...data.items]
        } else {
          libraryDocuments.value = data.items
        }
      }
    } catch {
      error.value = 'Failed to load library documents.'
    } finally {
      loading.value = false
    }
  }

  async function createLibraryDocument(body: CreateLibraryDocumentRequest) {
    loading.value = true
    error.value = ''
    try {
      const { data } = await client.POST('/api/v1/library-documents', {
        body,
      })
      if (data) {
        libraryDocuments.value = [data, ...libraryDocuments.value]
        return data
      }
      return null
    } catch {
      error.value = 'Failed to create library document.'
      return null
    } finally {
      loading.value = false
    }
  }

  async function updateLibraryDocument(documentId: string, body: UpdateLibraryDocumentRequest) {
    loading.value = true
    error.value = ''
    try {
      const { data } = await client.PUT('/api/v1/library-documents/{documentId}', {
        params: { path: { documentId } },
        body,
      })
      if (data) {
        const index = libraryDocuments.value.findIndex((d) => d.id === documentId)
        if (index !== -1) {
          libraryDocuments.value[index] = data
        }
        return data
      }
      return null
    } catch {
      error.value = 'Failed to update library document.'
      return null
    } finally {
      loading.value = false
    }
  }

  async function deleteLibraryDocument(documentId: string) {
    loading.value = true
    error.value = ''
    try {
      await client.DELETE('/api/v1/library-documents/{documentId}', {
        params: { path: { documentId } },
      })
      libraryDocuments.value = libraryDocuments.value.filter((d) => d.id !== documentId)
    } catch {
      error.value = 'Failed to delete library document.'
    } finally {
      loading.value = false
    }
  }

  async function listLibraryDocumentVersions(documentId: string) {
    loading.value = true
    error.value = ''
    try {
      const { data } = await client.GET('/api/v1/library-documents/{documentId}/versions', {
        params: { path: { documentId } },
      })
      if (data) {
        libraryDocumentVersions.value = data
      }
    } catch {
      error.value = 'Failed to load library document versions.'
    } finally {
      loading.value = false
    }
  }

  async function loadGroupedMemories(ctx: TaskMemoryContext) {
    loading.value = true
    error.value = ''
    try {
      const scopes: { scopeType: string; scopeId: string }[] = [
        { scopeType: 'account', scopeId: ctx.accountId },
        { scopeType: 'organization', scopeId: ctx.organizationId },
        { scopeType: 'project', scopeId: ctx.projectId },
      ]
      if (ctx.memberTagId) {
        scopes.push({ scopeType: 'member_tag', scopeId: ctx.memberTagId })
      }
      if (ctx.taskTemplateId) {
        scopes.push({ scopeType: 'task_template', scopeId: ctx.taskTemplateId })
      }
      scopes.push({ scopeType: 'task', scopeId: ctx.taskId })

      const results = await Promise.all(
        scopes.map(async (s) => {
          const { data } = await client.GET('/api/v1/memories', {
            params: { query: { scopeType: s.scopeType, scopeId: s.scopeId } },
          })
          return { scopeType: s.scopeType, items: data?.items ?? [] }
        }),
      )

      for (const scope of SCOPE_ORDER) {
        groupedMemories[scope] = []
      }
      for (const result of results) {
        groupedMemories[result.scopeType] = result.items
      }
    } catch {
      error.value = 'Failed to load grouped memories.'
    } finally {
      loading.value = false
    }
  }

  return {
    memories,
    groupedMemories,
    memoryVersions,
    libraryDocuments,
    libraryDocumentVersions,
    loading,
    error,
    listMemories,
    loadGroupedMemories,
    createMemory,
    updateMemory,
    deleteMemory,
    listMemoryVersions,
    listLibraryDocuments,
    createLibraryDocument,
    updateLibraryDocument,
    deleteLibraryDocument,
    listLibraryDocumentVersions,
  }
})

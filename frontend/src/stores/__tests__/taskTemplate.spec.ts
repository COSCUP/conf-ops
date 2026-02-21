import { describe, it, expect, beforeEach, vi } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { useTaskTemplateStore } from '../taskTemplate'

vi.mock('@/api/client', () => ({
  default: {
    GET: vi.fn(),
    POST: vi.fn(),
    PUT: vi.fn(),
    DELETE: vi.fn(),
  },
}))

import client from '@/api/client'

describe('useTaskTemplateStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('starts with empty state', () => {
    const store = useTaskTemplateStore()
    expect(store.templates).toEqual([])
    expect(store.currentTemplate).toBeNull()
    expect(store.todoTemplates).toEqual([])
    expect(store.dataSchemas).toEqual([])
    expect(store.linkedTags).toEqual([])
    expect(store.loading).toBe(false)
    expect(store.error).toBe('')
  })

  // ── Template CRUD ──────────────────────────────────────────

  it('fetchTemplates loads templates', async () => {
    const mockTemplates = [{ id: 'tt1', name: 'Template 1', createdAt: '', updatedAt: '' }]
    vi.mocked(client.GET).mockResolvedValue({ data: { templates: mockTemplates } } as never)

    const store = useTaskTemplateStore()
    await store.fetchTemplates('p1')

    expect(store.templates).toEqual(mockTemplates)
  })

  it('fetchTemplates sets error on failure', async () => {
    vi.mocked(client.GET).mockRejectedValue(new Error('fail'))

    const store = useTaskTemplateStore()
    await store.fetchTemplates('p1')

    expect(store.error).toBe('Failed to load task templates.')
  })

  it('getTemplate loads a single template', async () => {
    const mockTemplate = { id: 'tt1', name: 'Template 1', createdAt: '', updatedAt: '' }
    vi.mocked(client.GET).mockResolvedValue({ data: mockTemplate } as never)

    const store = useTaskTemplateStore()
    await store.getTemplate('p1', 'tt1')

    expect(store.currentTemplate).toEqual(mockTemplate)
  })

  it('getTemplate sets error on failure', async () => {
    vi.mocked(client.GET).mockRejectedValue(new Error('fail'))

    const store = useTaskTemplateStore()
    await store.getTemplate('p1', 'tt1')

    expect(store.error).toBe('Failed to load task template.')
  })

  it('createTemplate calls POST and refetches', async () => {
    const mockTemplate = { id: 'tt1', name: 'New Template', createdAt: '', updatedAt: '' }
    vi.mocked(client.POST).mockResolvedValue({ data: mockTemplate } as never)
    vi.mocked(client.GET).mockResolvedValue({ data: { templates: [] } } as never)

    const store = useTaskTemplateStore()
    const result = await store.createTemplate('p1', 'New Template', 'desc')

    expect(result).toEqual(mockTemplate)
    expect(client.POST).toHaveBeenCalled()
    expect(client.GET).toHaveBeenCalled()
  })

  it('createTemplate sets error on failure', async () => {
    vi.mocked(client.POST).mockRejectedValue(new Error('fail'))

    const store = useTaskTemplateStore()
    const result = await store.createTemplate('p1', 'New Template')

    expect(result).toBeNull()
    expect(store.error).toBe('Failed to create task template.')
  })

  it('updateTemplate sets currentTemplate from response', async () => {
    const mockTemplate = { id: 'tt1', name: 'Updated', createdAt: '', updatedAt: '' }
    vi.mocked(client.PUT).mockResolvedValue({ data: mockTemplate } as never)

    const store = useTaskTemplateStore()
    await store.updateTemplate('p1', 'tt1', { name: 'Updated' })

    expect(store.currentTemplate).toEqual(mockTemplate)
    expect(client.PUT).toHaveBeenCalled()
  })

  it('updateTemplate sets error on failure', async () => {
    vi.mocked(client.PUT).mockRejectedValue(new Error('fail'))

    const store = useTaskTemplateStore()
    await store.updateTemplate('p1', 'tt1', { name: 'Updated' })

    expect(store.error).toBe('Failed to update task template.')
  })

  it('deleteTemplate calls DELETE and refetches', async () => {
    vi.mocked(client.DELETE).mockResolvedValue({} as never)
    vi.mocked(client.GET).mockResolvedValue({ data: { templates: [] } } as never)

    const store = useTaskTemplateStore()
    await store.deleteTemplate('p1', 'tt1')

    expect(client.DELETE).toHaveBeenCalled()
    expect(client.GET).toHaveBeenCalled()
  })

  // ── Todo Templates ─────────────────────────────────────────

  it('fetchTodoTemplates loads todo templates', async () => {
    const mockTodoTemplates = [{ id: 'tod1', name: 'Todo 1', sortOrder: 0, createdAt: '', updatedAt: '' }]
    vi.mocked(client.GET).mockResolvedValue({ data: { todoTemplates: mockTodoTemplates } } as never)

    const store = useTaskTemplateStore()
    await store.fetchTodoTemplates('p1', 'tt1')

    expect(store.todoTemplates).toEqual(mockTodoTemplates)
  })

  it('fetchTodoTemplates sets error on failure', async () => {
    vi.mocked(client.GET).mockRejectedValue(new Error('fail'))

    const store = useTaskTemplateStore()
    await store.fetchTodoTemplates('p1', 'tt1')

    expect(store.error).toBe('Failed to load todo templates.')
  })

  it('createTodoTemplate calls POST with parentId and sortOrder', async () => {
    const mockTodo = { id: 'tod1', name: 'New Todo', sortOrder: 1, createdAt: '', updatedAt: '' }
    vi.mocked(client.POST).mockResolvedValue({ data: mockTodo } as never)
    vi.mocked(client.GET).mockResolvedValue({ data: { todoTemplates: [] } } as never)

    const store = useTaskTemplateStore()
    const result = await store.createTodoTemplate('p1', 'tt1', 'New Todo', 1, 'parent1', 'desc')

    expect(result).toEqual(mockTodo)
    expect(client.POST).toHaveBeenCalled()
    expect(client.GET).toHaveBeenCalled()
  })

  it('createTodoTemplate sets error on failure', async () => {
    vi.mocked(client.POST).mockRejectedValue(new Error('fail'))

    const store = useTaskTemplateStore()
    const result = await store.createTodoTemplate('p1', 'tt1', 'New Todo', 0)

    expect(result).toBeNull()
    expect(store.error).toBe('Failed to create todo template.')
  })

  it('updateTodoTemplate calls PUT and refetches', async () => {
    vi.mocked(client.PUT).mockResolvedValue({} as never)
    vi.mocked(client.GET).mockResolvedValue({ data: { todoTemplates: [] } } as never)

    const store = useTaskTemplateStore()
    await store.updateTodoTemplate('p1', 'tt1', 'tod1', { name: 'Updated' })

    expect(client.PUT).toHaveBeenCalled()
    expect(client.GET).toHaveBeenCalled()
  })

  it('deleteTodoTemplate calls DELETE and refetches', async () => {
    vi.mocked(client.DELETE).mockResolvedValue({} as never)
    vi.mocked(client.GET).mockResolvedValue({ data: { todoTemplates: [] } } as never)

    const store = useTaskTemplateStore()
    await store.deleteTodoTemplate('p1', 'tt1', 'tod1')

    expect(client.DELETE).toHaveBeenCalled()
    expect(client.GET).toHaveBeenCalled()
  })

  it('reorderTodoTemplates calls PUT with orders and refetches', async () => {
    vi.mocked(client.PUT).mockResolvedValue({} as never)
    vi.mocked(client.GET).mockResolvedValue({ data: { todoTemplates: [] } } as never)

    const store = useTaskTemplateStore()
    await store.reorderTodoTemplates('p1', 'tt1', [
      { id: 'tod1', sortOrder: 0 },
      { id: 'tod2', sortOrder: 1 },
    ])

    expect(client.PUT).toHaveBeenCalled()
    expect(client.GET).toHaveBeenCalled()
  })

  it('reorderTodoTemplates sets error on failure', async () => {
    vi.mocked(client.PUT).mockRejectedValue(new Error('fail'))

    const store = useTaskTemplateStore()
    await store.reorderTodoTemplates('p1', 'tt1', [{ id: 'tod1', sortOrder: 0 }])

    expect(store.error).toBe('Failed to reorder todo templates.')
  })

  // ── Data Schemas ───────────────────────────────────────────

  it('fetchDataSchemas loads data schemas', async () => {
    const mockSchemas = [{ id: 'ds1', name: 'Schema 1', fields: [], createdAt: '', updatedAt: '' }]
    vi.mocked(client.GET).mockResolvedValue({ data: { dataSchemas: mockSchemas } } as never)

    const store = useTaskTemplateStore()
    await store.fetchDataSchemas('p1', 'tt1')

    expect(store.dataSchemas).toEqual(mockSchemas)
  })

  it('fetchDataSchemas sets error on failure', async () => {
    vi.mocked(client.GET).mockRejectedValue(new Error('fail'))

    const store = useTaskTemplateStore()
    await store.fetchDataSchemas('p1', 'tt1')

    expect(store.error).toBe('Failed to load data schemas.')
  })

  it('createDataSchema calls POST with fields', async () => {
    const mockSchema = { id: 'ds1', name: 'New Schema', fields: [{ name: 'field1', fieldType: 'text' }], createdAt: '', updatedAt: '' }
    vi.mocked(client.POST).mockResolvedValue({ data: mockSchema } as never)
    vi.mocked(client.GET).mockResolvedValue({ data: { dataSchemas: [] } } as never)

    const store = useTaskTemplateStore()
    const result = await store.createDataSchema('p1', 'tt1', 'New Schema', [{ name: 'field1', fieldType: 'text' }] as never)

    expect(result).toEqual(mockSchema)
    expect(client.POST).toHaveBeenCalled()
    expect(client.GET).toHaveBeenCalled()
  })

  it('createDataSchema sets error on failure', async () => {
    vi.mocked(client.POST).mockRejectedValue(new Error('fail'))

    const store = useTaskTemplateStore()
    const result = await store.createDataSchema('p1', 'tt1', 'New Schema', [] as never)

    expect(result).toBeNull()
    expect(store.error).toBe('Failed to create data schema.')
  })

  it('updateDataSchema calls PUT and refetches', async () => {
    vi.mocked(client.PUT).mockResolvedValue({} as never)
    vi.mocked(client.GET).mockResolvedValue({ data: { dataSchemas: [] } } as never)

    const store = useTaskTemplateStore()
    await store.updateDataSchema('p1', 'tt1', 'ds1', { name: 'Updated' })

    expect(client.PUT).toHaveBeenCalled()
    expect(client.GET).toHaveBeenCalled()
  })

  it('deleteDataSchema calls DELETE and refetches', async () => {
    vi.mocked(client.DELETE).mockResolvedValue({} as never)
    vi.mocked(client.GET).mockResolvedValue({ data: { dataSchemas: [] } } as never)

    const store = useTaskTemplateStore()
    await store.deleteDataSchema('p1', 'tt1', 'ds1')

    expect(client.DELETE).toHaveBeenCalled()
    expect(client.GET).toHaveBeenCalled()
  })

  // ── Tags ───────────────────────────────────────────────────

  it('fetchLinkedTags loads tags', async () => {
    const mockTags = [{ id: 'tag1', memberTagId: 'mt1', name: 'Tag 1', createdAt: '', updatedAt: '' }]
    vi.mocked(client.GET).mockResolvedValue({ data: { tags: mockTags } } as never)

    const store = useTaskTemplateStore()
    await store.fetchLinkedTags('p1', 'tt1')

    expect(store.linkedTags).toEqual(mockTags)
  })

  it('fetchLinkedTags sets error on failure', async () => {
    vi.mocked(client.GET).mockRejectedValue(new Error('fail'))

    const store = useTaskTemplateStore()
    await store.fetchLinkedTags('p1', 'tt1')

    expect(store.error).toBe('Failed to load linked tags.')
  })

  it('linkTag calls POST and refetches', async () => {
    vi.mocked(client.POST).mockResolvedValue({} as never)
    vi.mocked(client.GET).mockResolvedValue({ data: { tags: [] } } as never)

    const store = useTaskTemplateStore()
    await store.linkTag('p1', 'tt1', 'mt1')

    expect(client.POST).toHaveBeenCalled()
    expect(client.GET).toHaveBeenCalled()
  })

  it('linkTag sets error on failure', async () => {
    vi.mocked(client.POST).mockRejectedValue(new Error('fail'))

    const store = useTaskTemplateStore()
    await store.linkTag('p1', 'tt1', 'mt1')

    expect(store.error).toBe('Failed to link tag.')
  })

  it('unlinkTag calls DELETE and refetches', async () => {
    vi.mocked(client.DELETE).mockResolvedValue({} as never)
    vi.mocked(client.GET).mockResolvedValue({ data: { tags: [] } } as never)

    const store = useTaskTemplateStore()
    await store.unlinkTag('p1', 'tt1', 'mt1')

    expect(client.DELETE).toHaveBeenCalled()
    expect(client.GET).toHaveBeenCalled()
  })

  it('unlinkTag sets error on failure', async () => {
    vi.mocked(client.DELETE).mockRejectedValue(new Error('fail'))

    const store = useTaskTemplateStore()
    await store.unlinkTag('p1', 'tt1', 'mt1')

    expect(store.error).toBe('Failed to unlink tag.')
  })
})

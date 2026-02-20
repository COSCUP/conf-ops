import { ref } from 'vue'
import { defineStore } from 'pinia'
import client from '@/api/client'
import type { components } from '@/api/schema'

type TaskTemplateResponse = components['schemas']['TaskTemplateResponse']
type TodoTemplateResponse = components['schemas']['TodoTemplateResponse']
type DataSchemaResponse = components['schemas']['DataSchemaResponse']
type DataSchemaField = components['schemas']['DataSchemaField']
type TaskTemplateTagResponse = components['schemas']['TaskTemplateTagResponse']

export const useTaskTemplateStore = defineStore('taskTemplate', () => {
  const templates = ref<TaskTemplateResponse[]>([])
  const currentTemplate = ref<TaskTemplateResponse | null>(null)
  const todoTemplates = ref<TodoTemplateResponse[]>([])
  const dataSchemas = ref<DataSchemaResponse[]>([])
  const linkedTags = ref<TaskTemplateTagResponse[]>([])
  const loading = ref(false)
  const error = ref('')

  async function fetchTemplates(projectId: string) {
    loading.value = true
    error.value = ''
    try {
      const { data } = await client.GET(
        '/api/v1/projects/{projectId}/task-templates',
        { params: { path: { projectId } } },
      )
      if (data) {
        templates.value = data.templates
      }
    } catch {
      error.value = 'Failed to load task templates.'
    } finally {
      loading.value = false
    }
  }

  async function getTemplate(projectId: string, templateId: string) {
    loading.value = true
    error.value = ''
    try {
      const { data } = await client.GET(
        '/api/v1/projects/{projectId}/task-templates/{templateId}',
        { params: { path: { projectId, templateId } } },
      )
      if (data) {
        currentTemplate.value = data
      }
    } catch {
      error.value = 'Failed to load task template.'
    } finally {
      loading.value = false
    }
  }

  async function createTemplate(projectId: string, name: string, description?: string) {
    loading.value = true
    error.value = ''
    try {
      const { data } = await client.POST(
        '/api/v1/projects/{projectId}/task-templates',
        {
          params: { path: { projectId } },
          body: { name, description: description ?? null },
        },
      )
      if (data) {
        await fetchTemplates(projectId)
        return data
      }
      return null
    } catch {
      error.value = 'Failed to create task template.'
      return null
    } finally {
      loading.value = false
    }
  }

  async function updateTemplate(
    projectId: string,
    templateId: string,
    body: { name?: string | null; description?: string | null },
  ) {
    loading.value = true
    error.value = ''
    try {
      const { data } = await client.PUT(
        '/api/v1/projects/{projectId}/task-templates/{templateId}',
        { params: { path: { projectId, templateId } }, body },
      )
      if (data) {
        currentTemplate.value = data
      }
    } catch {
      error.value = 'Failed to update task template.'
    } finally {
      loading.value = false
    }
  }

  async function deleteTemplate(projectId: string, templateId: string) {
    loading.value = true
    error.value = ''
    try {
      await client.DELETE(
        '/api/v1/projects/{projectId}/task-templates/{templateId}',
        { params: { path: { projectId, templateId } } },
      )
      await fetchTemplates(projectId)
    } catch {
      error.value = 'Failed to delete task template.'
    } finally {
      loading.value = false
    }
  }

  // ── Todo Templates ────────────────────────────────────────

  async function fetchTodoTemplates(projectId: string, templateId: string) {
    loading.value = true
    error.value = ''
    try {
      const { data } = await client.GET(
        '/api/v1/projects/{projectId}/task-templates/{templateId}/todo-templates',
        { params: { path: { projectId, templateId } } },
      )
      if (data) {
        todoTemplates.value = data.todoTemplates
      }
    } catch {
      error.value = 'Failed to load todo templates.'
    } finally {
      loading.value = false
    }
  }

  async function createTodoTemplate(
    projectId: string,
    templateId: string,
    name: string,
    sortOrder: number,
    parentId?: string,
    description?: string,
  ) {
    loading.value = true
    error.value = ''
    try {
      const { data } = await client.POST(
        '/api/v1/projects/{projectId}/task-templates/{templateId}/todo-templates',
        {
          params: { path: { projectId, templateId } },
          body: {
            name,
            sortOrder,
            parentId: parentId ?? null,
            description: description ?? null,
          },
        },
      )
      if (data) {
        await fetchTodoTemplates(projectId, templateId)
        return data
      }
      return null
    } catch {
      error.value = 'Failed to create todo template.'
      return null
    } finally {
      loading.value = false
    }
  }

  async function updateTodoTemplate(
    projectId: string,
    templateId: string,
    todoTemplateId: string,
    body: { name?: string | null; description?: string | null },
  ) {
    loading.value = true
    error.value = ''
    try {
      await client.PUT(
        '/api/v1/projects/{projectId}/task-templates/{templateId}/todo-templates/{todoTemplateId}',
        { params: { path: { projectId, templateId, todoTemplateId } }, body },
      )
      await fetchTodoTemplates(projectId, templateId)
    } catch {
      error.value = 'Failed to update todo template.'
    } finally {
      loading.value = false
    }
  }

  async function deleteTodoTemplate(
    projectId: string,
    templateId: string,
    todoTemplateId: string,
  ) {
    loading.value = true
    error.value = ''
    try {
      await client.DELETE(
        '/api/v1/projects/{projectId}/task-templates/{templateId}/todo-templates/{todoTemplateId}',
        { params: { path: { projectId, templateId, todoTemplateId } } },
      )
      await fetchTodoTemplates(projectId, templateId)
    } catch {
      error.value = 'Failed to delete todo template.'
    } finally {
      loading.value = false
    }
  }

  async function reorderTodoTemplates(
    projectId: string,
    templateId: string,
    orders: { id: string; sortOrder: number }[],
  ) {
    loading.value = true
    error.value = ''
    try {
      await client.PUT(
        '/api/v1/projects/{projectId}/task-templates/{templateId}/todo-templates/reorder',
        {
          params: { path: { projectId, templateId } },
          body: { orders },
        },
      )
      await fetchTodoTemplates(projectId, templateId)
    } catch {
      error.value = 'Failed to reorder todo templates.'
    } finally {
      loading.value = false
    }
  }

  // ── Data Schemas ──────────────────────────────────────────

  async function fetchDataSchemas(projectId: string, templateId: string) {
    loading.value = true
    error.value = ''
    try {
      const { data } = await client.GET(
        '/api/v1/projects/{projectId}/task-templates/{templateId}/data-schemas',
        { params: { path: { projectId, templateId } } },
      )
      if (data) {
        dataSchemas.value = data.dataSchemas
      }
    } catch {
      error.value = 'Failed to load data schemas.'
    } finally {
      loading.value = false
    }
  }

  async function createDataSchema(
    projectId: string,
    templateId: string,
    name: string,
    fields: DataSchemaField[],
  ) {
    loading.value = true
    error.value = ''
    try {
      const { data } = await client.POST(
        '/api/v1/projects/{projectId}/task-templates/{templateId}/data-schemas',
        {
          params: { path: { projectId, templateId } },
          body: { name, fields },
        },
      )
      if (data) {
        await fetchDataSchemas(projectId, templateId)
        return data
      }
      return null
    } catch {
      error.value = 'Failed to create data schema.'
      return null
    } finally {
      loading.value = false
    }
  }

  async function updateDataSchema(
    projectId: string,
    templateId: string,
    schemaId: string,
    body: { name?: string | null; fields?: DataSchemaField[] | null },
  ) {
    loading.value = true
    error.value = ''
    try {
      await client.PUT(
        '/api/v1/projects/{projectId}/task-templates/{templateId}/data-schemas/{schemaId}',
        { params: { path: { projectId, templateId, schemaId } }, body },
      )
      await fetchDataSchemas(projectId, templateId)
    } catch {
      error.value = 'Failed to update data schema.'
    } finally {
      loading.value = false
    }
  }

  async function deleteDataSchema(
    projectId: string,
    templateId: string,
    schemaId: string,
  ) {
    loading.value = true
    error.value = ''
    try {
      await client.DELETE(
        '/api/v1/projects/{projectId}/task-templates/{templateId}/data-schemas/{schemaId}',
        { params: { path: { projectId, templateId, schemaId } } },
      )
      await fetchDataSchemas(projectId, templateId)
    } catch {
      error.value = 'Failed to delete data schema.'
    } finally {
      loading.value = false
    }
  }

  // ── Tags ──────────────────────────────────────────────────

  async function fetchLinkedTags(projectId: string, templateId: string) {
    loading.value = true
    error.value = ''
    try {
      const { data } = await client.GET(
        '/api/v1/projects/{projectId}/task-templates/{templateId}/tags',
        { params: { path: { projectId, templateId } } },
      )
      if (data) {
        linkedTags.value = data.tags
      }
    } catch {
      error.value = 'Failed to load linked tags.'
    } finally {
      loading.value = false
    }
  }

  async function linkTag(projectId: string, templateId: string, memberTagId: string) {
    loading.value = true
    error.value = ''
    try {
      await client.POST(
        '/api/v1/projects/{projectId}/task-templates/{templateId}/tags',
        {
          params: { path: { projectId, templateId } },
          body: { memberTagId },
        },
      )
      await fetchLinkedTags(projectId, templateId)
    } catch {
      error.value = 'Failed to link tag.'
    } finally {
      loading.value = false
    }
  }

  async function unlinkTag(projectId: string, templateId: string, memberTagId: string) {
    loading.value = true
    error.value = ''
    try {
      await client.DELETE(
        '/api/v1/projects/{projectId}/task-templates/{templateId}/tags/{memberTagId}',
        { params: { path: { projectId, templateId, memberTagId } } },
      )
      await fetchLinkedTags(projectId, templateId)
    } catch {
      error.value = 'Failed to unlink tag.'
    } finally {
      loading.value = false
    }
  }

  return {
    templates,
    currentTemplate,
    todoTemplates,
    dataSchemas,
    linkedTags,
    loading,
    error,
    fetchTemplates,
    getTemplate,
    createTemplate,
    updateTemplate,
    deleteTemplate,
    fetchTodoTemplates,
    createTodoTemplate,
    updateTodoTemplate,
    deleteTodoTemplate,
    reorderTodoTemplates,
    fetchDataSchemas,
    createDataSchema,
    updateDataSchema,
    deleteDataSchema,
    fetchLinkedTags,
    linkTag,
    unlinkTag,
  }
})

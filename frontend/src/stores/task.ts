import { ref } from 'vue'
import { defineStore } from 'pinia'
import client from '@/api/client'
import type { components } from '@/api/schema'

type TaskResponse = components['schemas']['TaskResponse']
type TaskStatus = components['schemas']['TaskStatus']

export const useTaskStore = defineStore('task', () => {
  const tasks = ref<TaskResponse[]>([])
  const currentTask = ref<TaskResponse | null>(null)
  const loading = ref(false)
  const error = ref('')

  async function fetchTasks(projectId: string, status?: TaskStatus, tagId?: string) {
    loading.value = true
    error.value = ''
    try {
      const query: Record<string, string> = {}
      if (status) query.status = status
      if (tagId) query.tagId = tagId

      const { data } = await client.GET('/api/v1/projects/{projectId}/tasks', {
        params: {
          path: { projectId },
          ...(Object.keys(query).length > 0 ? { query } : {}),
        },
      })
      if (data) {
        tasks.value = data.tasks
      }
    } catch {
      error.value = 'Failed to load tasks.'
    } finally {
      loading.value = false
    }
  }

  async function getTask(projectId: string, taskId: string) {
    loading.value = true
    error.value = ''
    try {
      const { data } = await client.GET('/api/v1/projects/{projectId}/tasks/{taskId}', {
        params: { path: { projectId, taskId } },
      })
      if (data) {
        currentTask.value = data
      }
    } catch {
      error.value = 'Failed to load task.'
    } finally {
      loading.value = false
    }
  }

  async function createTask(
    projectId: string,
    taskTemplateId: string,
    ownerTagId: string,
    name: string,
    description?: string,
  ) {
    loading.value = true
    error.value = ''
    try {
      const { data } = await client.POST('/api/v1/projects/{projectId}/tasks', {
        params: { path: { projectId } },
        body: {
          taskTemplateId,
          ownerTagId,
          name,
          description: description ?? null,
        },
      })
      if (data) {
        await fetchTasks(projectId)
        return data
      }
      return null
    } catch {
      error.value = 'Failed to create task.'
      return null
    } finally {
      loading.value = false
    }
  }

  async function updateTask(
    projectId: string,
    taskId: string,
    body: { name?: string | null; description?: string | null },
  ) {
    loading.value = true
    error.value = ''
    try {
      const { data } = await client.PUT('/api/v1/projects/{projectId}/tasks/{taskId}', {
        params: { path: { projectId, taskId } },
        body,
      })
      if (data) {
        currentTask.value = data
      }
    } catch {
      error.value = 'Failed to update task.'
    } finally {
      loading.value = false
    }
  }

  async function updateTaskStatus(projectId: string, taskId: string, status: TaskStatus) {
    loading.value = true
    error.value = ''
    try {
      const { data } = await client.PUT('/api/v1/projects/{projectId}/tasks/{taskId}/status', {
        params: { path: { projectId, taskId } },
        body: { status },
      })
      if (data) {
        currentTask.value = data
        await fetchTasks(projectId)
      }
    } catch {
      error.value = 'Failed to update task status.'
    } finally {
      loading.value = false
    }
  }

  async function deleteTask(projectId: string, taskId: string) {
    loading.value = true
    error.value = ''
    try {
      await client.DELETE('/api/v1/projects/{projectId}/tasks/{taskId}', {
        params: { path: { projectId, taskId } },
      })
      await fetchTasks(projectId)
    } catch {
      error.value = 'Failed to delete task.'
    } finally {
      loading.value = false
    }
  }

  return {
    tasks,
    currentTask,
    loading,
    error,
    fetchTasks,
    getTask,
    createTask,
    updateTask,
    updateTaskStatus,
    deleteTask,
  }
})

import { ref } from 'vue'
import { defineStore } from 'pinia'
import client from '@/api/client'
import type { components } from '@/api/schema'

type TodoResponse = components['schemas']['TodoResponse']
type TodoStatus = components['schemas']['TodoStatus']
type TodoAssigneeResponse = components['schemas']['TodoAssigneeResponse']

export const useTodoStore = defineStore('todo', () => {
  const todos = ref<TodoResponse[]>([])
  const currentTodo = ref<TodoResponse | null>(null)
  const loading = ref(false)
  const error = ref('')

  async function fetchTodos(projectId: string, taskId: string) {
    loading.value = true
    error.value = ''
    try {
      const { data } = await client.GET(
        '/api/v1/projects/{projectId}/tasks/{taskId}/todos',
        { params: { path: { projectId, taskId } } },
      )
      if (data) {
        todos.value = data.todos
      }
    } catch {
      error.value = 'Failed to load todos.'
    } finally {
      loading.value = false
    }
  }

  async function createTodo(
    projectId: string,
    taskId: string,
    title: string,
    description?: string,
    parentId?: string,
  ) {
    loading.value = true
    error.value = ''
    try {
      const { data } = await client.POST(
        '/api/v1/projects/{projectId}/tasks/{taskId}/todos',
        {
          params: { path: { projectId, taskId } },
          body: {
            title,
            description: description ?? null,
            parentId: parentId ?? null,
          },
        },
      )
      if (data) {
        await fetchTodos(projectId, taskId)
        return data
      }
      return null
    } catch {
      error.value = 'Failed to create todo.'
      return null
    } finally {
      loading.value = false
    }
  }

  async function updateTodo(
    projectId: string,
    taskId: string,
    todoId: string,
    body: { title?: string | null; description?: string | null; dueDate?: string | null },
  ) {
    loading.value = true
    error.value = ''
    try {
      await client.PUT(
        '/api/v1/projects/{projectId}/tasks/{taskId}/todos/{todoId}',
        { params: { path: { projectId, taskId, todoId } }, body },
      )
      await fetchTodos(projectId, taskId)
    } catch {
      error.value = 'Failed to update todo.'
    } finally {
      loading.value = false
    }
  }

  async function updateTodoStatus(
    projectId: string,
    taskId: string,
    todoId: string,
    status: TodoStatus,
  ) {
    loading.value = true
    error.value = ''
    try {
      await client.PUT(
        '/api/v1/projects/{projectId}/tasks/{taskId}/todos/{todoId}/status',
        { params: { path: { projectId, taskId, todoId } }, body: { status } },
      )
      await fetchTodos(projectId, taskId)
    } catch {
      error.value = 'Failed to update todo status.'
    } finally {
      loading.value = false
    }
  }

  async function deleteTodo(projectId: string, taskId: string, todoId: string) {
    loading.value = true
    error.value = ''
    try {
      await client.DELETE(
        '/api/v1/projects/{projectId}/tasks/{taskId}/todos/{todoId}',
        { params: { path: { projectId, taskId, todoId } } },
      )
      await fetchTodos(projectId, taskId)
    } catch {
      error.value = 'Failed to delete todo.'
    } finally {
      loading.value = false
    }
  }

  async function addAssignee(
    projectId: string,
    taskId: string,
    todoId: string,
    memberId: string,
  ): Promise<TodoAssigneeResponse | null> {
    loading.value = true
    error.value = ''
    try {
      const { data } = await client.POST(
        '/api/v1/projects/{projectId}/tasks/{taskId}/todos/{todoId}/assignees',
        {
          params: { path: { projectId, taskId, todoId } },
          body: { memberId },
        },
      )
      return data ?? null
    } catch {
      error.value = 'Failed to add assignee.'
      return null
    } finally {
      loading.value = false
    }
  }

  async function removeAssignee(
    projectId: string,
    taskId: string,
    todoId: string,
    memberId: string,
  ) {
    loading.value = true
    error.value = ''
    try {
      await client.DELETE(
        '/api/v1/projects/{projectId}/tasks/{taskId}/todos/{todoId}/assignees/{memberId}',
        { params: { path: { projectId, taskId, todoId, memberId } } },
      )
    } catch {
      error.value = 'Failed to remove assignee.'
    } finally {
      loading.value = false
    }
  }

  return {
    todos,
    currentTodo,
    loading,
    error,
    fetchTodos,
    createTodo,
    updateTodo,
    updateTodoStatus,
    deleteTodo,
    addAssignee,
    removeAssignee,
  }
})

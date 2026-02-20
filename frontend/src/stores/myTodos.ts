import { ref } from 'vue'
import { defineStore } from 'pinia'
import client from '@/api/client'
import type { components } from '@/api/schema'

type MyTodoResponse = components['schemas']['MyTodoResponse']
type TodoStatus = components['schemas']['TodoStatus']

export const useMyTodosStore = defineStore('myTodos', () => {
  const items = ref<MyTodoResponse[]>([])
  const loading = ref(false)
  const error = ref('')

  async function fetchMyTodos(status?: TodoStatus, projectId?: string) {
    loading.value = true
    error.value = ''
    try {
      const query: Record<string, string> = {}
      if (status) query.status = status
      if (projectId) query.projectId = projectId

      const params = Object.keys(query).length > 0 ? { query } : {}
      const { data } = await client.GET('/api/v1/accounts/me/todos', {
        params,
      })
      if (data) {
        items.value = data.items
      }
    } catch {
      error.value = 'Failed to load my todos.'
    } finally {
      loading.value = false
    }
  }

  return {
    items,
    loading,
    error,
    fetchMyTodos,
  }
})

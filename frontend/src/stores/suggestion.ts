import { ref } from 'vue'
import { defineStore } from 'pinia'
import client from '@/api/client'
import type { components } from '@/api/schema'

export type SuggestionGroupItem = components['schemas']['SuggestionGroupItem']
export type SuggestionGroup = components['schemas']['SuggestionGroup']
export type Suggestion = components['schemas']['Suggestion']
export type SuggestionDecision = components['schemas']['SuggestionDecision']
export type DecideRequest = components['schemas']['DecideRequest']

export const useSuggestionStore = defineStore('suggestion', () => {
  const suggestionGroups = ref<SuggestionGroupItem[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)

  async function listSuggestions(projectId: string, taskId: string, cursor?: string) {
    loading.value = true
    error.value = null
    try {
      const query: Record<string, string> = { limit: '50' }
      if (cursor) query.cursor = cursor

      const { data } = await client.GET(
        '/api/v1/projects/{projectId}/tasks/{taskId}/suggestions' as never,
        { params: { path: { projectId, taskId }, query } } as never,
      )
      const result = data as { data: SuggestionGroupItem[] } | undefined
      if (result) {
        if (cursor) {
          suggestionGroups.value = [...suggestionGroups.value, ...result.data]
        } else {
          suggestionGroups.value = result.data
        }
      }
    } catch {
      error.value = 'Failed to load suggestions.'
    } finally {
      loading.value = false
    }
  }

  async function getSuggestionGroup(
    projectId: string,
    taskId: string,
    groupId: string,
  ): Promise<SuggestionGroupItem | null> {
    loading.value = true
    error.value = null
    try {
      const { data } = await client.GET(
        '/api/v1/projects/{projectId}/tasks/{taskId}/suggestions/{groupId}' as never,
        { params: { path: { projectId, taskId, groupId } } } as never,
      )
      const result = data as { messageId: string; suggestionGroup: SuggestionGroup } | undefined
      if (result) {
        const item: SuggestionGroupItem = {
          messageId: result.messageId,
          suggestionGroup: result.suggestionGroup,
        }
        return item
      }
      return null
    } catch {
      error.value = 'Failed to load suggestion group.'
      return null
    } finally {
      loading.value = false
    }
  }

  async function decideSuggestion(
    projectId: string,
    taskId: string,
    groupId: string,
    suggestionId: string,
    body: DecideRequest,
  ): Promise<boolean> {
    loading.value = true
    error.value = null
    try {
      await client.POST(
        '/api/v1/projects/{projectId}/tasks/{taskId}/suggestions/{groupId}/suggestions/{suggestionId}/decide' as never,
        {
          params: { path: { projectId, taskId, groupId, suggestionId } },
          body,
        } as never,
      )

      // Update local state: mark suggestion as decided
      for (const groupItem of suggestionGroups.value) {
        if (groupItem.suggestionGroup.id === groupId) {
          const suggestion = groupItem.suggestionGroup.suggestions.find(
            (s) => s.id === suggestionId,
          )
          if (suggestion) {
            suggestion.decision = body.decision
            suggestion.decidedAt = new Date().toISOString()
            if (body.modifiedParameters !== undefined) {
              suggestion.modifiedParameters = body.modifiedParameters
            }
          }
        }
      }

      return true
    } catch {
      error.value = 'Failed to record decision.'
      return false
    } finally {
      loading.value = false
    }
  }

  async function requestSuggestion(projectId: string, taskId: string): Promise<boolean> {
    loading.value = true
    error.value = null
    try {
      await client.POST(
        '/api/v1/projects/{projectId}/tasks/{taskId}/suggestions/request' as never,
        { params: { path: { projectId, taskId } } } as never,
      )
      return true
    } catch {
      error.value = 'Failed to request suggestion.'
      return false
    } finally {
      loading.value = false
    }
  }

  async function resolvePlaceholders(
    projectId: string,
    taskId: string,
    text: string,
  ): Promise<{ text: string; unresolved: string[] } | null> {
    try {
      const { data } = await client.POST(
        '/api/v1/projects/{projectId}/tasks/{taskId}/ai/resolve-placeholders' as never,
        {
          params: { path: { projectId, taskId } },
          body: { text },
        } as never,
      )
      const result = data as { text: string; unresolved: string[] } | undefined
      return result ?? null
    } catch {
      error.value = 'Failed to resolve placeholders.'
      return null
    }
  }

  function $reset() {
    suggestionGroups.value = []
    loading.value = false
    error.value = null
  }

  return {
    suggestionGroups,
    loading,
    error,
    listSuggestions,
    getSuggestionGroup,
    decideSuggestion,
    requestSuggestion,
    resolvePlaceholders,
    $reset,
  }
})

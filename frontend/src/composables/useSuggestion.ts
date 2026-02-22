import { storeToRefs } from 'pinia'
import { useSuggestionStore } from '@/stores/suggestion'
import type { SuggestionDecision } from '@/stores/suggestion'

export function useSuggestion(projectId: string, taskId: string) {
  const store = useSuggestionStore()
  const { suggestionGroups, loading, error } = storeToRefs(store)

  function loadSuggestions(cursor?: string) {
    return store.listSuggestions(projectId, taskId, cursor)
  }

  function decide(
    groupId: string,
    suggestionId: string,
    decision: SuggestionDecision,
    lastSeenMessageId: string,
    modifiedParameters?: unknown,
  ) {
    return store.decideSuggestion(projectId, taskId, groupId, suggestionId, {
      decision,
      lastSeenMessageId,
      modifiedParameters,
    })
  }

  function requestNewSuggestion() {
    return store.requestSuggestion(projectId, taskId)
  }

  function resolvePlaceholders(text: string) {
    return store.resolvePlaceholders(projectId, taskId, text)
  }

  return {
    suggestionGroups,
    loading,
    error,
    loadSuggestions,
    decide,
    requestNewSuggestion,
    resolvePlaceholders,
  }
}

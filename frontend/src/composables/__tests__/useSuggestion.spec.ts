import { describe, it, expect, vi, beforeEach } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { useSuggestion } from '../useSuggestion'
import { useSuggestionStore } from '@/stores/suggestion'

vi.mock('@/api/client', () => ({
  default: {
    GET: vi.fn(),
    POST: vi.fn(),
  },
}))

describe('useSuggestion', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('returns reactive refs from the store', () => {
    const { suggestionGroups, loading, error } = useSuggestion('project-1', 'task-1')
    expect(suggestionGroups.value).toEqual([])
    expect(loading.value).toBe(false)
    expect(error.value).toBeNull()
  })

  it('loadSuggestions delegates to store.listSuggestions', async () => {
    const { loadSuggestions } = useSuggestion('project-1', 'task-1')
    const store = useSuggestionStore()
    const spy = vi.spyOn(store, 'listSuggestions').mockResolvedValue(undefined)

    await loadSuggestions()
    expect(spy).toHaveBeenCalledWith('project-1', 'task-1', undefined)

    await loadSuggestions('cursor-abc')
    expect(spy).toHaveBeenCalledWith('project-1', 'task-1', 'cursor-abc')
  })

  it('decide delegates to store.decideSuggestion', async () => {
    const { decide } = useSuggestion('project-1', 'task-1')
    const store = useSuggestionStore()
    const spy = vi.spyOn(store, 'decideSuggestion').mockResolvedValue(true)

    await decide('group-1', 'sugg-1', 'accept', 'msg-last')
    expect(spy).toHaveBeenCalledWith('project-1', 'task-1', 'group-1', 'sugg-1', {
      decision: 'accept',
      lastSeenMessageId: 'msg-last',
      modifiedParameters: undefined,
    })
  })

  it('decide passes modifiedParameters when provided', async () => {
    const { decide } = useSuggestion('project-1', 'task-1')
    const store = useSuggestionStore()
    const spy = vi.spyOn(store, 'decideSuggestion').mockResolvedValue(true)

    const modified = { to: 'new@example.com' }
    await decide('group-1', 'sugg-1', 'modify_and_accept', 'msg-last', modified)
    expect(spy).toHaveBeenCalledWith('project-1', 'task-1', 'group-1', 'sugg-1', {
      decision: 'modify_and_accept',
      lastSeenMessageId: 'msg-last',
      modifiedParameters: modified,
    })
  })

  it('requestNewSuggestion delegates to store.requestSuggestion', async () => {
    const { requestNewSuggestion } = useSuggestion('project-1', 'task-1')
    const store = useSuggestionStore()
    const spy = vi.spyOn(store, 'requestSuggestion').mockResolvedValue(true)

    await requestNewSuggestion()
    expect(spy).toHaveBeenCalledWith('project-1', 'task-1')
  })

  it('resolvePlaceholders delegates to store.resolvePlaceholders', async () => {
    const { resolvePlaceholders } = useSuggestion('project-1', 'task-1')
    const store = useSuggestionStore()
    const resolved = { text: 'Hello Alice', unresolved: [] }
    const spy = vi.spyOn(store, 'resolvePlaceholders').mockResolvedValue(resolved)

    const result = await resolvePlaceholders('Hello {{profile.name}}')
    expect(spy).toHaveBeenCalledWith('project-1', 'task-1', 'Hello {{profile.name}}')
    expect(result).toEqual(resolved)
  })
})

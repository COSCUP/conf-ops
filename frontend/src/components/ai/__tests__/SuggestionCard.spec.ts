import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import SuggestionCard from '../SuggestionCard.vue'
import type { Suggestion } from '@/stores/suggestion'

// Mock SuggestionModifyDialog to avoid nested complexity in unit tests
vi.mock('../SuggestionModifyDialog.vue', () => ({
  default: {
    name: 'SuggestionModifyDialog',
    props: ['suggestion', 'show', 'projectId', 'taskId'],
    emits: ['confirm', 'close'],
    template: '<div class="mock-modify-dialog"><slot /></div>',
  },
}))

const pendingSuggestion: Suggestion = {
  id: 'sugg-1',
  tool: 'send_email',
  summary: 'Send a welcome email to the contact',
  reasoning: 'The task was recently created and an intro email is needed.',
  parameters: { to: 'test@example.com', subject: 'Welcome' },
  decision: 'pending',
  decidedAt: null,
  decidedBy: null,
  modifiedParameters: null,
  executionResult: null,
  contextUsed: [],
}

const acceptedSuggestion: Suggestion = {
  ...pendingSuggestion,
  id: 'sugg-2',
  decision: 'accept',
  decidedAt: '2025-06-01T10:00:00Z',
}

describe('SuggestionCard', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('renders tool name', () => {
    const wrapper = mount(SuggestionCard, {
      props: {
        suggestion: pendingSuggestion,
        groupId: 'group-1',
        messageId: 'msg-1',
        projectId: 'proj-1',
        taskId: 'task-1',
      },
    })
    expect(wrapper.text()).toContain('send_email')
  })

  it('renders suggestion summary', () => {
    const wrapper = mount(SuggestionCard, {
      props: {
        suggestion: pendingSuggestion,
        groupId: 'group-1',
        messageId: 'msg-1',
        projectId: 'proj-1',
        taskId: 'task-1',
      },
    })
    expect(wrapper.text()).toContain('Send a welcome email to the contact')
  })

  it('renders action buttons for pending suggestion', () => {
    const wrapper = mount(SuggestionCard, {
      props: {
        suggestion: pendingSuggestion,
        groupId: 'group-1',
        messageId: 'msg-1',
        projectId: 'proj-1',
        taskId: 'task-1',
      },
    })
    expect(wrapper.text()).toContain('Accept')
    expect(wrapper.text()).toContain('Modify')
    expect(wrapper.text()).toContain('Reject')
    expect(wrapper.text()).toContain('Re-suggest')
  })

  it('does not render action buttons for decided suggestion', () => {
    const wrapper = mount(SuggestionCard, {
      props: {
        suggestion: acceptedSuggestion,
        groupId: 'group-1',
        messageId: 'msg-1',
        projectId: 'proj-1',
        taskId: 'task-1',
      },
    })
    expect(wrapper.find('.suggestion-actions').exists()).toBe(false)
  })

  it('shows decision badge for decided suggestion', () => {
    const wrapper = mount(SuggestionCard, {
      props: {
        suggestion: acceptedSuggestion,
        groupId: 'group-1',
        messageId: 'msg-1',
        projectId: 'proj-1',
        taskId: 'task-1',
      },
    })
    expect(wrapper.find('.decision-badge').exists()).toBe(true)
    expect(wrapper.text()).toContain('Accepted')
  })

  it('does not show decision badge for pending suggestion', () => {
    const wrapper = mount(SuggestionCard, {
      props: {
        suggestion: pendingSuggestion,
        groupId: 'group-1',
        messageId: 'msg-1',
        projectId: 'proj-1',
        taskId: 'task-1',
      },
    })
    expect(wrapper.find('.decision-badge').exists()).toBe(false)
  })

  it('shows parameters section', () => {
    const wrapper = mount(SuggestionCard, {
      props: {
        suggestion: pendingSuggestion,
        groupId: 'group-1',
        messageId: 'msg-1',
        projectId: 'proj-1',
        taskId: 'task-1',
      },
    })
    expect(wrapper.find('.suggestion-parameters').exists()).toBe(true)
    expect(wrapper.text()).toContain('Parameters:')
  })

  it('toggles reasoning section on button click', async () => {
    const wrapper = mount(SuggestionCard, {
      props: {
        suggestion: pendingSuggestion,
        groupId: 'group-1',
        messageId: 'msg-1',
        projectId: 'proj-1',
        taskId: 'task-1',
      },
    })

    expect(wrapper.find('.reasoning-content').exists()).toBe(false)

    await wrapper.find('.reasoning-toggle').trigger('click')
    expect(wrapper.find('.reasoning-content').exists()).toBe(true)
    expect(wrapper.text()).toContain('The task was recently created')

    await wrapper.find('.reasoning-toggle').trigger('click')
    expect(wrapper.find('.reasoning-content').exists()).toBe(false)
  })

  it('emits decide with accept decision on Accept button click', async () => {
    const wrapper = mount(SuggestionCard, {
      props: {
        suggestion: pendingSuggestion,
        groupId: 'group-1',
        messageId: 'msg-1',
        projectId: 'proj-1',
        taskId: 'task-1',
      },
    })

    await wrapper.find('.action-btn--accept').trigger('click')

    const emitted = wrapper.emitted('decide')
    expect(emitted).toBeTruthy()
    expect(emitted![0]).toEqual([
      {
        groupId: 'group-1',
        suggestionId: 'sugg-1',
        decision: 'accept',
        messageId: 'msg-1',
      },
    ])
  })

  it('emits decide with reject decision on Reject button click', async () => {
    const wrapper = mount(SuggestionCard, {
      props: {
        suggestion: pendingSuggestion,
        groupId: 'group-1',
        messageId: 'msg-1',
        projectId: 'proj-1',
        taskId: 'task-1',
      },
    })

    await wrapper.find('.action-btn--reject').trigger('click')

    const emitted = wrapper.emitted('decide')
    expect(emitted).toBeTruthy()
    expect(emitted![0]).toEqual([
      {
        groupId: 'group-1',
        suggestionId: 'sugg-1',
        decision: 'reject',
        messageId: 'msg-1',
      },
    ])
  })

  it('shows re-suggest input on first click, emits on second click', async () => {
    const wrapper = mount(SuggestionCard, {
      props: {
        suggestion: pendingSuggestion,
        groupId: 'group-1',
        messageId: 'msg-1',
        projectId: 'proj-1',
        taskId: 'task-1',
      },
    })

    // First click shows the instruction input
    await wrapper.find('.action-btn--resuggest').trigger('click')
    expect(wrapper.emitted('decide')).toBeFalsy()
    expect(wrapper.find('.resuggest-input').exists()).toBe(true)

    // Fill in instructions
    await wrapper.find('.resuggest-textarea').setValue('Please suggest a phone call instead')

    // Second click sends the decision
    await wrapper.find('.action-btn--resuggest').trigger('click')

    const emitted = wrapper.emitted('decide')
    expect(emitted).toBeTruthy()
    expect(emitted![0]).toEqual([
      {
        groupId: 'group-1',
        suggestionId: 'sugg-1',
        decision: 're_suggest',
        messageId: 'msg-1',
        additionalInstructions: 'Please suggest a phone call instead',
      },
    ])
  })

  it('shows modify dialog when Modify button is clicked', async () => {
    const wrapper = mount(SuggestionCard, {
      props: {
        suggestion: pendingSuggestion,
        groupId: 'group-1',
        messageId: 'msg-1',
        projectId: 'proj-1',
        taskId: 'task-1',
      },
    })

    expect(wrapper.find('.mock-modify-dialog').exists()).toBe(false)

    await wrapper.find('.action-btn--modify').trigger('click')

    expect(wrapper.find('.mock-modify-dialog').exists()).toBe(true)
  })

  it('renders contextUsed when available', async () => {
    const suggestionWithContext: Suggestion = {
      ...pendingSuggestion,
      contextUsed: [
        { scopeType: 'organization', memoryId: 'mem-1', content: 'Always use formal tone' },
        { scopeType: 'task', memoryId: 'mem-2', content: 'Contact prefers email' },
      ],
    }
    const wrapper = mount(SuggestionCard, {
      props: {
        suggestion: suggestionWithContext,
        groupId: 'group-1',
        messageId: 'msg-1',
        projectId: 'proj-1',
        taskId: 'task-1',
      },
    })

    // Context section exists but collapsed by default
    expect(wrapper.find('.suggestion-context').exists()).toBe(true)
    expect(wrapper.find('.context-list').exists()).toBe(false)

    // Expand context
    await wrapper.findAll('.reasoning-toggle')[1]!.trigger('click')
    expect(wrapper.find('.context-list').exists()).toBe(true)
    expect(wrapper.text()).toContain('Always use formal tone')
    expect(wrapper.text()).toContain('organization')
  })

  it('shows decided timestamp when available', () => {
    const wrapper = mount(SuggestionCard, {
      props: {
        suggestion: acceptedSuggestion,
        groupId: 'group-1',
        messageId: 'msg-1',
        projectId: 'proj-1',
        taskId: 'task-1',
      },
    })
    expect(wrapper.find('.decided-at').exists()).toBe(true)
  })
})

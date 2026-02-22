import { describe, it, expect, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import SuggestionModifyDialog from '../SuggestionModifyDialog.vue'
import type { Suggestion } from '@/stores/suggestion'

const baseSuggestion: Suggestion = {
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

describe('SuggestionModifyDialog', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('does not render dialog when show is false', () => {
    const wrapper = mount(SuggestionModifyDialog, {
      props: { suggestion: baseSuggestion, show: false, projectId: 'proj-1', taskId: 'task-1' },
    })
    expect(wrapper.find('.dialog-overlay').exists()).toBe(false)
  })

  it('renders dialog when show is true', () => {
    const wrapper = mount(SuggestionModifyDialog, {
      props: { suggestion: baseSuggestion, show: true, projectId: 'proj-1', taskId: 'task-1' },
    })
    expect(wrapper.find('.dialog-overlay').exists()).toBe(true)
    expect(wrapper.find('.dialog').exists()).toBe(true)
  })

  it('displays tool name in header', () => {
    const wrapper = mount(SuggestionModifyDialog, {
      props: { suggestion: baseSuggestion, show: true, projectId: 'proj-1', taskId: 'task-1' },
    })
    expect(wrapper.find('.dialog-tool').text()).toBe('send_email')
  })

  it('populates textarea with formatted parameters JSON', () => {
    const wrapper = mount(SuggestionModifyDialog, {
      props: { suggestion: baseSuggestion, show: true, projectId: 'proj-1', taskId: 'task-1' },
    })
    const textarea = wrapper.find('.parameters-editor')
    const value = (textarea.element as HTMLTextAreaElement).value
    expect(value).toContain('"to"')
    expect(value).toContain('test@example.com')
    expect(value).toContain('"subject"')
    expect(value).toContain('Welcome')
  })

  it('emits confirm with parsed JSON on Confirm click', async () => {
    const wrapper = mount(SuggestionModifyDialog, {
      props: { suggestion: baseSuggestion, show: true, projectId: 'proj-1', taskId: 'task-1' },
    })

    const textarea = wrapper.find('.parameters-editor')
    await textarea.setValue('{"to":"other@example.com","subject":"Hi"}')

    await wrapper.find('.btn--primary').trigger('click')

    const emitted = wrapper.emitted('confirm')
    expect(emitted).toBeTruthy()
    expect(emitted![0]![0]).toEqual({ to: 'other@example.com', subject: 'Hi' })
  })

  it('shows JSON error message on invalid JSON confirm', async () => {
    const wrapper = mount(SuggestionModifyDialog, {
      props: { suggestion: baseSuggestion, show: true, projectId: 'proj-1', taskId: 'task-1' },
    })

    const textarea = wrapper.find('.parameters-editor')
    await textarea.setValue('not valid json')

    await wrapper.find('.btn--primary').trigger('click')

    expect(wrapper.find('.json-error').exists()).toBe(true)
    expect(wrapper.find('.json-error').text()).toContain('Invalid JSON')
    // Should not have emitted confirm
    expect(wrapper.emitted('confirm')).toBeUndefined()
  })

  it('emits close on Cancel click', async () => {
    const wrapper = mount(SuggestionModifyDialog, {
      props: { suggestion: baseSuggestion, show: true, projectId: 'proj-1', taskId: 'task-1' },
    })

    await wrapper.find('.btn--secondary').trigger('click')

    expect(wrapper.emitted('close')).toBeTruthy()
  })

  it('emits close when clicking overlay background', async () => {
    const wrapper = mount(SuggestionModifyDialog, {
      props: { suggestion: baseSuggestion, show: true, projectId: 'proj-1', taskId: 'task-1' },
    })

    await wrapper.find('.dialog-overlay').trigger('click')

    expect(wrapper.emitted('close')).toBeTruthy()
  })

  it('resets JSON error when dialog re-opens', async () => {
    const wrapper = mount(SuggestionModifyDialog, {
      props: { suggestion: baseSuggestion, show: true, projectId: 'proj-1', taskId: 'task-1' },
    })

    // Cause an error
    const textarea = wrapper.find('.parameters-editor')
    await textarea.setValue('invalid')
    await wrapper.find('.btn--primary').trigger('click')
    expect(wrapper.find('.json-error').exists()).toBe(true)

    // Simulate dialog close and re-open by toggling show prop
    await wrapper.setProps({ show: false })
    await wrapper.setProps({ show: true })
    expect(wrapper.find('.json-error').exists()).toBe(false)
  })
})

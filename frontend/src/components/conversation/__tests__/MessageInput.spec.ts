import { describe, it, expect, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import MessageInput from '../MessageInput.vue'
import type { MentionMember } from '../MessageInput.vue'

const defaultMembers: MentionMember[] = [
  { id: 'user-1', displayName: 'Alice' },
  { id: 'user-2', displayName: 'Bob' },
  { id: 'user-3', displayName: 'Alice Smith' },
]

function createWrapper(disabled = false, members: MentionMember[] = []) {
  return mount(MessageInput, {
    props: { disabled, members },
  })
}

describe('MessageInput', () => {
  it('renders textarea and send button', () => {
    const wrapper = createWrapper()
    expect(wrapper.find('textarea').exists()).toBe(true)
    expect(wrapper.text()).toContain('Send')
  })

  it('emits send with trimmed text and empty mentions on form submit', async () => {
    const wrapper = createWrapper()
    const textarea = wrapper.find('textarea')
    await textarea.setValue('  Hello  ')

    const form = wrapper.find('form')
    await form.trigger('submit')

    expect(wrapper.emitted('send')).toHaveLength(1)
    expect(wrapper.emitted('send')![0]).toEqual(['Hello', [], []])
  })

  it('clears input after send', async () => {
    const wrapper = createWrapper()
    const textarea = wrapper.find('textarea')
    await textarea.setValue('Hello')

    const form = wrapper.find('form')
    await form.trigger('submit')

    expect((textarea.element as HTMLTextAreaElement).value).toBe('')
  })

  it('does not emit send when text is empty', async () => {
    const wrapper = createWrapper()
    const form = wrapper.find('form')
    await form.trigger('submit')

    expect(wrapper.emitted('send')).toBeUndefined()
  })

  it('disables textarea when disabled prop is true', () => {
    const wrapper = createWrapper(true)
    const textarea = wrapper.find('textarea')
    expect((textarea.element as HTMLTextAreaElement).disabled).toBe(true)
  })

  it('emits typing on input', async () => {
    vi.useFakeTimers()
    const wrapper = createWrapper()
    const textarea = wrapper.find('textarea')
    await textarea.setValue('H')
    await textarea.trigger('input')

    expect(wrapper.emitted('typing')?.[0]).toEqual([true])

    vi.advanceTimersByTime(2000)
    const typingEvents = wrapper.emitted('typing')!
    expect(typingEvents[typingEvents.length - 1]).toEqual([false])
    vi.useRealTimers()
  })

  it('renders mention dropdown when @ is typed and members are provided', async () => {
    const wrapper = createWrapper(false, defaultMembers)
    const textarea = wrapper.find('textarea')

    await textarea.setValue('@')
    await textarea.trigger('input')

    const dropdown = wrapper.find('.mention-dropdown')
    expect(dropdown.exists()).toBe(true)
    const items = wrapper.findAll('.mention-item')
    expect(items).toHaveLength(3)
    expect(items[0]!.text()).toBe('Alice')
    expect(items[1]!.text()).toBe('Bob')
    expect(items[2]!.text()).toBe('Alice Smith')
  })

  it('filters mention dropdown based on typed text after @', async () => {
    const wrapper = createWrapper(false, defaultMembers)
    const textarea = wrapper.find('textarea')

    await textarea.setValue('@ali')
    await textarea.trigger('input')

    const items = wrapper.findAll('.mention-item')
    expect(items).toHaveLength(2)
    expect(items[0]!.text()).toBe('Alice')
    expect(items[1]!.text()).toBe('Alice Smith')
  })

  it('does not show dropdown when no members match the query', async () => {
    const wrapper = createWrapper(false, defaultMembers)
    const textarea = wrapper.find('textarea')

    await textarea.setValue('@xyz')
    await textarea.trigger('input')

    const dropdown = wrapper.find('.mention-dropdown')
    expect(dropdown.exists()).toBe(false)
  })

  it('does not show dropdown when no members prop is provided', async () => {
    const wrapper = createWrapper(false, [])
    const textarea = wrapper.find('textarea')

    await textarea.setValue('@')
    await textarea.trigger('input')

    const dropdown = wrapper.find('.mention-dropdown')
    expect(dropdown.exists()).toBe(false)
  })

  it('inserts selected member name into text on mention selection', async () => {
    const wrapper = createWrapper(false, defaultMembers)
    const textarea = wrapper.find('textarea')

    await textarea.setValue('@Ali')
    await textarea.trigger('input')

    const firstItem = wrapper.find('.mention-item')
    await firstItem.trigger('mousedown')

    expect((textarea.element as HTMLTextAreaElement).value).toBe('@Alice ')
  })

  it('emits send with mention payload when a member is selected and message is sent', async () => {
    const wrapper = createWrapper(false, defaultMembers)
    const textarea = wrapper.find('textarea')

    await textarea.setValue('@Ali')
    await textarea.trigger('input')

    const firstItem = wrapper.find('.mention-item')
    await firstItem.trigger('mousedown')

    const form = wrapper.find('form')
    await form.trigger('submit')

    expect(wrapper.emitted('send')).toHaveLength(1)
    const [sentText, sentMentions] = wrapper.emitted('send')![0] as [
      string,
      { type: 'member'; id: string }[],
    ]
    expect(sentText).toBe('@Alice')
    expect(sentMentions).toEqual([{ type: 'member', id: 'user-1' }])
  })

  it('hides dropdown after selecting a mention', async () => {
    const wrapper = createWrapper(false, defaultMembers)
    const textarea = wrapper.find('textarea')

    await textarea.setValue('@')
    await textarea.trigger('input')

    expect(wrapper.find('.mention-dropdown').exists()).toBe(true)

    const firstItem = wrapper.find('.mention-item')
    await firstItem.trigger('mousedown')

    expect(wrapper.find('.mention-dropdown').exists()).toBe(false)
  })
})

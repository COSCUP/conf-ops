import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import MessageItem from '../MessageItem.vue'
import type { MessageResponse } from '@/stores/conversation'

function createMessage(overrides?: Partial<MessageResponse>): MessageResponse {
  return {
    id: 'msg-1',
    taskId: 'task-1',
    sourceType: 'member',
    sourceId: 'member-1',
    content: { text: 'Hello world', mentions: [] },
    attachments: null,
    actionResult: null,
    lastSeenMessageId: null,
    createdAt: '2025-06-01T10:30:00Z',
    ...overrides,
  }
}

describe('MessageItem', () => {
  it('renders message text', () => {
    const wrapper = mount(MessageItem, {
      props: { message: createMessage(), isUnread: false, projectId: 'proj-1', taskId: 'task-1' },
    })
    expect(wrapper.text()).toContain('Hello world')
  })

  it('renders source id for member messages', () => {
    const wrapper = mount(MessageItem, {
      props: { message: createMessage({ sourceId: 'user-abc' }), isUnread: false, projectId: 'proj-1', taskId: 'task-1' },
    })
    expect(wrapper.text()).toContain('user-abc')
  })

  it('renders "System" for system messages', () => {
    const wrapper = mount(MessageItem, {
      props: {
        message: createMessage({ sourceType: 'system', sourceId: null }),
        isUnread: false,
        projectId: 'proj-1',
        taskId: 'task-1',
      },
    })
    expect(wrapper.text()).toContain('System')
  })

  it('applies message-unread class when unread', () => {
    const wrapper = mount(MessageItem, {
      props: { message: createMessage(), isUnread: true, projectId: 'proj-1', taskId: 'task-1' },
    })
    expect(wrapper.find('.message-unread').exists()).toBe(true)
  })

  it('does not apply message-unread class when read', () => {
    const wrapper = mount(MessageItem, {
      props: { message: createMessage(), isUnread: false, projectId: 'proj-1', taskId: 'task-1' },
    })
    expect(wrapper.find('.message-unread').exists()).toBe(false)
  })

  it('applies message-system class for system messages', () => {
    const wrapper = mount(MessageItem, {
      props: {
        message: createMessage({ sourceType: 'system' }),
        isUnread: false,
        projectId: 'proj-1',
        taskId: 'task-1',
      },
    })
    expect(wrapper.find('.message-system').exists()).toBe(true)
  })

  it('handles content without text field', () => {
    const wrapper = mount(MessageItem, {
      props: {
        message: createMessage({ content: { action: 'status_change' } }),
        isUnread: false,
        projectId: 'proj-1',
        taskId: 'task-1',
      },
    })
    expect(wrapper.find('.message-body').text()).toBe('')
  })

  it('renders system message with event and details', () => {
    const wrapper = mount(MessageItem, {
      props: {
        message: createMessage({
          sourceType: 'system',
          sourceId: null,
          content: { event: 'task_status_changed', details: { from: 'open', to: 'in_progress' } },
        }),
        isUnread: false,
        projectId: 'proj-1',
        taskId: 'task-1',
      },
    })
    expect(wrapper.find('.message-body').text()).toContain('[task_status_changed]')
    expect(wrapper.find('.message-body').text()).toContain('open')
  })

  it('renders tool_execution message with tool name and status', () => {
    const wrapper = mount(MessageItem, {
      props: {
        message: createMessage({
          sourceType: 'tool_execution',
          content: { toolName: 'send_email', status: 'success', parameters: {}, result: {} },
        }),
        isUnread: false,
        projectId: 'proj-1',
        taskId: 'task-1',
      },
    })
    expect(wrapper.find('.message-body').text()).toContain('[Tool: send_email]')
    expect(wrapper.find('.message-body').text()).toContain('success')
  })
})

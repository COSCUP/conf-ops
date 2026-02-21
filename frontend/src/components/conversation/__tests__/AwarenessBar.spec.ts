import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import AwarenessBar from '../AwarenessBar.vue'
import type { AwarenessEntry } from '@/composables/useWebSocket'

function makeEntry(overrides?: Partial<AwarenessEntry>): AwarenessEntry {
  return {
    memberId: 'member-1',
    displayName: 'Alice',
    color: '#3b82f6',
    cursorPosition: null,
    isTyping: false,
    ...overrides,
  }
}

describe('AwarenessBar', () => {
  it('shows online count when entries exist', () => {
    const wrapper = mount(AwarenessBar, {
      props: { entries: [makeEntry()] },
    })
    expect(wrapper.text()).toContain('1 online')
  })

  it('renders user dots for each entry', () => {
    const wrapper = mount(AwarenessBar, {
      props: {
        entries: [
          makeEntry({ memberId: 'm1', displayName: 'Alice' }),
          makeEntry({ memberId: 'm2', displayName: 'Bob', color: '#ef4444' }),
        ],
      },
    })
    expect(wrapper.findAll('.user-dot')).toHaveLength(2)
    expect(wrapper.text()).toContain('2 online')
  })

  it('shows typing indicator for one user', () => {
    const wrapper = mount(AwarenessBar, {
      props: { entries: [makeEntry({ isTyping: true, displayName: 'Alice' })] },
    })
    expect(wrapper.text()).toContain('Alice is typing...')
  })

  it('shows typing indicator for two users', () => {
    const wrapper = mount(AwarenessBar, {
      props: {
        entries: [
          makeEntry({ memberId: 'm1', displayName: 'Alice', isTyping: true }),
          makeEntry({ memberId: 'm2', displayName: 'Bob', isTyping: true }),
        ],
      },
    })
    expect(wrapper.text()).toContain('Alice and Bob are typing...')
  })

  it('shows typing indicator for many users', () => {
    const wrapper = mount(AwarenessBar, {
      props: {
        entries: [
          makeEntry({ memberId: 'm1', displayName: 'Alice', isTyping: true }),
          makeEntry({ memberId: 'm2', displayName: 'Bob', isTyping: true }),
          makeEntry({ memberId: 'm3', displayName: 'Charlie', isTyping: true }),
        ],
      },
    })
    expect(wrapper.text()).toContain('Alice and 2 others are typing...')
  })

  it('shows nothing when no entries', () => {
    const wrapper = mount(AwarenessBar, {
      props: { entries: [] },
    })
    expect(wrapper.find('.online-users').exists()).toBe(false)
    expect(wrapper.find('.typing-indicator').exists()).toBe(false)
  })

  it('does not show typing when no one is typing', () => {
    const wrapper = mount(AwarenessBar, {
      props: { entries: [makeEntry({ isTyping: false })] },
    })
    expect(wrapper.find('.typing-indicator').exists()).toBe(false)
  })
})

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import NotificationBell from '../NotificationBell.vue'
import { useNotificationStore } from '@/stores/notification'

vi.mock('@/api/client', () => ({
  default: {
    GET: vi.fn().mockResolvedValue({ data: { count: 0 } }),
    PUT: vi.fn(),
    POST: vi.fn(),
    DELETE: vi.fn(),
  },
}))

describe('NotificationBell', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.useFakeTimers()
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it('renders bell button', () => {
    const wrapper = mount(NotificationBell)
    expect(wrapper.find('.notification-bell').exists()).toBe(true)
    expect(wrapper.find('.bell-icon').exists()).toBe(true)
  })

  it('does not show badge when unread count is 0', () => {
    const store = useNotificationStore()
    store.unreadCount = 0
    const wrapper = mount(NotificationBell)
    expect(wrapper.find('.badge').exists()).toBe(false)
  })

  it('shows badge with unread count', async () => {
    const store = useNotificationStore()
    store.unreadCount = 5
    const wrapper = mount(NotificationBell)
    await wrapper.vm.$nextTick()
    expect(wrapper.find('.badge').exists()).toBe(true)
    expect(wrapper.find('.badge').text()).toBe('5')
  })

  it('shows 99+ when unread count exceeds 99', async () => {
    const store = useNotificationStore()
    store.unreadCount = 150
    const wrapper = mount(NotificationBell)
    await wrapper.vm.$nextTick()
    expect(wrapper.find('.badge').text()).toBe('99+')
  })

  it('emits toggle on click', async () => {
    const wrapper = mount(NotificationBell)
    await wrapper.find('.notification-bell').trigger('click')
    expect(wrapper.emitted('toggle')).toBeTruthy()
    expect(wrapper.emitted('toggle')).toHaveLength(1)
  })
})

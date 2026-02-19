import { describe, it, expect, beforeEach, vi } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { createRouter, createMemoryHistory } from 'vue-router'
import AccountSettingsView from '../AccountSettingsView.vue'
import { useAuthStore } from '@/stores/auth'

vi.mock('@/api/client', () => ({
  default: {
    GET: vi.fn(),
    POST: vi.fn(),
    PUT: vi.fn(),
    PATCH: vi.fn(),
    DELETE: vi.fn(),
    use: vi.fn(),
  },
  setupAuthInterceptor: vi.fn(),
}))

import client from '@/api/client'

function createTestRouter() {
  return createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: '/settings', name: 'settings', component: { template: '<div />' } },
    ],
  })
}

describe('AccountSettingsView', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()

    vi.mocked(client.GET).mockImplementation((url: string) => {
      if (String(url).includes('passkeys')) {
        return Promise.resolve({ data: { passkeys: [] } }) as never
      }
      return Promise.resolve({
        data: {
          channels: {
            email: {
              enabled: true,
              categories: {
                taskUpdates: true,
                todoAssignments: true,
                aiSuggestions: true,
                mentions: true,
                systemAnnouncements: true,
              },
            },
            webPush: {
              enabled: true,
              categories: {
                taskUpdates: true,
                todoAssignments: true,
                aiSuggestions: true,
                mentions: true,
                systemAnnouncements: true,
              },
            },
            inApp: {
              enabled: true,
              categories: {
                taskUpdates: true,
                todoAssignments: true,
                aiSuggestions: true,
                mentions: true,
                systemAnnouncements: true,
              },
            },
          },
        },
      }) as never
    })
  })

  it('renders settings sections', async () => {
    const store = useAuthStore()
    store.setTokens('token')
    store.$patch({
      currentUser: {
        id: '1',
        email: 'a@b.c',
        name: 'Test User',
        avatarUrl: null,
        bio: null,
        locale: 'en',
      },
    })

    const router = createTestRouter()
    const wrapper = mount(AccountSettingsView, {
      global: { plugins: [router] },
    })

    await flushPromises()

    expect(wrapper.text()).toContain('Account Settings')
    expect(wrapper.text()).toContain('Display Name')
    expect(wrapper.text()).toContain('Passkeys')
    expect(wrapper.text()).toContain('Notification Preferences')
  })

  it('shows empty state when no passkeys', async () => {
    vi.mocked(client.GET).mockImplementation((url: string) => {
      if (String(url).includes('passkeys')) {
        return Promise.resolve({ data: { passkeys: [] } }) as never
      }
      return Promise.resolve({
        data: {
          channels: {
            email: {
              enabled: true,
              categories: {
                taskUpdates: true,
                todoAssignments: true,
                aiSuggestions: true,
                mentions: true,
                systemAnnouncements: true,
              },
            },
            webPush: {
              enabled: true,
              categories: {
                taskUpdates: true,
                todoAssignments: true,
                aiSuggestions: true,
                mentions: true,
                systemAnnouncements: true,
              },
            },
            inApp: {
              enabled: true,
              categories: {
                taskUpdates: true,
                todoAssignments: true,
                aiSuggestions: true,
                mentions: true,
                systemAnnouncements: true,
              },
            },
          },
        },
      }) as never
    })

    const router = createTestRouter()
    const wrapper = mount(AccountSettingsView, {
      global: { plugins: [router] },
    })

    await flushPromises()

    expect(wrapper.text()).toContain('No passkeys registered')
  })
})

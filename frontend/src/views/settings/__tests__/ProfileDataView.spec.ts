import { describe, it, expect, beforeEach, vi } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { createRouter, createMemoryHistory } from 'vue-router'
import ProfileDataView from '../ProfileDataView.vue'

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
      { path: '/settings/profile', name: 'settings-profile', component: { template: '<div />' } },
    ],
  })
}

describe('ProfileDataView', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('shows empty schema message when no fields configured', async () => {
    vi.mocked(client.GET).mockResolvedValue({
      data: {
        profileData: {},
        profileSchema: [],
      },
    } as never)

    const router = createTestRouter()
    const wrapper = mount(ProfileDataView, {
      global: { plugins: [router] },
    })

    await flushPromises()

    expect(wrapper.text()).toContain('No profile fields have been configured yet')
  })

  it('renders dynamic fields based on profileSchema', async () => {
    vi.mocked(client.GET).mockResolvedValue({
      data: {
        profileData: { nickname: 'Alice', website: 'https://example.com' },
        profileSchema: [
          { key: 'nickname', label: 'Nickname', description: 'Your nickname', type: 'text' },
          { key: 'website', label: 'Website', description: 'Your website URL', type: 'url' },
        ],
      },
    } as never)

    const router = createTestRouter()
    const wrapper = mount(ProfileDataView, {
      global: { plugins: [router] },
    })

    await flushPromises()

    expect(wrapper.text()).toContain('Nickname')
    expect(wrapper.text()).toContain('Website')
    expect(wrapper.find('form').exists()).toBe(true)
  })

  it('calls updateProfile on save', async () => {
    vi.mocked(client.GET).mockResolvedValue({
      data: {
        profileData: { bio: 'Hello' },
        profileSchema: [
          { key: 'bio', label: 'Bio', description: 'About you', type: 'text' },
        ],
      },
    } as never)
    vi.mocked(client.PUT).mockResolvedValue({
      data: {
        profileData: { bio: 'Hello' },
        profileSchema: [
          { key: 'bio', label: 'Bio', description: 'About you', type: 'text' },
        ],
      },
    } as never)

    const router = createTestRouter()
    const wrapper = mount(ProfileDataView, {
      global: { plugins: [router] },
    })

    await flushPromises()

    await wrapper.find('form').trigger('submit')
    await flushPromises()

    expect(client.PUT).toHaveBeenCalled()
  })
})

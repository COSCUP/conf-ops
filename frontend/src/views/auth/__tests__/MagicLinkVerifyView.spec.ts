import { describe, it, expect, beforeEach, vi } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { createRouter, createMemoryHistory, type Router } from 'vue-router'
import MagicLinkVerifyView from '../MagicLinkVerifyView.vue'

vi.mock('@/api/client', () => ({
  default: {
    GET: vi.fn(),
    POST: vi.fn(),
    use: vi.fn(),
  },
  setupAuthInterceptor: vi.fn(),
}))

import client from '@/api/client'

function createTestRouter(): Router {
  return createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: '/auth/magic-link', name: 'magic-link-verify', component: MagicLinkVerifyView },
      { path: '/login', name: 'login', component: { template: '<div>Login</div>' } },
      { path: '/', name: 'dashboard', component: { template: '<div>Dashboard</div>' } },
    ],
  })
}

describe('MagicLinkVerifyView', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('shows error when no token in query', async () => {
    const router = createTestRouter()
    await router.push('/auth/magic-link')
    await router.isReady()

    const wrapper = mount(MagicLinkVerifyView, {
      global: { plugins: [router] },
    })

    await flushPromises()
    expect(wrapper.text()).toContain('Invalid or missing token')
  })

  it('verifies token and redirects on success', async () => {
    vi.mocked(client.GET).mockResolvedValueOnce({
      data: { accessToken: 'new-token', tokenType: 'Bearer', expiresIn: 900 },
    } as never).mockResolvedValueOnce({
      data: {
        id: '1',
        email: 'a@b.c',
        name: 'A',
        avatarUrl: null,
        bio: null,
        locale: 'en',
        createdAt: '2025-01-01T00:00:00Z',
        updatedAt: '2025-01-01T00:00:00Z',
      },
    } as never)

    const router = createTestRouter()
    await router.push('/auth/magic-link?token=valid-token')
    await router.isReady()

    mount(MagicLinkVerifyView, {
      global: { plugins: [router] },
    })

    await flushPromises()
    expect(router.currentRoute.value.name).toBe('dashboard')
  })

  it('shows error on verification failure', async () => {
    vi.mocked(client.GET).mockResolvedValue({ data: undefined } as never)

    const router = createTestRouter()
    await router.push('/auth/magic-link?token=bad-token')
    await router.isReady()

    const wrapper = mount(MagicLinkVerifyView, {
      global: { plugins: [router] },
    })

    await flushPromises()
    expect(wrapper.text()).toContain('invalid or has expired')
  })
})

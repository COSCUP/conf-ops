import { describe, it, expect, beforeEach, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { createRouter, createMemoryHistory } from 'vue-router'
import LoginView from '../LoginView.vue'

vi.mock('@/api/client', () => ({
  default: {
    GET: vi.fn(),
    POST: vi.fn(),
    use: vi.fn(),
  },
  setupAuthInterceptor: vi.fn(),
}))

vi.mock('@simplewebauthn/browser', () => ({
  startAuthentication: vi.fn(),
}))

import client from '@/api/client'

function createTestRouter() {
  return createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: '/login', name: 'login', component: { template: '<div />' } },
      { path: '/', name: 'dashboard', component: { template: '<div />' } },
    ],
  })
}

describe('LoginView', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('renders email input and login buttons', () => {
    const router = createTestRouter()
    const wrapper = mount(LoginView, {
      global: { plugins: [router] },
    })

    expect(wrapper.find('input[type="email"]').exists()).toBe(true)
    expect(wrapper.text()).toContain('Send magic link')
    expect(wrapper.text()).toContain('Sign in with Passkey')
  })

  it('shows error when submitting empty email', async () => {
    const router = createTestRouter()
    const wrapper = mount(LoginView, {
      global: { plugins: [router] },
    })

    await wrapper.find('form').trigger('submit')
    expect(wrapper.text()).toContain('Please enter your email address')
  })

  it('sends magic link and shows confirmation', async () => {
    vi.mocked(client.POST).mockResolvedValue({ data: {} } as never)

    const router = createTestRouter()
    const wrapper = mount(LoginView, {
      global: { plugins: [router] },
    })

    await wrapper.find('input[type="email"]').setValue('user@example.com')
    await wrapper.find('form').trigger('submit')
    await vi.dynamicImportSettled()

    expect(wrapper.text()).toContain('Check your email')
    expect(wrapper.text()).toContain('user@example.com')
  })

  it('shows error on magic link failure', async () => {
    vi.mocked(client.POST).mockRejectedValue(new Error('fail'))

    const router = createTestRouter()
    const wrapper = mount(LoginView, {
      global: { plugins: [router] },
    })

    await wrapper.find('input[type="email"]').setValue('user@example.com')
    await wrapper.find('form').trigger('submit')
    await vi.dynamicImportSettled()

    expect(wrapper.text()).toContain('Failed to send magic link')
  })
})

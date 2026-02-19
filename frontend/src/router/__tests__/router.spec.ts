import { describe, it, expect, beforeEach, vi } from 'vitest'
import { createRouter, createWebHistory, type Router } from 'vue-router'
import { createPinia, setActivePinia } from 'pinia'
import { useAuthStore } from '@/stores/auth'

vi.mock('@/api/client', () => ({
  default: {
    GET: vi.fn(),
    POST: vi.fn(),
    use: vi.fn(),
  },
  setupAuthInterceptor: vi.fn(),
}))

function createTestRouter(): Router {
  const router = createRouter({
    history: createWebHistory(),
    routes: [
      {
        path: '/login',
        name: 'login',
        component: { template: '<div>Login</div>' },
        meta: { requiresAuth: false },
      },
      {
        path: '/',
        name: 'dashboard',
        component: { template: '<div>Dashboard</div>' },
        meta: { requiresAuth: true },
      },
    ],
  })

  router.beforeEach((to) => {
    const store = useAuthStore()
    const requiresAuth = to.matched.some((record) => record.meta.requiresAuth === true)
    if (requiresAuth && !store.isAuthenticated) {
      return { name: 'login' }
    }
    return true
  })

  return router
}

describe('Router guard', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('redirects to login when not authenticated', async () => {
    const router = createTestRouter()
    await router.push('/')
    await router.isReady()
    expect(router.currentRoute.value.name).toBe('login')
  })

  it('allows access to login page when not authenticated', async () => {
    const router = createTestRouter()
    await router.push('/login')
    await router.isReady()
    expect(router.currentRoute.value.name).toBe('login')
  })

  it('allows access to protected route when authenticated', async () => {
    const store = useAuthStore()
    store.setTokens('test-token')

    const router = createTestRouter()
    await router.push('/')
    await router.isReady()
    expect(router.currentRoute.value.name).toBe('dashboard')
  })
})

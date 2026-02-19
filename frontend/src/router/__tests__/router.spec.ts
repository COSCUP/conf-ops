import { describe, it, expect, beforeEach } from 'vitest'
import { createRouter, createWebHistory, type Router } from 'vue-router'
import { useAuth } from '@/composables/useAuth'

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
    const { checkAuth } = useAuth()
    const requiresAuth = to.matched.some((record) => record.meta.requiresAuth !== false)
    if (requiresAuth && !checkAuth()) {
      return { name: 'login' }
    }
    return true
  })

  return router
}

describe('Router guard', () => {
  beforeEach(() => {
    const { setAuthenticated } = useAuth()
    setAuthenticated(false)
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
    const { setAuthenticated } = useAuth()
    setAuthenticated(true)

    const router = createTestRouter()
    await router.push('/')
    await router.isReady()
    expect(router.currentRoute.value.name).toBe('dashboard')
  })
})

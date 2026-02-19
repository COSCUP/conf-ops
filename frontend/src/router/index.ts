import { createRouter, createWebHistory } from 'vue-router'
import { useAuthStore } from '@/stores/auth'

const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  routes: [
    {
      path: '/login',
      component: () => import('@/layouts/AuthLayout.vue'),
      children: [
        {
          path: '',
          name: 'login',
          component: () => import('@/views/LoginView.vue'),
        },
      ],
      meta: { requiresAuth: false },
    },
    {
      path: '/auth/magic-link',
      component: () => import('@/layouts/AuthLayout.vue'),
      children: [
        {
          path: '',
          name: 'magic-link-verify',
          component: () => import('@/views/auth/MagicLinkVerifyView.vue'),
        },
      ],
      meta: { requiresAuth: false },
    },
    {
      path: '/',
      component: () => import('@/layouts/AppLayout.vue'),
      meta: { requiresAuth: true },
      children: [
        {
          path: '',
          name: 'dashboard',
          component: () => import('@/views/DashboardView.vue'),
        },
        {
          path: 'projects',
          name: 'projects',
          component: () => import('@/views/DashboardView.vue'),
        },
        {
          path: 'settings',
          name: 'settings',
          component: () => import('@/views/DashboardView.vue'),
        },
      ],
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

export default router

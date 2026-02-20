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
          component: () => import('@/views/auth/LoginView.vue'),
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
          path: 'organizations',
          name: 'organizations',
          component: () => import('@/views/organizations/OrganizationListView.vue'),
        },
        {
          path: 'organizations/:orgId/settings',
          name: 'org-settings',
          component: () => import('@/views/organizations/OrganizationSettingsView.vue'),
        },
        {
          path: 'organizations/:orgId/projects',
          name: 'org-projects',
          component: () => import('@/views/projects/ProjectListView.vue'),
        },
        {
          path: 'projects/:projectId',
          name: 'project-dashboard',
          component: () => import('@/views/projects/ProjectDashboardView.vue'),
        },
        {
          path: 'projects/:projectId/settings',
          name: 'project-settings',
          component: () => import('@/views/projects/ProjectSettingsView.vue'),
        },
        {
          path: 'projects/:projectId/members',
          name: 'project-members',
          component: () => import('@/views/projects/ProjectMembersView.vue'),
        },
        {
          path: 'projects/:projectId/member-tags',
          name: 'project-member-tags',
          component: () => import('@/views/projects/MemberTagsView.vue'),
        },
        {
          path: 'projects/:projectId/contacts',
          name: 'project-contacts',
          component: () => import('@/views/projects/ProjectContactsView.vue'),
        },
        {
          path: 'projects/:projectId/task-templates',
          name: 'project-task-templates',
          component: () => import('@/views/projects/TaskTemplatesView.vue'),
        },
        {
          path: 'projects/:projectId/task-templates/:templateId',
          name: 'task-template-editor',
          component: () => import('@/views/projects/TaskTemplateEditorView.vue'),
        },
        {
          path: 'organizations/:orgId/contacts',
          name: 'org-contacts',
          component: () => import('@/views/organizations/ContactsView.vue'),
        },
        {
          path: 'settings',
          name: 'settings',
          component: () => import('@/views/settings/AccountSettingsView.vue'),
        },
        {
          path: 'settings/profile',
          name: 'settings-profile',
          component: () => import('@/views/settings/ProfileDataView.vue'),
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

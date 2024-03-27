import { RouteRecordRaw, createRouter, createWebHistory } from 'vue-router'
import HomeView from '@/views/HomeView.vue'
import LoginView from '@/views/LoginView.vue'
import TokenView from '@/views/TokenView.vue'
import SystemView from '@/views/SystemView.vue'
import DashboardView from '@/views/system/DashboardView.vue'

const routes: RouteRecordRaw[] = [
  { path: '/', component: HomeView },
  { path: '/login/:projectId/:roleId?', component: LoginView, props: true },
  { path: '/token/:token', component: TokenView, props: true },
  {
    path: '/system',
    component: SystemView,
    children: [
      { path: '', component: DashboardView },
      { path: 'manage-role', component: () => import('@/views/system/RoleManageView.vue') },
      { path: 'ticket', component: () => import('@/views/system/ticket/TicketView.vue') },
      {
        path: 'ticket/:id(\\d+)',
        component: () => import('@/views/system/ticket/TicketDetailView.vue'),
        props: (to) => ({ id: Number(to.params.id) })
      },
      { path: 'ticket/add', component: () => import('@/views/system/ticket/ChooseTicketSchemaView.vue') },
      {
        path: 'ticket/add/:id(\\d+)',
        component: () => import('@/views/system/ticket/AddTicketView.vue'),
        props: (to) => ({ id: Number(to.params.id) })
      },
      { path: 'manage-ticket', component: () => import('@/views/system/TicketManageView.vue') }
    ]
  }
]

export const router = createRouter({
  // 4. 内部提供了 history 模式的实现。为了简单起见，我们在这里使用 hash 模式。
  history: createWebHistory(),
  routes, // `routes: routes` 的缩写
})

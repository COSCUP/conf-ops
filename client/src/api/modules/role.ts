import { instance, makeRequest } from '@/api/base'
import { User } from '@/api/modules/project'

export interface PublicRole {
  name: string
  login_message?: string
}

export interface Role {
  id: string
  name: string
  project_id: string
  login_message?: string
  welcome_message?: string
}

export interface RoleUser {
  name: string
  locale: 'zh' | 'en'
  emails: string[]
}

export const createRole = () => ({
  get (id: string) {
    return makeRequest<PublicRole>(instance(`/project/role/${id}`))
  },
  getMyRoles () {
    return makeRequest<Role[]>(instance('/project/role/roles'))
  },
  getManageRoles () {
    return makeRequest<Role[]>(instance('/project/role/admin/roles'))
  },
  updateManageRole (id: string, role: { name?: string, login_message?: string, welcome_message?: string }) {
    return makeRequest<Role>(instance(`/project/role/admin/roles/${id}`, { method: 'PUT', body: role }))
  },
  getUsersInManageRole (id: string) {
    return makeRequest<User[]>(instance(`/project/role/admin/roles/${id}/users`))
  },
  addUsersToManageRole (id: string, users: RoleUser[]) {
    return makeRequest<void>(instance(`/project/role/admin/roles/${id}/users`, { method: 'POST', body: users }))
  },
  removeUserInManageRole (id: string, user_id: string) {
    return makeRequest<void>(instance(`/project/role/admin/roles/${id}/users/${user_id}`, { method: 'DELETE' }))
  }
})

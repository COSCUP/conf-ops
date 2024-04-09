import { instance, makeRequest } from '../base'

export interface Project {
  id: string
  name_zh: string
  description_zh: string
  name_en: string
  description_en: string
}

export interface User {
  id: string
  name: string
  locale: string
  project_id: string
  created_at: Date
  updated_at: Date
}

export interface Feature {
  type: 'RoleManage' | 'Ticket' | 'TicketManage',
  todo: [number, number]
}

export const createProject = () => ({
  getList () {
    return makeRequest<Project[]>(instance('/projects'))
  },
  get (id: string) {
    return makeRequest<Project>(instance(`/projects/${id}`))
  },
  login (project_id: string, email: string) {
    return makeRequest<void>(
      instance('/project/login', { method: 'POST', body: { project_id, email }, retry: false })
    )
  },
  verifyToken (token: string) {
    return makeRequest<void>(instance('/project/token', { method: 'POST', body: { token }, retry: false }))
  },
  logout () {
    return makeRequest<void>(instance('/project/logout', { method: 'POST' }))
  },
  getProjectInfo () {
    return makeRequest<Project>(instance('/project'))
  },
  getMeInfo () {
    return makeRequest<User>(instance('/project/me'))
  },
  getFeatures () {
    return makeRequest<Feature[]>(instance('/project/features'))
  }
})

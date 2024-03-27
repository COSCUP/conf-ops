import { createRole } from '@/api/modules/role'
import { createProject } from './modules/project'
import { createTicket } from '@/api/modules/ticket'

export const api = {
  project: createProject(),
  role: createRole(),
  ticket: createTicket()
}

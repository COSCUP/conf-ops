import { instance, makeRequest } from '../../base'
import { User } from '../project'
import { AddTicketReq, Ticket, TicketDetail, TicketProcessFlow, TicketSchema, TicketSchemaDetail } from './types'

export const createTicket = () => ({
  getMyTickets () {
    return makeRequest<Ticket[]>(instance('/project/ticket/tickets'))
  },
  getDetail (id: number) {
    return makeRequest<TicketDetail>(instance(`/project/ticket/tickets/${id}`))
  },
  process (id: number, flow: TicketProcessFlow) {
    return makeRequest<void, Record<string, string>>(instance(`/project/ticket/tickets/${id}/process`, { method: 'POST', body: { flow } }))
  },
  getProbablySchemas () {
    return makeRequest<TicketSchema[]>(instance('/project/ticket/schemas'))
  },
  getSchemaDetail (id: number) {
    return makeRequest<TicketSchemaDetail>(instance(`/project/ticket/schemas/${id}`))
  },
  getProbablyAssignUsers (schema_id: number, flow_id: number) {
    return makeRequest<User[]>(instance(`/project/ticket/schemas/${schema_id}/flows/${flow_id}/probably_assign_users`))
  },
  addTicketForSchema (id: number, data: AddTicketReq) {
    return makeRequest<void>(instance(`/project/ticket/schemas/${id}/tickets`, { method: 'POST', body: data }))
  }
})

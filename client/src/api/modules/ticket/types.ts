import { User } from '@/api/modules/project'
import { Role } from '@/api/modules/role'

export interface Ticket {
  id: number
  ticket_schema_id: number
  title: string
  status: 'InProgress' | 'Pending' | 'Finished'
  finished: boolean
  created_at: Date
  updated_at: Date
}

export interface TicketSchema {
  id: number
  title_zh: string
  description_zh: string
  title_en: string
  description_en: string
  created_at: Date
  updated_at: Date
}

export interface TicketSchemaFlow {
  id: number
  ticket_schema_id: number
  operator_id: string
  order: number
  name_zh: string
  name_en: string
  created_at: Date
  updated_at: Date
}

export type FormFieldDefault<C> = {
  type: 'Static',
  content: C
} | {
  type: 'Dynamic',
  content: {
    schema_form_id: number
    flow_id?: number
    field_id: number
    value?: C
  }
}

export type FormFieldDefine<V = number | string> = {
  type: 'SingleLineText',
  max_texts: number
  text_type?: 'email' | 'string' | 'url'
  default?: FormFieldDefault<string>
} | {
  type: 'MultiLineText',
  max_texts: number
  max_lines: number
  default?: FormFieldDefault<string>
} |{
  type: 'SingleChoice',
  options: Array<{
    text: string
    value: V
  }>
  default?: FormFieldDefault<V>
} | {
  type: 'MultipleChoice',
  options: Array<{
    text: string
    value: V
  }>
  max_options: number,
  is_checkbox: boolean,
  default?: FormFieldDefault<V[]>
} | {
  type: 'Bool',
  default?: FormFieldDefault<boolean>
} | {
  type: 'Image',
  min_width?: number,
  max_width?: number,
  min_height?: number,
  max_height?: number,
  mimes: string[],
  default?: FormFieldDefault<string>
} | {
  type: 'File',
  max_size: number,
  mimes: string[],
  default?: FormFieldDefault<string>
} | {
  type: 'IfEqual',
  key: string
  from: FormFieldDefault<V>
  value: V[]
} | {
  type: 'IfEnd',
  key: string
}

export interface TicketFormSchema {
  type: 'Form'
  form: {
    id: number
    ticket_schema_flow_id: number
    expired_at?: Date
    created_at: Date
    updated_at: Date
  }
  fields: Array<{
    id: number
    ticket_schema_form_id: number
    order: number
    key: string
    name_zh: string
    description_zh: string
    name_en: string
    description_en: string
    define: FormFieldDefine
    required: boolean
    editable: boolean
    created_at: Date
    updated_at: Date
  }>
}

export interface TicketReviewSchema {
  type: 'Review'
  id: number
  ticket_schema_flow_id: number
  restarted: boolean
  created_at: Date
  updated_at: Date
}

export type TicketSchemaFlowValue = TicketFormSchema | TicketReviewSchema

export interface TicketSchemaFlowItem extends TicketSchemaFlow {
  module: TicketSchemaFlowValue
}

export interface TicketFormValue {
  type: 'Form'
  id: number
  ticket_flow_id: number
  ticket_schema_flow_id: number
  value: Record<string, string | number | boolean | string[] | number[]>
  created_at: Date
  updated_at: Date
}

export interface TicketReviewValue {
  type: 'Review'
  id: number
  ticket_flow_id: number
  ticket_schema_review_id: number
  approved: boolean
  comment?: string
  created_at: Date
  updated_at: Date
}

export type TicketFlowValue = {
  type: 'None'
} | TicketFormValue | TicketReviewValue

export type TicketFlowOperator = ({
  type: 'User'
} & User) | {
  type: 'Role'
} & Role | {
  type: 'None'
}

export interface TicketFlowItem {
  id: number
  ticket_id: number
  user_id?: string
  ticket_schema_flow_id: number
  finished: boolean
  created_at: Date
  updated_at: Date
  module: TicketFlowValue
  operator: TicketFlowOperator
}

export interface TicketFlowStatus {
  schema: TicketSchemaFlowItem,
  flow?: TicketFlowItem
}


export interface TicketDetail extends Ticket {
  schema: TicketSchema
  flows: TicketFlowStatus[]
}

export interface TicketFormProcessFlow {
  type: 'Form'
  [key: string]: any
}

export interface TicketReviewProcessFlow {
  type: 'Review'
  approved: boolean
  comment?: string
}

export type TicketProcessFlow = TicketFormProcessFlow | TicketReviewProcessFlow

export interface TicketSchemaDetail extends TicketSchema {
  flows: TicketSchemaFlowItem[]
}

export interface AddTicketReq {
  title: string
  assign_flow_users: Record<number, string>
}

import { Feature } from '@/api/modules/project'
import { Component } from 'vue'
import { UserRole, Ticket, GuiManagement } from '@vicons/carbon'

export interface FeatureDetail {
  key: string
  icon: Component
  url: string
}

const featureTable: {[key in Feature['type']]: FeatureDetail } = {
  RoleManage: {
    key: 'role-manage',
    icon: UserRole,
    url: '/system/manage-role'
  },
  Ticket: {
    key: 'ticket',
    icon: Ticket,
    url: '/system/ticket'
  },
  TicketManage: {
    key: 'ticket-manage',
    icon: GuiManagement,
    url: '/system/manage-ticket'
  }
}

export function getFeatureDetail (type: Feature['type']): FeatureDetail {
  return featureTable[type]
}

import { Component, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { Home, AddAlt, Ticket, PendingFilled, InProgress, CheckmarkFilled } from '@vicons/carbon'
import { usePageTitle } from '@/functions/usePage'

export enum TicketBreadcrumbType {
  HOME,
  DETAIL,
  ADD1,
  ADD2
}

export interface TicketBreadcrumbValue {
  key: string,
  icon: Component,
  url: string | null
}

const table: { [key in TicketBreadcrumbType]: TicketBreadcrumbValue } = {
  [TicketBreadcrumbType.HOME]: {
    key: 'ticket.page.home',
    icon: Home,
    url: '/system/ticket'
  },
  [TicketBreadcrumbType.DETAIL]: {
    key: 'ticket.page.detail',
    icon: Ticket,
    url: ''
  },
  [TicketBreadcrumbType.ADD1]: {
    key: 'ticket.page.add-1',
    icon: AddAlt,
    url: '/system/ticket/add'
  },
  [TicketBreadcrumbType.ADD2]: {
    key: 'ticket.page.add-2',
    icon: AddAlt,
    url: ''
  }
}

export function useTicketBreadcrumb (types: TicketBreadcrumbType[]) {
  const { t } = useI18n()

  usePageTitle(() => t(table[types[types.length - 1]].key))

  return computed(() => types.map(type => ({
    key: t(table[type].key),
    icon: table[type].icon,
    url: table[type].url
  })))
}

export const getStatusIcon = (status: 'Pending' | 'InProgress' | 'Finished') => {
  switch (status) {
    case 'Pending':
      return PendingFilled
    case 'InProgress':
      return InProgress
    case 'Finished':
      return CheckmarkFilled
  }
}

export const getStatusType = (status: 'Pending' | 'InProgress' | 'Finished') => {
  switch (status) {
    case 'Pending':
      return 'error'
    case 'InProgress':
      return 'warning'
    case 'Finished':
      return 'success'
  }
}

<template>
  <TicketBreadcrumb :value="breadcrumbs">
    <RouterLink
      class="no-underline"
      to="/system/ticket/add"
    >
      <NButton
        type="primary"
      >
        {{ t('add') }}
        <template #icon>
          <NIcon>
            <AddAlt />
          </NIcon>
        </template>
      </NButton>
    </RouterLink>
  </TicketBreadcrumb>
  <NList
    hoverable
    clickable
  >
    <NListItem
      v-for="ticket in tickets"
      :key="ticket.id"
    >
      <RouterLink
        :to="`/system/ticket/${ticket.id}`"
        class="no-underline"
      >
        <NThing>
          <template #header>
            {{ ticket.title }}
          </template>
          <template #header-extra>
            <NText
              :type="getStatusType(ticket.status)"
              class="flex flex-items-center break-keep"
            >
              <NIcon
                class="ma-1"
                :size="24"
                :component="getStatusIcon(ticket.status)"
              />
              {{ t(`ticket.status.${ticket.status}`) }}
            </NText>
          </template>
          <template #description>
            <NP class="my-0">
              {{ t('form.created_at') }}: <NTime :value="ticket.created_at" />
            </NP>
            <NP class="my-0">
              {{ t('form.updated_at') }}: <NTime :value="ticket.updated_at" />
            </NP>
          </template>
        </NThing>
      </RouterLink>
    </NListItem>
  </NList>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { AddAlt } from '@vicons/carbon'
import TicketBreadcrumb from '@/components/system/ticket/TicketBreadcrumb.vue'
import { RouterLink } from 'vue-router'
import { TicketBreadcrumbType, useTicketBreadcrumb, getStatusIcon, getStatusType } from '@/functions/useTicket'
import { useAPI } from '@/functions/useAPI'
import { api } from '@/api'
import { usePageLoading } from '@/functions/usePage'

const { t } = useI18n()

const breadcrumbs = useTicketBreadcrumb([TicketBreadcrumbType.HOME])

const { data: tickets, loading } = useAPI(api.ticket.getMyTickets, { default: [] })

usePageLoading(() => loading.value)
</script>

<i18n lang="json">
{
  "en": {
    "add": "Add Ticket",
    "form": {
      "status": "Status",
      "id": "ID",
      "title": "Title",
      "created_at": "Created At",
      "updated_at": "Updated At"
    },
    "actions": "Actions",
    "enter": "Enter"
  },
  "zh": {
    "add": "新增工單",
    "form": {
      "status": "狀態",
      "id": "ID",
      "title": "標題",
      "created_at": "創建時間",
      "updated_at": "更新時間"
    },
    "actions": "操作",
    "enter": "進入"
  }
}
</i18n>

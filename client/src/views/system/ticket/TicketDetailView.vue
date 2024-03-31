<template>
  <TicketBreadcrumb :value="breadcrumbs" />
  <NCard v-if="ticket">
    <TicketHeader
      :ticket="ticket"
      :is-pending="isPending"
      :current="processFlow"
    />
    <TicketFlowModule
      v-if="isPending && processFlow"
      :id="id"
      :current="processFlow"
      :flows="ticket?.flows ?? []"
      @refresh="refresh"
    />
    <NResult
      v-else-if="processFlow"
      status="info"
      class="my-8"
      :title="t(`ticket.status.${ticket?.status}`)"
      :description="t(`message.${ticket?.status}`)"
    >
      <template #icon>
        <NText
          :type="getStatusType(ticket.status)"
        >
          <NIcon
            :size="100"
            :component="getStatusIcon(ticket.status)"
          />
        </NText>
      </template>
      <template #footer>
        <NButton
          v-if="ticket.status !== 'Finished'"
          class="mr-2"
          type="info"
          @click="refresh"
        >
          {{ t('refresh') }}
        </NButton>
        <NButton
          class="ml-2"
          type="primary"
          @click="router.back"
        >
          {{ t('back') }}
        </NButton>
      </template>
    </NResult>
  </NCard>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import TicketBreadcrumb from '@/components/system/ticket/TicketBreadcrumb.vue'
import { TicketBreadcrumbType, useTicketBreadcrumb, getStatusIcon, getStatusType } from '@/functions/useTicket'
import { useAPI } from '@/functions/useAPI'
import { api } from '@/api'
import { usePageLoading } from '@/functions/usePage'
import { computed } from 'vue'
import TicketFlowModule from '@/components/system/ticket/TicketFlowModule.vue'
import { useRouter } from 'vue-router'
import { useProject } from '@/functions/useProject'
import TicketHeader from '@/components/system/ticket/TicketHeader.vue'

const props = defineProps<{
  id: number
}>()

const router = useRouter()
const { t } = useI18n()

const { reloadFeatures } = useProject()
const breadcrumbs = useTicketBreadcrumb([TicketBreadcrumbType.HOME, TicketBreadcrumbType.DETAIL])

const { data: ticket, loading, execute: reloadTicket } = useAPI(() => api.ticket.getDetail(props.id))

const refresh = () => {
  reloadTicket()
  reloadFeatures()
}

usePageLoading(() => loading.value)

const isPending = computed(() => ticket.value?.status === 'Pending')

const processFlow = computed(() =>
  ticket.value?.flows.find(flow => flow.flow?.finished === false) ?? ticket.value?.flows[ticket.value?.flows.length - 1]
)
</script>

<i18n lang="json">
{
  "en": {
    "message": {
      "InProgress": "This ticket is in progress, please wait patiently",
      "Finished": "This ticket has been finished"
    },
    "refresh": "Refresh",
    "back": "Back"
  },
  "zh": {
    "message": {
      "InProgress": "此工單正在進行中，請耐心等待",
      "Finished": "此工單已完成"
    },
    "refresh": "重新整理",
    "back": "返回"
  }
}
</i18n>

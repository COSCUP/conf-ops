<template>
  <TicketBreadcrumb :value="breadcrumbs" />
  <NCard v-if="ticket">
    <NTimeline
      class="mb-4 mt-4 overflow-x-auto overflow-y-hidden"
      :icon-size="36"
      horizontal
      size="large"
    >
      <NTimelineItem
        v-for="flow in ticket?.flows || []"
        :key="flow.flow?.id"
        class="md:flex-1 !last:pr-0 !<md:pr-4 mt-2"
        :type="getFlowType(flow)"
        :line-type="flow.flow?.finished ? 'default' : 'dashed'"
        :time="getFlowTime(flow)"
      >
        <template #header>
          <NText
            class="flex flex-content-center line-height-[1]"
            :type="getFlowType(flow)"
          >
            <NIcon
              v-if="isDisplayFlowStatusIcon(flow)"
              class="mr-1"
              :component="getFlowStatusIcon(flow)"
            />
            {{ t(`ticket.module.${flow.schema.module.type}`) }}
          </NText>
        </template>
        <NText
          class="block"
          :type="getFlowType(flow)"
        >
          {{ flow.schema.name }}
        </NText>
        <component
          class="mt-1"
          :is="getOperatorText(flow)"
        />
        <template #icon>
          <NIcon :component="getFlowIcon(flow)" />
        </template>
      </NTimelineItem>
    </NTimeline>
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
import { TicketFlowStatus } from '@/api/modules/ticket/types'
import { DataFormat, Review, Unknown, CloseFilled } from '@vicons/carbon'
import { computed } from 'vue'
import { h } from 'vue'
import { NTag } from 'naive-ui'
import TicketFlowModule from '@/components/system/ticket/TicketFlowModule.vue'
import { useRouter } from 'vue-router'
import { useProject } from '@/functions/useProject'

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

const getFlowType = (flow: TicketFlowStatus) => {
  if (flow.flow?.module.type === 'Review' && !flow.flow.module.approved && flow.flow.id !== processFlow.value?.flow?.id) {
    return 'error'
  }
  if (flow.flow?.finished || flow.flow?.id === processFlow.value?.flow?.id) {
    return getStatusType(getFlowStatus(flow))
  }
  return 'default'
}

const getFlowTime = (flow: TicketFlowStatus) => {
  if (
    flow.flow?.finished ||
    (flow.flow?.module.type === 'Review' && !flow.flow.module.approved && flow.flow.id !== processFlow.value?.flow?.id)
  ) {
    return flow.flow?.updated_at.toLocaleString()
  }
  return ''
}

const getFlowIcon = (flow: TicketFlowStatus) => {
  switch (flow.schema.module.type) {
    case 'Form':
      return DataFormat
    case 'Review':
      return Review
    default:
      return Unknown
  }
}

const getFlowStatus = (flow: TicketFlowStatus) => {
  if (flow.flow?.finished) {
    return 'Finished'
  }
  if (flow.flow?.id === processFlow.value?.flow?.id && isPending.value) {
    return 'Pending'
  }
  return 'InProgress'
}

const isDisplayFlowStatusIcon = (flow: TicketFlowStatus) => {
  return flow.flow?.finished ||
    flow.flow?.id === processFlow.value?.flow?.id ||
    (flow.flow?.module.type === 'Review' && !flow.flow.module.approved && flow.flow.id !== processFlow.value?.flow?.id)
}

const getFlowStatusIcon = (flow: TicketFlowStatus) => {
  if (flow.flow?.module.type === 'Review' && !flow.flow.module.approved && flow.flow.id !== processFlow.value?.flow?.id) {
    return CloseFilled
  }
  return getStatusIcon(getFlowStatus(flow))
}

const getOperatorText = (flow: TicketFlowStatus) => {
  const operator = flow.flow?.operator
  const getTag = (children: () => any) => h(
      NTag,
      {
        class: 'break-all',
        round: true,
        bordered: false,
        size: 'small'
      },
      children
    )

  if (operator?.type === 'User') {
    return getTag(() => [
        h(
          'span',
          { class: 'color-gray' },
          t('operator.user')
        ),
        ' ',
        operator.name
      ]
    )
  }
  if (operator?.type === 'Role') {
    return getTag(() => [
        h(
          'span',
          { class: 'color-gray' },
          t('operator.role')
        ),
        ' ',
        operator.name
      ]
    )
  }
  return getTag(() => h(
      'span',
      {},
      t('operator.none')
    )
  )
}
</script>

<i18n lang="json">
{
  "en": {
    "message": {
      "InProgress": "This ticket is in progress, please wait patiently",
      "Finished": "This ticket has been finished"
    },
    "operator": {
      "user": "user",
      "role": "role",
      "none": "None"
    },
    "refresh": "Refresh",
    "back": "Back"
  },
  "zh": {
    "message": {
      "InProgress": "此工單正在進行中，請耐心等待",
      "Finished": "此工單已完成"
    },
    "operator": {
      "user": "使用者",
      "role": "角色",
      "none": "無"
    },
    "refresh": "重新整理",
    "back": "返回"
  }
}
</i18n>@/functions/useTicket

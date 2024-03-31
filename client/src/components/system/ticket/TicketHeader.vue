<template>
  <NSteps
    class="pt-4 pb-8"
    :vertical="isMobile"
  >
    <NStep
      v-for="flow in ticket?.flows || []"
      :key="flow.flow?.id"
      :status="getFlowStepStatus(flow)"
    >
      <template #title>
        <NText
          class="flex flex-items-center h-[23px]"
          :type="getFlowType(flow)"
        >
          <NIcon
            class="mr-1"
            :component="getFlowIcon(flow)"
          />
          {{ t(`ticket.module.${flow.schema.module.type}`) }}
        </NText>
      </template>
      <NText
        class="block"
        :type="getFlowType(flow)"
      >
        {{ flow.schema[`name_${locale}`] }}
      </NText>
      <component
        class="my-1"
        :is="getOperatorText(flow)"
      />
      <NTime
        v-if="getFlowTime(flow)"
        class="flex text-sm"
        :time="getFlowTime(flow)"
      />
    </NStep>
  </NSteps>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { TicketDetail, TicketFlowStatus } from '/Users/yoyo930021/git/conf-ops/client/src/api/modules/ticket/types'
import { DataFormat, Review, Unknown } from '@vicons/carbon'
import { getStatusType } from '@/functions/useTicket'
import { NTag } from 'naive-ui'
import { h } from 'vue'
import { useLocale } from '@/i18n'
import { useBreakpoint } from '@/functions/useBreakpoint'

const props = defineProps<{
  ticket: TicketDetail,
  isPending: boolean,
  current: TicketFlowStatus | undefined
}>()

const { t } = useI18n()
const { locale } = useLocale()
const { isMobile } = useBreakpoint()

const getFlowType = (flow: TicketFlowStatus) => {
  if (flow.flow?.module.type === 'Review' && !flow.flow.module.approved && flow.flow.id !== props.current?.flow?.id) {
    return 'error'
  }
  if (flow.flow?.finished || flow.flow?.id === props.current?.flow?.id) {
    return getStatusType(getFlowStatus(flow))
  }
  return 'default'
}

const getFlowTime = (flow: TicketFlowStatus) => {
  if (
    flow.flow?.finished ||
    (flow.flow?.module.type === 'Review' && !flow.flow.module.approved && flow.flow.id !== props.current?.flow?.id)
  ) {
    return flow.flow?.updated_at
  }
  return undefined
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
  if (flow.flow?.id === props.current?.flow?.id && props.isPending) {
    return 'Pending'
  }
  return 'InProgress'
}

const getFlowStepStatus = (flow: TicketFlowStatus) => {
  if (flow.flow?.finished) {
    return 'finish'
  }
  if (flow.flow?.id === props.current?.flow?.id && props.isPending) {
    return 'process'
  }
  return 'wait'
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
        operator[`name_${locale.value}`]
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
    "operator": {
      "user": "user",
      "role": "role",
      "none": "None"
    }
  },
  "zh": {
    "operator": {
      "user": "使用者",
      "role": "角色",
      "none": "無"
    }
  }
}
</i18n>

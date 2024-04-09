<template>
  <template v-if="notApprovedTicketReviewValue">
    <NAlert
      type="error"
      show-icon
      :title="t('not_approved')"
      class="mb-4"
    >
      {{ notApprovedTicketReviewValue.comment }}
    </NAlert>
  </template>
  <template v-if="current.schema.module.type === 'Form' && current.flow?.module.type === 'Form'">
    <TicketFormFlow
      :id="id"
      :ticket-schema-id="current.schema.ticket_schema_id"
      :schema="current.schema.module"
      :form-value="current.flow?.module"
      :isReview="isReview"
      @refresh="emit('refresh')"
    />
  </template>
  <template v-else-if="current.schema.module.type === 'Form'">
    <TicketFormFlow
      :id="id"
      :ticket-schema-id="current.schema.ticket_schema_id"
      :schema="current.schema.module"
      :form-value="null"
      :isReview="isReview"
      @refresh="emit('refresh')"
    />
  </template>
  <template v-else-if="current.schema.module.type === 'Review' && current.flow?.module.type === 'Review'">
    <TicketReviewFlow
      :id="id"
      :current-id="current.flow.id"
      :schema="current.schema.module"
      :flows="flows"
      :review-value="current.flow?.module"
      :isReview="isReview"
      @refresh="emit('refresh')"
    />
  </template>
  <template v-else-if="current.schema.module.type === 'Review'">
    <TicketReviewFlow
      :id="id"
      :current-id="current.flow?.id"
      :schema="current.schema.module"
      :flows="flows"
      :review-value="null"
      :isReview="isReview"
      @refresh="emit('refresh')"
    />
  </template>
</template>

<script setup lang="ts">
import { TicketFlowStatus } from '@/api/modules/ticket/types'
import TicketFormFlow from './module/TicketFormFlow.vue'
import TicketReviewFlow from './module/TicketReviewFlow.vue'
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'

const props = defineProps<{
  id: number,
  current: TicketFlowStatus
  flows: TicketFlowStatus[]
  isReview?: boolean
}>()

const emit = defineEmits<{
  refresh: []
}>()

const { t } = useI18n()

const notApprovedTicketReviewValue = computed(() => {
  const flow = props.flows.find(flow => flow.flow?.module.type === 'Review' && flow.flow?.module.approved === false)
  if (props.current.flow?.id === flow?.flow?.id) {
    return null
  }
  if (flow?.flow?.module.type === 'Review') {
    return flow.flow.module
  }
  return null
})
</script>

<i18n lang="json">
{
  "en": {
    "not_approved": "Not approved"
  },
  "zh": {
    "not_approved": "未通過"
  }
}
</i18n>

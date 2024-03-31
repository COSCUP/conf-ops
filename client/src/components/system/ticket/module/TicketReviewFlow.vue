<template>
  <template v-if="!isReview">
    <template
      v-for="flow in previousFlows"
      :key="flow.flow?.id"
    >
      <NDivider>{{ flow.schema[`name_${locale}`] }}</NDivider>
      <TicketFlowModule
        :id="id"
        :current="flow"
        :flows="previousFlows"
        isReview
      />
    </template>
    <NDivider>{{ t('action') }}</NDivider>
  </template>
  <NForm
    ref="formRef"
    :model="formData"
    :rules="rules"
  >
    <NFormItem
      path="comment"
      :show-label="false"
    >
      <NInput
        type="textarea"
        v-model:value="formData.comment"
        show-count
        :maxlength="255"
        :placeholder="t('comment')"
        :disabled="isReview"
      />
    </NFormItem>
    <NFlex
      v-if="!isReview"
      justify="space-between"
    >
      <NButton
        class="w-[100px]"
        type="primary"
        :loading="loading"
        @click="execute(true)"
      >
        {{ t('approved') }}
      </NButton>
      <NButton
        class="w-[100px]"
        type="error"
        :loading="loading"
        @click="execute(false)"
      >
        {{ t('failed') }}
      </NButton>
    </NFlex>
  </NForm>
</template>

<script setup lang="ts">
import { api } from '@/api'
import { TicketFlowStatus, TicketReviewSchema, TicketReviewValue } from '@/api/modules/ticket/types'
import { useFormAPI } from '@/functions/useAPI'
import { FormInst, FormRules, useDialog } from 'naive-ui'
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import TicketFlowModule from '../TicketFlowModule.vue'
import { useLocale } from '@/i18n'

const props = defineProps<{
  id: number
  schema: TicketReviewSchema
  flows: TicketFlowStatus[]
  currentId?: number
  reviewValue: TicketReviewValue | null
  isReview?: boolean
}>()

const emit = defineEmits<{
  refresh: []
}>()

const dialog = useDialog()
const { t } = useI18n()
const { locale } = useLocale()

const formRef = ref<FormInst | null>(null)
const formData = ref({
  comment: ''
})

const previousFlows = computed(() => {
  const index = props.flows.findIndex(flow => flow.flow?.id === props.currentId)
  return props.flows.slice(0, index)
})

const rules = computed(() => ({
  comment: { required: true, message: t('required'), trigger: 'blur' }
} as FormRules))

const { execute, loading } = useFormAPI(
  (approved: boolean) => api.ticket.process(props.id, {
    type: 'Review',
    approved,
    comment: formData.value.comment
  }),
  () => formRef.value,
  () => {
    dialog.success({ content: t('save.success') })
    emit('refresh')
  }
)
</script>

<i18n lang="json">
{
  "en": {
    "action": "Action",
    "approved": "Approved",
    "failed": "Failed",
    "comment": "Comment",
    "required": "此欄位為必填"
  },
  "zh": {
    "action": "操作",
    "approved": "通過",
    "failed": "未通過",
    "comment": "說明",
    "required": "此欄位為必填"
  }
}
</i18n>

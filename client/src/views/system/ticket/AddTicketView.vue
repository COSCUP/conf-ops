<template>
  <TicketBreadcrumb :value="breadcrumbs" />
  <NCard>
    <NForm
      v-if="schema"
      ref="formRef"
      :label-placement="isMobile ? 'top' : 'left'"
      :model="addTicketValue"
      @submit.prevent="execute"
    >
      <NFormItem
        :label="t('form.title')"
        path="title"
      >
        <NInput
          v-model:value="addTicketValue.title"
          :default-value="schema.title"
          show-count
          :maxlength="150"
        />
      </NFormItem>
      <NCollapse
        default-expanded-names="advanced"
      >
        <NCollapseItem
          :title="t('form.advanced')"
          name="advanced"
        >
          <NCard
            :title="t('form.assign_flow_users')"
          >
            <NFormItem
              v-for="flow in schema.flows"
              :key="flow.id"
              :label="`${flow.order}. ${flow.name}`"
              :path="`assign_flow_users.[${flow.id}]`"
            >
              <RoleUserSelect
                v-model="addTicketValue.assign_flow_users[flow.id]"
                :user-api="() => api.ticket.getProbablyAssignUsers(props.id, flow.id)"
              />
            </NFormItem>
          </NCard>
        </NCollapseItem>
      </NCollapse>
      <NFormItem
        class="mt-4"
      >
        <NButton
          type="primary"
          block
          :loading="submitLoading"
          attr-type="submit"
        >
          {{ t('add') }}
        </NButton>
      </NFormItem>
    </NForm>
  </NCard>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import TicketBreadcrumb from '@/components/system/ticket/TicketBreadcrumb.vue'
import { TicketBreadcrumbType, useTicketBreadcrumb } from '@/functions/useTicket'
import { useAPI, useFormAPI } from '@/functions/useAPI'
import { api } from '@/api'
import { usePageLoading } from '@/functions/usePage'
import { ref, watch } from 'vue'
import { AddTicketReq } from '@/api/modules/ticket/types'
import { FormInst, useDialog } from 'naive-ui'
import RoleUserSelect from '@/components/system/common/RoleUserSelect.vue'
import { useRouter } from 'vue-router'
import { useBreakpoint } from '@/functions/useBreakpoint'

const props = defineProps<{
  id: number
}>()

const dialog = useDialog()
const router = useRouter()
const { t } = useI18n()
const { isMobile } = useBreakpoint()

const breadcrumbs = useTicketBreadcrumb([TicketBreadcrumbType.HOME, TicketBreadcrumbType.ADD1, TicketBreadcrumbType.ADD2])
const { data: schema, loading } = useAPI(() => api.ticket.getSchemaDetail(props.id))

usePageLoading(() => loading.value)

const formRef = ref<FormInst | null>(null)
const addTicketValue = ref<AddTicketReq>({
  title: '',
  assign_flow_users: {}
})

watch(schema, () => {
  if (schema.value) {
    addTicketValue.value.title = schema.value.title
  }
})

const { execute, loading: submitLoading } = useFormAPI(
  () => api.ticket.addTicketForSchema(props.id, addTicketValue.value),
  () => formRef.value,
  () => {
    dialog.success({ content: t('save.success') })
    router.push('/system/ticket')
  }
)
</script>

<i18n lang="json">
{
  "en": {
    "form": {
      "title": "Ticket Title",
      "advanced": "Advanced",
      "assign_flow_users": "Assign Flow Users"
    },
    "add": "Add"
  },
  "zh": {
    "form": {
      "title": "工單標題",
      "advanced": "進階",
      "assign_flow_users": "指派流程使用者"
    },
    "add": "新增"
  }
}
</i18n>
@/functions/useTicket

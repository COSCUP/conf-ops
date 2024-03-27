<template>
  <NCard>
    <NForm
      :model="formValue"
      @submit.prevent="save"
    >
      <NFormItem :label="t('id')">
        <NText>{{ role.id }}</NText>
      </NFormItem>
      <NFormItem :label="t('name')">
        <NInput
          v-model:value="formValue.name"
          show-count
          :maxlength="50"
        />
      </NFormItem>
      <NFormItem :label="t('login_message')">
        <NInput
          v-model:value="formValue.login_message"
          type="textarea"
        />
      </NFormItem>
      <NFormItem :label="t('welcome_message')">
        <NInput
          v-model:value="formValue.welcome_message"
          type="textarea"
        />
      </NFormItem>
      <NButton
        type="primary"
        size="large"
        block
        :loading="loading"
        attr-type="submit"
      >
        {{ t('save.title') }}
      </NButton>
    </NForm>
  </NCard>
</template>

<script setup lang="ts">
import { api } from '@/api'
import { Role } from '@/api/modules/role'
import { useFormAPI } from '@/functions/useAPI'
import { FormInst, useDialog } from 'naive-ui'
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'

const props = defineProps<{
  role: Role
}>()

const emit = defineEmits<{ saved: [] }>()

const { t } = useI18n()
const dialog = useDialog()

const formRef = ref<FormInst | null>(null)
const formValue = ref({
  name: props.role.name,
  login_message: props.role.login_message,
  welcome_message: props.role.welcome_message
})


const { execute: save, loading } = useFormAPI(
  () => api.role.updateManageRole(props.role.id, formValue.value),
  () => formRef.value,
  () => {
    dialog.success({ content: t('save.success') })
    emit('saved')
  }
)
</script>

<i18n lang="json">
{
  "en": {
    "id": "Role ID",
    "name": "Role Name",
    "login_message": "Login Message",
    "welcome_message": "Welcome Message"
  },
  "zh": {
    "id": "角色 ID",
    "name": "角色名",
    "login_message": "登入訊息",
    "welcome_message": "歡迎訊息"
  }
}
</i18n>

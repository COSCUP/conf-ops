<template>
  <NCard>
    <NForm
      ref="formRef"
      :model="formValue"
      @submit.prevent="save"
    >
      <NFormItem :label="t('id')">
        <NText>{{ role.id }}</NText>
      </NFormItem>
      <NFlex>
        <NFormItem
          class="flex-[1_1_200px]"
          :label="t('name', {}, { locale: 'zh' })"
        >
          <NInput
            v-model:value="formValue.name_zh"
            show-count
            :maxlength="50"
          />
        </NFormItem>
        <NFormItem
          class="flex-[1_1_200px]"
          :label="t('name', {}, { locale: 'en' })"
        >
          <NInput
            v-model:value="formValue.name_en"
            show-count
            :maxlength="50"
          />
        </NFormItem>
      </NFlex>
      <NFlex>
        <NFormItem
          class="flex-[1_1_200px]"
          :label="t('login_message', {}, { locale: 'zh' })"
        >
          <NInput
            v-model:value="formValue.login_message_zh"
            type="textarea"
          />
        </NFormItem>
        <NFormItem
          class="flex-[1_1_200px]"
          :label="t('login_message', {}, { locale: 'en' })"
        >
          <NInput
            v-model:value="formValue.login_message_en"
            type="textarea"
          />
        </NFormItem>
      </NFlex>
      <NFlex>
        <NFormItem
          class="flex-[1_1_200px]"
          :label="t('welcome_message', {}, { locale: 'zh' })"
        >
          <NInput
            v-model:value="formValue.welcome_message_zh"
            type="textarea"
          />
        </NFormItem>
        <NFormItem
          class="flex-[1_1_200px]"
          :label="t('welcome_message', {}, { locale: 'en' })"
        >
          <NInput
            v-model:value="formValue.welcome_message_en"
            type="textarea"
          />
        </NFormItem>
      </NFlex>
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
  name_zh: props.role.name_zh,
  name_en: props.role.name_en,
  login_message_zh: props.role.login_message_zh,
  login_message_en: props.role.login_message_en,
  welcome_message_zh: props.role.welcome_message_zh,
  welcome_message_en: props.role.welcome_message_en
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

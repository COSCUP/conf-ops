<template>
  <HomeLayout>
    <NFlex
      vertical
      align="center"
    >
      <img class="mt-4 max-w-full w-xs" src="@/assets/logo.svg">
      <NH1 class="mb-1">
        {{ project?.name }}
        <template v-if="role">
          <span> - {{ role.name }}</span>
        </template>
      </NH1>
      <NP class="mt-1">
        {{ project?.description }}
      </NP>
      <NAlert
        v-if="role"
        type="info"
        class="w-xs"
      >
        {{ role.login_message }}
      </NAlert>
      <NCard
        :title="t('title')"
        class="w-auto"
      >
        <NForm
          ref="formRef"
          :model="formData"
          :rules="rules"
          @submit.prevent="login"
        >
          <NFormItem
            :show-label="false"
            path="email"
          >
            <NInput
              v-model:value="formData.email"
              size="large"
              :placeholder="t('email.label')"
            />
          </NFormItem>
          <NP class="whitespace-pre-line my-2">
            {{ t('login.note') }}
          </NP>
          <NButton
            type="primary"
            size="large"
            block
            :loading="loading"
            attr-type="submit"
          >
            {{ t('login.label') }}
          </NButton>
        </NForm>
      </NCard>
    </NFlex>
  </HomeLayout>
</template>

<script setup lang="ts">
import { api } from '@/api'
import HomeLayout from '@/components/HomeLayout.vue'
import { makeEmptyRequest } from '@/api/base'
import { useAPI, useFormAPI } from '@/functions/useAPI'
import { usePageNotFound, usePageLoading, usePageTitle } from '@/functions/usePage'
import { FormInst, FormRules, useDialog } from 'naive-ui'
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'

const props = defineProps<{
  projectId: string,
  roleId?: string
}>()

const { t } = useI18n()
const dialog = useDialog()
const router = useRouter()

const { data: project, loading: projectLoading } = useAPI(() => api.project.get(props.projectId))

const { data: role, loading: roleLoading } = useAPI(() => props.roleId ? api.role.get(props.roleId) : makeEmptyRequest())

usePageTitle(() => `${t('title', [project.value?.name ?? ''])}`)
usePageLoading(() => projectLoading.value || roleLoading.value)
usePageNotFound(() => !projectLoading.value && !project.value)

const formRef = ref<FormInst | null>(null)
const formData = ref({ email: '' })

const rules = computed(() => {
  return {
    email: [
      { required: true, message: t('email.required'), trigger: 'blur' },
      { type: 'email', message: t('email.invalid'), trigger: ['input', 'blur'] }
    ]
  } as FormRules
})

const { execute: login, loading } = useFormAPI(
  () => api.project.login(props.projectId, formData.value.email),
  () => formRef.value,
  () => {
    dialog.success({
      title: t('login.success.title'),
      content: t('login.success.message'),
      positiveText: t('ok'),
      onPositiveClick: () => {
        router.push(`/`)
      }
    })
  }
)

</script>

<i18n lang="json">
{
  "en": {
    "title": "Login {0}",
    "email": {
      "label": "Enter Email",
      "required": "Email is required",
      "invalid": "Invalid email"
    },
    "login": {
      "label": "Login",
      "success": {
        "title": "Login successful",
        "message": "Please check your email to confirm login, check spam if not found."
      },
      "note": "Please note that you can only send once every 5 minutes."
    },
    "welcome": "Welcome to {0}!"
  },
  "zh": {
    "title": "登入 {0}",
    "email": {
      "label": "請輸入 Email",
      "required": "Email 是必需的",
      "invalid": "無效的 Email"
    },
    "login": {
      "label": "登入",
      "success": {
        "title": "登入成功",
        "message": "請前往你的 Email 信箱確認登入，找不到時請檢查垃圾信件。"
      },
      "note": "請注意，每 5 分鐘只能寄一次登入信。"
    },
    "welcome": "歡迎來到 {0}!"
  }
}
</i18n>

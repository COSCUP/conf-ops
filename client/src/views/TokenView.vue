<template>
  <HomeLayout>
    <NFlex
      vertical
      align="center"
      class="pt-4 overflow-hidden"
    >
      <NH1 class="mb-0">
        {{ t('title') }}
      </NH1>
      <NP>
        {{ message }}
      </NP>
      <NSpin :size="80" />
    </NFlex>
  </HomeLayout>
</template>

<script setup lang="ts">
import { api } from '@/api';
import HomeLayout from '@/components/HomeLayout.vue'
import { useActionAPI } from '@/functions/useAPI'
import { usePageTitle } from '@/functions/usePage'
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'

const props = defineProps<{
  token: string
}>()


const { t } = useI18n()
const router = useRouter()

usePageTitle(() => t('title'))

const { execute } = useActionAPI(
  () => api.project.verifyToken(props.token),
  () => {
    message.value = t('success')
    setTimeout(() => {
      router.push(`/system`)
    }, 4000)
  },
  {
    failure: () => {
      message.value = t('failure')
      setTimeout(() => {
        router.push(`/`)
      }, 4000)
    }
  }
)
execute(null)

const message = ref(t('waiting'))
</script>

<i18n lang="json">
{
  "en": {
    "title": "Logging in",
    "waiting": "Please wait",
    "success": "Logged in successfully, redirecting to project page",
    "failure": "Login failed, please resend the email, redirecting to home."
  },
  "zh": {
    "title": "登入中",
    "waiting": "請稍候",
    "success": "登入成功，即將跳轉至專案頁面",
    "failure": "登入失敗，請重新寄信，即將跳轉至首頁。"
  }
}
</i18n>

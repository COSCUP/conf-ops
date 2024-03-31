<template>
  <HomeLayout>
    <NResult
      class="mt-4"
      status="404"
      :title="t('title')"
      :description="t('description')"
    >
      <template #footer>
        <NButton
          @click="router.push('/')"
        >
          {{ t('back') }}
        </NButton>
      </template>
    </NResult>
  </HomeLayout>
</template>

<script setup lang="ts">
import { usePageTitle } from '@/functions/usePage'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'

const { t } = useI18n()
const router = useRouter()

usePageTitle(() => t('title'))
</script>

<i18n lang="json">
{
  "en": {
    "title": "Page Not Found",
    "description": "You may have come to the wrong place or the page has been removed.",
    "back": "Back Home"
  },
  "zh": {
    "title": "網頁不存在",
    "description": "你可能來錯地方了，或者這個頁面已經不存在了。",
    "back": "返回首頁"
  }
}
</i18n>


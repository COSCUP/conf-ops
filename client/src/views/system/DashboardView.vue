<template>
  <NAlert
    v-for="role in roles"
    type="info"
    :title="t('welcome', { user: user?.name, role: role[`name_${locale}`] })"
  >
    {{ role[`welcome_message_${locale}`] }}
  </NAlert>
  <NFlex class="mt-2">
    <NCard class="md:w-auto">
      <NStatistic :label="t('todo')">
        <NText
          type="error"
        >
          {{ requiredTodo }}
        </NText>
        <template #suffix>
          <NText class="text-sm">{{ t('todo_suffix') }}</NText>
        </template>
      </NStatistic>
    </NCard>
    <NCard class="md:w-auto">
      <NStatistic :label="t('progress')">
        <NText
          type="warning"
        >
          {{ optionalTodo }}
        </NText>
        <template #suffix>
          <NText class="text-sm">{{ t('progress_suffix') }}</NText>
        </template>
      </NStatistic>
    </NCard>
  </NFlex>
</template>

<script setup lang="ts">
import { usePageLoading, usePageTitle } from '@/functions/usePage'
import { useProject } from '@/functions/useProject'
import { useLocale } from '@/i18n'
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()
const { locale } = useLocale()

const { user, features, roles, loading } = useProject()

const requiredTodo = computed(() => features.value.reduce((count, feature) => count + feature.todo[0], 0))

const optionalTodo = computed(() => features.value.reduce((count, feature) => count + feature.todo[1], 0))

usePageTitle(() => t('title'))
usePageLoading(() => loading.value)
</script>

<i18n lang="json">
{
  "en": {
    "title": "Dashboard",
    "welcome": "Hi {user} {role}",
    "todo": "Todo",
    "todo_suffix": "Todos",
    "progress": "In Progress",
    "progress_suffix": "In Progress"
  },
  "zh": {
    "title": "控制盤",
    "welcome": "你好 {user} {role}",
    "todo": "待處理",
    "todo_suffix": "件待辦事項",
    "progress": "進行中",
    "progress_suffix": "件進行中事項"
  }
}
</i18n>


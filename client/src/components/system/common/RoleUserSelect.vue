<template>
  <NSelect
    v-model:value="modelValue"
    :loading="loading"
    :options="options"
    @focus="focus"
  />
</template>

<script setup lang="ts">
import { APIResponse } from '@/api/base'
import { User } from '@/api/modules/project'
import { useAPI } from '@/functions/useAPI'
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'

const props = defineProps<{
  userApi: () => APIResponse<User[]>
}>()
const modelValue = defineModel({ type: String })

const { t } = useI18n()

const { data, loading, execute } = useAPI(props.userApi, { default: [], immediate: false })

const options = computed(() => [
  { label: t('no-assign'), value: undefined },
  ...data.value.map((item) => ({
    label: item.name,
    value: item.id
  }))
])

const focus = () => {
  if (data.value.length === 0) {
    execute()
  }
}
</script>

<i18n lang="json">
{
  "en": {
    "no-assign": "No assign"
  },
  "zh": {
    "no-assign": "未指派"
  }
}
</i18n>

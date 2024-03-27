<template>
  <NLayoutHeader class="px-4 py-2">
    <NFlex
      align="center"
      justify="space-between"
    >
      <RouterLink
        to="/"
        class="no-underline"
      >
        <n-text
          type="primary"
          class="text-lg font-bold"
        >
          {{ t('title') }}
        </n-text>
      </RouterLink>
      <NSwitch v-model:value="locale">
        <template #checked>
          中
        </template>
        <template #unchecked>
          EN
        </template>
      </NSwitch>
    </NFlex>
  </NLayoutHeader>
  <NLayoutContent class="min-h-[calc(100vh-76px)] pa-2">
    <slot />
  </NLayoutContent>
</template>

<script setup lang="ts">
import { useLocale } from '@/i18n'
import { computed } from 'vue';
import { useI18n } from 'vue-i18n'
import { RouterLink } from 'vue-router'

const { locale: _locale } = useLocale()
const { t } = useI18n()

const locale = computed({
  get: () => _locale.value === 'zh',
  set: (value: boolean) => {
    _locale.value = value ? 'zh' : 'en'
  }
})
</script>

<i18n lang="json">
{
  "en": {
    "title": "ConfOps"
  },
  "zh": {
    "title": "ConfOps"
  }
}
</i18n>

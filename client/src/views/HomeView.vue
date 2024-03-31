<template>
  <HomeLayout>
    <NEmpty
      v-if="loading === false && projects.length === 0"
      class="pt-8"
    />
    <NGrid
      v-else
      cols="1 m:3"
      responsive="screen"
    >
      <n-gi
        v-for="project in projects"
        :key="project.id"
        class="ma-2"
      >
        <RouterLink
          :to="`/login/${project.id}`"
          class="no-underline"
        >
          <NCard
            :title="project[`name_${locale}`]"
            hoverable
            class="hover:scale-x-102 hover:scale-y-108 relative pb-6"
          >
            {{ project[`description_${locale}`] }}
            <NButton type="primary" secondary circle class="absolute bottom-4 right-4">
              <template #icon>
                <NIcon>
                  <ArrowRight />
                </NIcon>
              </template>
            </NButton>
          </NCard>
        </RouterLink>
      </n-gi>
    </NGrid>
  </HomeLayout>
</template>

<script setup lang="ts">
import { api } from '@/api';
import HomeLayout from '@/components/HomeLayout.vue'
import { useAPI } from '@/functions/useAPI'
import { usePageLoading, usePageTitle } from '@/functions/usePage'
import { useLocale } from '@/i18n'
import { ArrowRight } from '@vicons/carbon'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()
const { locale } = useLocale()

const { data: projects, loading } = useAPI(api.project.getList, { default: [] })

usePageTitle(() => t('title'))
usePageLoading(() => loading.value)
</script>

<i18n lang="json">
{
  "en": {
    "title": "Home"
  },
  "zh": {
    "title": "首頁"
  }
}
</i18n>

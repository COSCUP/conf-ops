<template>
  <TicketBreadcrumb :value="breadcrumbs" />
  <NEmpty
    v-if="loading === false && schemas.length === 0"
    class="pt-8"
  />
  <NList
    v-else
    hoverable
    clickable
  >
    <NListItem
      v-for="schema in schemas"
      :key="schema.id"
    >
      <RouterLink
        :to="`/system/ticket/add/${schema.id}`"
        class="no-underline"
      >
        <NThing>
          <template #header>
            {{ schema[`title_${locale}`] }}
          </template>
          <template #header-extra>
            {{ t('last_updated', [schema.updated_at.toLocaleString()]) }}
          </template>
          <template #description>
            <NText class="whitespace-pre-line">
              {{ schema[`description_${locale}`] }}
            </NText>
          </template>
          <template #action>
            <NFlex
              justify="flex-end"
            >
              <NIcon size="24">
                <Add />
              </NIcon>
            </NFlex>
          </template>
        </NThing>
      </RouterLink>
    </NListItem>
  </NList>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import TicketBreadcrumb from '@/components/system/ticket/TicketBreadcrumb.vue'
import { TicketBreadcrumbType, useTicketBreadcrumb } from '@/functions/useTicket'
import { useAPI } from '@/functions/useAPI'
import { api } from '@/api'
import { usePageLoading } from '@/functions/usePage'
import { RouterLink } from 'vue-router'
import { Add } from '@vicons/carbon'
import { useLocale } from '@/i18n'

const { t } = useI18n()
const { locale } = useLocale()

const breadcrumbs = useTicketBreadcrumb([TicketBreadcrumbType.HOME, TicketBreadcrumbType.ADD1])
const { data: schemas, loading } = useAPI(api.ticket.getProbablySchemas, { default: [] })

usePageLoading(() => loading.value)
</script>

<i18n lang="json">
{
  "en": {
    "last_updated": "Last Updated: {0}"
  },
  "zh": {
    "last_updated": "最後更新時間：{0}"
  }
}
</i18n>
@/functions/useTicket

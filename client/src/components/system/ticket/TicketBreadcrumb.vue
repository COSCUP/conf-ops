<template>
  <NFlex
    class="h-[34px] mb-2"
    justify="space-between"
    align="center"
  >
    <NBreadcrumb>
      <NBreadcrumbItem
        v-for="level in value"
        :key="level.key"
      >
        <RouterLink
          :to="level.url"
          class="no-underline"
        >
          <NIcon :component="level.icon" /> {{ level.key }}
        </RouterLink>
      </NBreadcrumbItem>
    </NBreadcrumb>
    <slot />
  </NFlex>
</template>

<script setup lang="ts">
import { TicketBreadcrumbValue } from '@/functions/useTicket'

defineProps<{
  value: TicketBreadcrumbValue[]
}>()
</script>@/functions/useTicket

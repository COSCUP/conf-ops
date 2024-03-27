<template>
  <NCard content-class="!<md:pa-2">
    <NEmpty
      v-if="loading === false && roles.length === 0"
      class="pt-8"
    />
    <NCollapse
      v-else
      accordion
    >
      <NCollapseItem
        v-for="role in roles"
        :key="role.id"
        :title="role.name"
        :name="role.id"
      >
        <NCollapse>
          <NCollapseItem
            class="!<md:ml-2"
            :title="t('basic')"
          >
            <RoleForm
              :role="role"
              @saved="reload"
            />
          </NCollapseItem>
          <NCollapseItem
            class="!<md:ml-2"
            :title="t('member')"
          >
            <RoleMember
              :roleId="role.id"
            />
          </NCollapseItem>
        </NCollapse>
      </NCollapseItem>
    </NCollapse>
  </NCard>
</template>

<script setup lang="ts">
import { api } from '@/api'
import RoleForm from '@/components/system/role/RoleForm.vue'
import RoleMember from '@/components/system/role/RoleMember.vue'
import { useAPI } from '@/functions/useAPI'
import { usePageLoading, usePageTitle } from '@/functions/usePage'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()

usePageTitle(() => t('title'))

const { data: roles, loading, execute: reload } = useAPI(api.role.getManageRoles, { default: [] })

usePageLoading(() => loading.value)
</script>

<i18n lang="json">
{
  "en": {
    "title": "Role Management",
    "basic": "Basic Information",
    "member": "Member Management"
  },
  "zh": {
    "title": "角色管理",
    "basic": "基本信息",
    "member": "成員管理"
  }
}
</i18n>

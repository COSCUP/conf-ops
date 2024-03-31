<template>
  <NLayoutHeader class="px-4 py-2">
    <NFlex
      align="center"
      justify="space-between"
    >
      <NFlex
        align="center"
        justify="flex-start"
      >
        <NButton
          ghost
          size="small"
          @click="collapse = !collapse"
        >
          <NIcon :size="18">
            <Menu />
          </NIcon>
        </NButton>
        <n-text
          type="primary"
          class="text-lg font-bold cursor-default"
        >
          {{ project?.[`name_${locale}`] }} ConfOps
        </n-text>
      </NFlex>
      <NDropdown
        trigger="click"
        :options="userDropdownOptions"
        size="small"
        @select="handleUserDropdownClick"
      >
        <NButton quaternary circle>
          <NIcon :size="18">
            <UserAvatar />
          </NIcon>
        </NButton>
      </NDropdown>
    </NFlex>
  </NLayoutHeader>
  <NLayout has-sider>
    <NLayoutSider
      v-model:collapsed="collapse"
      bordered
      collapse-mode="transform"
      :show-trigger="!isMobile"
      :collapsed-width="0"
      :width="isMobile ? '100vw' : '240px'"
    >
      <NMenu
        :options="menus"
        @update:value="handleMenuClick"
      />
    </NLayoutSider>
    <NLayoutContent class="h-[calc(100vh-76px)] h-[calc(100dvh-76px)] pa-2 max-h-[calc(100vh-76px)] max-h-[calc(100dvh-76px)]">
      <RouterView />
    </NLayoutContent>
  </NLayout>
</template>

<script setup lang="ts">
import { usePageLoading } from '@/functions/usePage'
import { provideProject } from '@/functions/useProject'
import { renderIcon } from '@/utils/naive/helper'
import { computed, h, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { Dashboard, UserAvatar, Menu } from '@vicons/carbon'
import { FeatureDetail, getFeatureDetail } from '@/utils/features'
import { RouterLink, RouterView, useRouter } from 'vue-router'
import { NBadge, NFlex, useDialog } from 'naive-ui'
import { Feature } from '@/api/modules/project'
import { api } from '@/api'
import { useBreakpoint } from '@/functions/useBreakpoint'
import { useLocale } from '@/i18n'

const { t } = useI18n()
const { locale } = useLocale()
const dialog = useDialog()
const router = useRouter()
const { project, features, loading } = provideProject()
const { isMobile } = useBreakpoint()

usePageLoading(() => loading.value)

const generateMenuLabel = (feature: Feature, detail: FeatureDetail) => () => h(
  RouterLink,
  {
    to: detail.url
  },
  {
    default: () => h(
      NFlex,
      {
        justify: 'space-between',
        align: 'center'
      },
      {
        default: () => {
          const [required, optional] = feature.todo
          if (required || optional) {
            return [
              t(`feature.${detail.key}`),
              h(
                NBadge,
                {
                  type: required ? 'error' : 'warning',
                  value: required || optional
                }
              )
            ]
          }
          return [t(`feature.${detail.key}`)]
        }
      }
    )
  }
)

const collapse = ref(false)
watch(isMobile, () => {
  collapse.value = isMobile.value
})

const menus = computed(() => {
  const roleFeatures = features.value.filter(feature => feature.type === 'RoleManage')
  const roleMenus = roleFeatures.length > 0 ? [
    {
      type: 'group',
      label: t('member'),
      key: 'role',
      children: roleFeatures.map(feature => {
        const detail = getFeatureDetail(feature.type)
        return {
          label: generateMenuLabel(feature, detail),
          key: detail.key,
          icon: renderIcon(detail.icon)
        }
      })
    }
  ] : []

  const ticketFeatures = features.value.filter(feature => ['Ticket', 'TicketManage'].includes(feature.type))
  const ticketMenus = ticketFeatures.length > 0 ? [
    {
      type: 'group',
      label: t('ticket'),
      key: 'ticket',
      children: ticketFeatures.map(feature => {
        const detail = getFeatureDetail(feature.type)
        return {
          label: generateMenuLabel(feature, detail),
          key: detail.key,
          icon: renderIcon(detail.icon)
        }
      })
    }
  ] : []

  return [
    {
      label: () => h(
        RouterLink,
        {
          to: '/system/'
        },
        {
          default: () => t('dashboard')
        }
      ),
      key: 'dashboard',
      icon: renderIcon(Dashboard)
    },
    ...roleMenus,
    ...ticketMenus
  ]
})

const handleMenuClick = () => {
  if (isMobile.value) {
    collapse.value = true
  }
}

const userDropdownOptions = computed(() => [
  {
    label: t('logout.title'),
    key: 'logout',
  }
])

const handleUserDropdownClick = (key: string) => {
  if (key === 'logout') {
    dialog.info(
      {
        title: t('logout.title'),
        content: t('logout.confirm'),
        positiveText: t('ok'),
        negativeText: t('cancel'),
        onPositiveClick: () => {
          dialog.destroyAll()
          api.project.logout()
          router.push('/')
        }
      }
    )
  }
}
</script>

<i18n lang="json">
{
  "en": {
    "dashboard": "Dashboard",
    "member": "Member",
    "ticket": "Ticket",
    "feature": {
      "role-manage": "Manage Role",
      "ticket-manage": "Manage Ticket",
      "ticket": "Ticket"
    },
    "logout": {
      "title": "Logout",
      "confirm": "Are you sure you want to logout?"
    }
  },
  "zh": {
    "dashboard": "儀表板",
    "member": "成員",
    "ticket": "工單",
    "feature": {
      "role-manage": "角色管理",
      "ticket-manage": "工單管理",
      "ticket": "工單處理"
    },
    "logout": {
      "title": "登出",
      "confirm": "確定要登出嗎？"
    }
  }
}
</i18n>
